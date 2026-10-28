import { test, expect, type Page } from "@playwright/test";
import { statSync } from "node:fs";

test.describe.configure({ mode: "serial" });

type FixtureExpectation = {
  name: string;
  selectors?: string[];
  headings?: string[];
};

const FIXTURES: FixtureExpectation[] = [
  {
    name: "code",
    headings: ["Code"],
    selectors: [
      "pre code",
      "code.mdv-code-inline[data-lang='rust']",
      "pre.mdv-code[data-lang='jsonc'] .mdv-tok-string",
      "pre.mdv-code[data-lang='http'] .mdv-tok-keyword",
      "pre.mdv-code[data-lang='http'] .mdv-tok-constant",
    ],
  },
  {
    name: "drawio",
    headings: ["Draw.io"],
    selectors: [".drawio-viewer.mxgraph > svg"],
  },
  {
    name: "everything",
    headings: ["Everything"],
    selectors: [".drawio-viewer.mxgraph > svg", ".katex", ".mermaid svg", ".plotly-chart .svg-container", "pre code"],
  },
  {
    name: "gfm",
    headings: ["GFM"],
    selectors: ["table", ".mdv-task-checked", ".mdv-task-unchecked"],
  },
  {
    name: "math",
    headings: ["Math"],
    selectors: [".katex"],
  },
  {
    name: "mermaid",
    headings: ["Mermaid"],
    selectors: [".mermaid svg"],
  },
  {
    name: "plotly",
    headings: ["Plotly"],
    selectors: [".plotly-chart .svg-container"],
  },
];

async function screenshotAndAssertSize(page: Page, name: string, path: string) {
  await page.screenshot({ path, fullPage: true });
  const size = statSync(path).size;
  expect(size, `${name} screenshot should be > 10 KB`).toBeGreaterThan(10 * 1024);
}

for (const fixture of FIXTURES) {
  test(`fixture: ${fixture.name}`, async ({ page }, testInfo) => {
    const response = await page.goto(`/fixtures/${fixture.name}.md`);
    expect(response?.ok(), `GET /fixtures/${fixture.name}.md should be 2xx`).toBeTruthy();
    await page.waitForLoadState("networkidle");

    if (fixture.headings) {
      for (const heading of fixture.headings) {
        const headingLocator = page.locator("h1, h2, h3", { hasText: heading }).first();
        await expect(headingLocator).toBeVisible({ timeout: 10_000 });
      }
    }

    if (fixture.selectors) {
      for (const selector of fixture.selectors) {
        await expect(page.locator(selector).first(), `expected ${selector} on ${fixture.name}`).toBeVisible({
          timeout: 15_000,
        });
      }
    }

    if (fixture.name === "drawio") {
      const drawioState = await page
        .locator(".drawio-viewer.mxgraph")
        .first()
        .evaluate((element) => ({
          config: element.getAttribute("data-mxgraph"),
          viewerReady:
            typeof (
              window as unknown as {
                GraphViewer?: { processElements?: unknown };
              }
            ).GraphViewer?.processElements === "function",
        }));
      const config = JSON.parse(drawioState.config ?? "null") as { xml?: string } | null;

      expect(drawioState.viewerReady, "draw.io viewer API should be available").toBe(true);
      expect(config?.xml, "draw.io viewer config should contain the source XML").toContain("<mxfile");
    }

    await screenshotAndAssertSize(page, fixture.name, testInfo.outputPath(`${fixture.name}.png`));
  });
}

test("inline hashbang code is highlighted in the code fixture", async ({ page }) => {
  const response = await page.goto("/fixtures/code.md");
  expect(response?.ok(), "GET /fixtures/code.md should be 2xx").toBeTruthy();
  await page.waitForLoadState("networkidle");

  const inline = page.locator("code.mdv-code-inline[data-lang='rust']").first();
  await expect(inline).toBeVisible({ timeout: 10_000 });

  // A highlighted token must have a color distinct from a plain inline-code chip.
  const kwColor = await inline
    .locator(".mdv-tok-type, .mdv-tok-keyword")
    .first()
    .evaluate((el) => getComputedStyle(el).color);
  const plainColor = await page
    .locator(":not(pre) > code:not(.mdv-code-inline)")
    .first()
    .evaluate((el) => getComputedStyle(el).color);
  expect(kwColor).not.toBe(plainColor);

  // The marker must not leak into the rendered text.
  await expect(inline).not.toContainText("#!rust");
});

test("server health probe", async ({ request }) => {
  const res = await request.get("/");
  expect(res.status()).toBeLessThan(500);
});
