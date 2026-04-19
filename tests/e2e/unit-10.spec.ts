import { test, expect } from "@playwright/test";
import { statSync } from "node:fs";
import { resolve } from "node:path";

test("unit-10 plotly chart renders", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  const container = page.locator(".plotly-chart .svg-container");
  await expect(container).toHaveCount(1, { timeout: 30_000 });
  await expect(page.locator(".plotly-chart .main-svg").first()).toHaveCount(1, {
    timeout: 30_000,
  });
  const screenshotPath = resolve(__dirname, "..", "..", "artifacts", "unit-10.png");
  await page.screenshot({ path: screenshotPath, fullPage: true });
  const size = statSync(screenshotPath).size;
  expect(size).toBeGreaterThan(10 * 1024);
});
