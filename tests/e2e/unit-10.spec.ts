import { test, expect } from "@playwright/test";
import { statSync } from "node:fs";

test("unit-10 plotly chart renders", async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  const charts = page.locator(".plotly-chart");
  await expect(charts).toHaveCount(4);
  for (const chart of await charts.all()) {
    const container = chart.locator(".svg-container");
    await expect(container).toHaveCount(1, { timeout: 30_000 });
    await expect(chart.locator(".main-svg").first()).toHaveCount(1, {
      timeout: 30_000,
    });
  }
  const screenshotPath = testInfo.outputPath("unit-10.png");
  await page.screenshot({ path: screenshotPath, fullPage: true });
  const size = statSync(screenshotPath).size;
  expect(size).toBeGreaterThan(10 * 1024);
});
