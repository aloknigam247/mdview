import { test, expect, type Page } from "@playwright/test";

// Regression tests for #18: one physical keypress must select exactly one keymap
// action. Two defects allowed Ctrl+Shift+B to fire both `toggle-bionic` and
// `toggle-toc`: `matchBinding` treated Shift as optional for every alphabetic
// binding, and the keydown handler tested each action in an independent `if`
// without returning after the first match.
//
// The serve-only process loads config.toml once at startup and applies it to
// every path, so a fixture cannot carry its own `[keymap]`. Instead these tests
// install bindings at runtime: render.rs assigns `keymap: bindings` into
// `window.__mdv_config`, so that property IS the object the dispatcher closes
// over and re-reads on every keydown. Mutating it in place therefore configures
// the real shipped dispatcher.

type Chord = {
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
  super: boolean;
  kind: "char" | "named";
  key: string;
};

// Mirrors what `binding_to_json` emits: character keys always lowercased.
const chord = (over: Partial<Chord> & { key: string }): Chord => ({
  ctrl: false,
  shift: false,
  alt: false,
  super: false,
  kind: "char",
  ...over,
});

const CTRL_SHIFT_B = chord({ key: "b", ctrl: true, shift: true });
const CTRL_B = chord({ key: "b", ctrl: true });
const M = chord({ key: "m" });

/**
 * Replace every binding with `bindings`, and wrap each toggle so it appends its
 * name to an ordered `window.__mdvActions` log. The log is what proves *how
 * many* actions a single keypress ran, which a DOM-only assertion cannot show.
 */
async function installKeymap(page: Page, bindings: Record<string, Chord>): Promise<void> {
  await page.evaluate((binds) => {
    const w = window as unknown as {
      __mdv_config?: { keymap?: Record<string, unknown> };
      __mdvActions?: string[];
      __mdvToggleBionic?: () => void;
      __mdvToggleCodemap?: () => void;
      __mdvToggleTheme?: () => void;
      __mdvToggleToc?: () => void;
    };

    const km = w.__mdv_config?.keymap;
    if (!km) throw new Error("window.__mdv_config.keymap is missing");

    // Mutate in place — reassigning would break the dispatcher's reference and
    // silently disable every binding.
    for (const key of Object.keys(km)) delete km[key];
    for (const [action, binding] of Object.entries(binds)) km[action] = binding;

    const log: string[] = [];
    w.__mdvActions = log;
    const wrap = (name: string, original?: () => void) => () => {
      log.push(name);
      original?.();
    };
    w.__mdvToggleBionic = wrap("toggle-bionic", w.__mdvToggleBionic);
    w.__mdvToggleCodemap = wrap("toggle-codemap", w.__mdvToggleCodemap);
    w.__mdvToggleTheme = wrap("toggle-theme", w.__mdvToggleTheme);
    w.__mdvToggleToc = wrap("toggle-toc", w.__mdvToggleToc);
  }, bindings);
}

const actionLog = (page: Page) =>
  page.evaluate(() => (window as unknown as { __mdvActions: string[] }).__mdvActions);

async function openFixture(page: Page): Promise<void> {
  const response = await page.goto("/fixtures/gfm.md");
  expect(response?.ok(), "fixture should be served").toBeTruthy();
  await page.waitForLoadState("networkidle");

  // Without these the assertions below could pass vacuously.
  await expect(page.locator("#mdv-toc")).toHaveAttribute("aria-hidden", "true");
  expect(
    await page.locator("article p").count(),
    "fixture must have paragraphs for bionic to transform",
  ).toBeGreaterThan(0);
  await expect(page.locator(".mdv-bionic")).toHaveCount(0);
}

test("Ctrl+Shift+B fires bionic without toggling toc", async ({ page }) => {
  await openFixture(page);
  await installKeymap(page, { "toggle-bionic": CTRL_SHIFT_B, "toggle-toc": CTRL_B });

  await page.keyboard.press("Control+Shift+B");

  // Before the fix this was ["toggle-bionic", "toggle-toc"]: Ctrl+B matched a
  // Shift-bearing event, and the handler ran every match.
  await expect.poll(() => actionLog(page)).toEqual(["toggle-bionic"]);
  await expect(page.locator(".mdv-bionic").first()).toBeVisible();
  await expect(page.locator("#mdv-toc")).toHaveAttribute("aria-hidden", "true");
});

test("Ctrl+B still toggles toc", async ({ page }) => {
  await openFixture(page);
  await installKeymap(page, { "toggle-bionic": CTRL_SHIFT_B, "toggle-toc": CTRL_B });

  await page.keyboard.press("Control+B");

  // Guards against over-correcting the fix into "only Shift-bearing chords match".
  await expect.poll(() => actionLog(page)).toEqual(["toggle-toc"]);
  await expect(page.locator("#mdv-toc")).toHaveAttribute("aria-hidden", "false");
  await expect(page.locator(".mdv-bionic")).toHaveCount(0);
});

test("duplicate exact chord fires first action only", async ({ page }) => {
  await openFixture(page);
  // Both actions on the identical chord: strict modifier matching cannot
  // separate these, so only the dispatcher's early return can.
  await installKeymap(page, { "toggle-bionic": CTRL_SHIFT_B, "toggle-toc": CTRL_SHIFT_B });

  await page.keyboard.press("Control+Shift+B");

  // First-defined wins: quit, toggle-theme, toggle-bionic, toggle-codemap, toggle-toc.
  await expect.poll(() => actionLog(page)).toEqual(["toggle-bionic"]);
  await expect(page.locator("#mdv-toc")).toHaveAttribute("aria-hidden", "true");
});

// Regression tests for #52: the codemap must be reachable only through an opt-in
// [keymap] binding. A hardcoded bare-`m` keydown listener used to toggle it
// independently of the dispatcher, so `m` ran two actions per press.

const codemapVisible = (page: Page) =>
  page.evaluate(
    () => (window as unknown as { __mdvCodemapVisible: () => boolean }).__mdvCodemapVisible(),
  );

test("m toggles codemap exactly once", async ({ page }) => {
  await openFixture(page);
  await installKeymap(page, { "toggle-codemap": M });

  await page.keyboard.press("m");

  // Before the fix this was ["toggle-codemap", "toggle-codemap"]: the hardcoded
  // bare-`m` listener fired alongside the dispatcher.
  await expect.poll(() => actionLog(page)).toEqual(["toggle-codemap"]);
});

// Regression test for #56: the minimap was lazy-mounted visible then immediately
// flipped hidden in the same call, so the first press netted to hidden. It must
// now reveal on the first press.
test("first toggle-codemap press reveals the codemap", async ({ page }) => {
  await openFixture(page);
  await installKeymap(page, { "toggle-codemap": M });

  expect(await codemapVisible(page)).toBe(false);

  await page.keyboard.press("m");
  await expect.poll(() => actionLog(page)).toEqual(["toggle-codemap"]);
  await expect.poll(() => codemapVisible(page)).toBe(true);

  await page.keyboard.press("m");
  await expect.poll(() => codemapVisible(page)).toBe(false);
});

test("m bound to toc does not also toggle codemap", async ({ page }) => {
  await openFixture(page);
  await installKeymap(page, { "toggle-toc": M });

  await page.keyboard.press("m");

  // Before the fix this was ["toggle-toc", "toggle-codemap"].
  await expect.poll(() => actionLog(page)).toEqual(["toggle-toc"]);
  await expect(page.locator("#mdv-toc")).toHaveAttribute("aria-hidden", "false");
  expect(await codemapVisible(page)).toBe(false);
});

// Regression tests for #53: a binding on a Shift-requiring punctuation key (e.g.
// '?') must fire when that glyph is typed, even though the DOM reports
// shiftKey: true for it. Producing '?' on many layouts inherently needs Shift, so
// a binding without an explicit Shift must ignore the flag.

const QUESTION = chord({ key: "?" });

test("'?' binding toggles toc when '?' is typed", async ({ page }) => {
  await openFixture(page);
  await installKeymap(page, { "toggle-toc": QUESTION });

  // page.keyboard.press("?") emits e.key === "?" with e.shiftKey === true on US layouts.
  await page.keyboard.press("?");

  await expect.poll(() => actionLog(page)).toEqual(["toggle-toc"]);
  await expect(page.locator("#mdv-toc")).toHaveAttribute("aria-hidden", "false");
});

test("'?' binding matches shifted and unshifted events and rejects a different key", async ({
  page,
}) => {
  await openFixture(page);
  await installKeymap(page, { "toggle-toc": QUESTION });

  // A different punctuation key must not fire the '?' binding.
  await page.evaluate(() => {
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "/", bubbles: true }));
  });
  await expect.poll(() => actionLog(page)).toEqual([]);

  // Unshifted '?' matches (layouts where '?' does not require Shift).
  await page.evaluate(() => {
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "?", shiftKey: false, bubbles: true }));
  });
  await expect.poll(() => actionLog(page)).toEqual(["toggle-toc"]);

  // Shift-bearing '?' also matches.
  await page.evaluate(() => {
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "?", shiftKey: true, bubbles: true }));
  });
  await expect.poll(() => actionLog(page)).toEqual(["toggle-toc", "toggle-toc"]);
});

test("explicit Shift+'?' binding requires shiftKey", async ({ page }) => {
  await openFixture(page);
  await installKeymap(page, { "toggle-toc": chord({ key: "?", shift: true }) });

  // Unshifted '?' must NOT fire an explicit Shift+? binding.
  await page.evaluate(() => {
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "?", shiftKey: false, bubbles: true }));
  });
  await expect.poll(() => actionLog(page)).toEqual([]);

  // Shift-bearing '?' fires it.
  await page.evaluate(() => {
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "?", shiftKey: true, bubbles: true }));
  });
  await expect.poll(() => actionLog(page)).toEqual(["toggle-toc"]);
});
