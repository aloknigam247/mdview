import { test, expect } from "@playwright/test";

// Regression test for #23: clicking a TOC entry must scroll only the block axis and
// return the page to the left edge. `scrollIntoView`'s inline default ('nearest')
// shifted `window.scrollX` when the document overflows horizontally, which visibly ate
// the page's left margin.
test("toc click resets horizontal scroll position", async ({ page }) => {
  const response = await page.goto("/fixtures/toc-horizontal-overflow.md");
  expect(response?.ok(), "fixture should be served").toBeTruthy();
  await page.waitForLoadState("networkidle");

  // Without real horizontal overflow the assertions below would pass even unfixed.
  const maxX = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  expect(maxX, "fixture must overflow horizontally for this test to mean anything").toBeGreaterThan(
    100,
  );

  // The TOC is emitted hidden by design and revealed only through the toggle.
  await page.evaluate(() => (window as unknown as { __mdvToggleToc: () => void }).__mdvToggleToc());
  const link = page.locator("#mdv-toc a", { hasText: "Target section" });
  await expect(link).toBeVisible();

  // Start scrolled to the right, so a handler that merely leaves the inline axis
  // alone is not enough — the click must actively bring the left margin back.
  await page.evaluate(() => window.scrollTo(100, 0));
  await expect.poll(() => page.evaluate(() => window.scrollX)).toBe(100);

  // The browser cannot scroll past the end of the document, so the reachable
  // destination is the target's offset clamped to the maximum scroll position.
  const expectedY = await page.evaluate(() => {
    const h = document.getElementById("target-section");
    if (!h) throw new Error("target heading not found");
    const targetY = h.getBoundingClientRect().top + window.scrollY;
    const maxY = document.documentElement.scrollHeight - document.documentElement.clientHeight;
    return Math.min(targetY, maxY);
  });
  expect(expectedY, "target heading must be below the fold").toBeGreaterThan(200);

  await link.click();

  // Smooth scrolling is asynchronous: wait for it to settle before judging the inline
  // axis, otherwise an early animation frame could pass even against the old handler.
  await expect
    .poll(() => page.evaluate(() => Math.round(window.scrollY)), { timeout: 10_000 })
    .toBeGreaterThan(expectedY - 5);

  await expect.poll(() => page.evaluate(() => window.scrollX), { timeout: 10_000 }).toBe(0);
  expect(await page.evaluate(() => location.hash)).toBe("#target-section");
});
