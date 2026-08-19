import { test, expect, type Page } from "@playwright/test";

type Chord = {
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
  super: boolean;
  kind: "char" | "named";
  key: string;
};

const T: Chord = {
  ctrl: false,
  shift: false,
  alt: false,
  super: false,
  kind: "char",
  key: "t",
};

async function installThemeBinding(page: Page): Promise<void> {
  await page.evaluate((binding) => {
    const w = window as unknown as { __mdv_config?: { keymap?: Record<string, unknown> } };
    const km = w.__mdv_config?.keymap;
    if (!km) throw new Error("window.__mdv_config.keymap is missing");

    for (const key of Object.keys(km)) delete km[key];
    km["toggle-theme"] = binding;
  }, T);
}

async function readDiagramColors(page: Page) {
  return page.evaluate(() => ({
    drawio: document.querySelector(".mxgraph svg [fill]")?.getAttribute("fill") ?? null,
    mermaid:
      document.querySelector(".mermaid svg style")?.textContent?.match(/fill:([^;}]+)/)?.[1] ??
      null,
    plotly: document.querySelector(".plotly-chart .main-svg .ygrid")?.getAttribute("style") ?? null,
  }));
}

test("theme action emits colorscheme and recolors every diagram", async ({ page }) => {
  const response = await page.goto("/fixtures/everything.md");
  expect(response?.ok(), "fixture should be served").toBeTruthy();
  await page.waitForLoadState("networkidle");
  await expect(page.locator(".mermaid svg").first()).toBeVisible({ timeout: 15_000 });
  await expect(page.locator(".plotly-chart .main-svg").first()).toBeVisible({ timeout: 15_000 });
  await expect(page.locator(".mxgraph").first()).toBeVisible({ timeout: 15_000 });
  await installThemeBinding(page);

  const beforeClass = await page.locator("html").getAttribute("class");
  const before = await readDiagramColors(page);
  expect(before.mermaid, "Mermaid selector must capture a color").toBeTruthy();
  expect(before.plotly, "Plotly selector must capture a color").toBeTruthy();
  expect(before.drawio, "draw.io selector must capture a color").toBeTruthy();

  const eventSeen = page.evaluate(
    () =>
      new Promise((resolve, reject) => {
        const w = window as unknown as {
          __mdv_on?: (name: string, fn: (detail: unknown) => void) => void;
        };
        if (typeof w.__mdv_on !== "function") {
          reject(new Error("window.__mdv_on is missing"));
          return;
        }
        w.__mdv_on("colorscheme", (detail) => resolve(detail));
      }),
  );

  await page.keyboard.press("t");
  await expect.poll(() => page.locator("html").getAttribute("class")).not.toBe(beforeClass);
  await expect(eventSeen).resolves.toMatchObject({
    colors: expect.objectContaining({
      "--mdv-accent": expect.any(String),
      "--mdv-accent-blue": expect.any(String),
      "--mdv-accent-green": expect.any(String),
      "--mdv-accent-mauve": expect.any(String),
      "--mdv-accent-peach": expect.any(String),
      "--mdv-accent-yellow": expect.any(String),
      "--mdv-bg": expect.any(String),
      "--mdv-border-subtle": expect.any(String),
      "--mdv-code-bg": expect.any(String),
      "--mdv-fg": expect.any(String),
      "--mdv-link": expect.any(String),
      "--mdv-muted": expect.any(String),
    }),
  });
  await expect.poll(async () => (await readDiagramColors(page)).mermaid).not.toBe(before.mermaid);
  await expect.poll(async () => (await readDiagramColors(page)).plotly).not.toBe(before.plotly);
  await expect.poll(async () => (await readDiagramColors(page)).drawio).not.toBe(before.drawio);
});
