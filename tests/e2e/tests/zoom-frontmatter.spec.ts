import { test, expect } from "@playwright/test";

test("ctrl+wheel zoom scales the frontmatter hero card like body text", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });

  const response = await page.goto("/fixtures/everything.md");
  expect(response?.ok(), "fixture should be served").toBeTruthy();
  await page.waitForLoadState("networkidle");

  const title = page.locator(".mdview-frontmatter__title").first();
  const bodyProbe = page.locator("article.mdv p").first();
  await expect(title).toBeVisible({ timeout: 15_000 });
  await expect(bodyProbe).toBeVisible({ timeout: 15_000 });

  const fontSize = (loc: ReturnType<typeof page.locator>) =>
    loc.evaluate((el) => Number.parseFloat(window.getComputedStyle(el).fontSize));

  const titleBefore = await fontSize(title);
  const bodyBefore = await fontSize(bodyProbe);
  const boxBefore = await title.boundingBox();
  expect(boxBefore).not.toBeNull();

  // Five ctrl+wheel-up steps: --mdv-zoom 1.0 -> 1.5 (0.1 per step).
  for (let i = 0; i < 5; i += 1) {
    await page.evaluate(() =>
      window.dispatchEvent(
        new WheelEvent("wheel", { bubbles: true, cancelable: true, ctrlKey: true, deltaY: -100 }),
      ),
    );
    await page.waitForTimeout(120);
  }

  await expect
    .poll(async () =>
      page.evaluate(() =>
        getComputedStyle(document.documentElement).getPropertyValue("--mdv-zoom").trim(),
      ),
    )
    .toBe("1.5");

  const titleAfter = await fontSize(title);
  const bodyAfter = await fontSize(bodyProbe);
  const boxAfter = await title.boundingBox();
  expect(boxAfter).not.toBeNull();

  const bodyRatio = bodyAfter / bodyBefore;
  const titleRatio = titleAfter / titleBefore;

  expect(bodyRatio).toBeGreaterThan(1.4);
  // The card title must grow by ~the same ratio as body text, not stay fixed.
  expect(titleRatio).toBeGreaterThanOrEqual(bodyRatio * 0.95);
  expect(titleRatio).toBeLessThanOrEqual(bodyRatio * 1.05);
  expect(boxAfter!.height).toBeGreaterThan(boxBefore!.height * 1.3);

  // Cycle back out; the card must shrink consistently with body text.
  for (let i = 0; i < 5; i += 1) {
    await page.evaluate(() =>
      window.dispatchEvent(
        new WheelEvent("wheel", { bubbles: true, cancelable: true, ctrlKey: true, deltaY: 100 }),
      ),
    );
    await page.waitForTimeout(120);
  }
  await expect
    .poll(async () =>
      page.evaluate(() =>
        getComputedStyle(document.documentElement).getPropertyValue("--mdv-zoom").trim(),
      ),
    )
    .toBe("1.0");
  await expect.poll(async () => fontSize(title)).toBeLessThan(titleBefore * 1.05);
});
