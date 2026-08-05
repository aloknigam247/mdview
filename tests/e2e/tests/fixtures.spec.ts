import { test, expect, type Page } from "@playwright/test";
import { statSync } from "node:fs";
import { join } from "node:path";

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
    selectors: ["pre code", "pre.mdv-code[data-lang='jsonc'] .mdv-tok-string"],
  },
  {
    name: "drawio",
    headings: ["Draw.io"],
    selectors: [".drawio-viewer > *"],
  },
  {
    name: "everything",
    headings: ["Everything"],
    selectors: [".mermaid svg", ".katex", ".plotly-chart .svg-container", ".drawio-viewer > *", "pre code"],
  },
  {
    name: "gfm",
    headings: ["GFM"],
    selectors: ["table", "ul.contains-task-list"],
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

async function screenshotAndAssertSize(page: Page, name: string) {
  const path = join("artifacts", `${name}.png`);
  await page.screenshot({ path, fullPage: true });
  const size = statSync(path).size;
  expect(size, `${name} screenshot should be > 10 KB`).toBeGreaterThan(10 * 1024);
}

for (const fixture of FIXTURES) {
  test(`fixture: ${fixture.name}`, async ({ page }) => {
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
        await expect(
          page.locator(selector).first(),
          `expected ${selector} on ${fixture.name}`,
        ).toBeVisible({ timeout: 15_000 });
      }
    }

    await screenshotAndAssertSize(page, fixture.name);
  });
}

test("server health probe", async ({ request }) => {
  const res = await request.get("/");
  expect(res.status()).toBeLessThan(500);
});
