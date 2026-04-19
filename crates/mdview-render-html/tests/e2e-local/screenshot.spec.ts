import { expect, test } from "@playwright/test";
import { statSync } from "node:fs";
import { resolve } from "node:path";

test("screenshot everything fixture", async ({ page }) => {
  await page.goto("/everything");
  await page.waitForLoadState("networkidle");
  const out = resolve(__dirname, "../../artifacts/unit-03.png");
  await page.screenshot({ path: out, fullPage: true });
  const size = statSync(out).size;
  expect(size).toBeGreaterThan(10 * 1024);
});

test("index lists fixtures", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator("h1")).toContainText("mdview-render-html");
});
