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
