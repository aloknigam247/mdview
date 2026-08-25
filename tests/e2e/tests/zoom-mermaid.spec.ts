import { test, expect } from "@playwright/test";

test("zoom scales rendered mermaid svg", async ({ page }) => {
  await page.setViewportSize({ width: 640, height: 720 });

  const response = await page.goto("/fixtures/mermaid.md");
  expect(response?.ok(), "fixture should be served").toBeTruthy();
  await page.waitForLoadState("networkidle");

  const diagram = page.locator(".mermaid").first();
  const svg = diagram.locator("svg").first();
  const following = diagram.locator("xpath=following-sibling::*[1]");
  await expect(svg).toBeVisible({ timeout: 15_000 });

  const before = await svg.boundingBox();
  const beforeLayout = await diagram.evaluate((el) => ({
    height: el.scrollHeight,
    width: el.scrollWidth,
  }));
  expect(before).not.toBeNull();

  await page.evaluate(() => {
    window.dispatchEvent(
      new WheelEvent("wheel", {
        bubbles: true,
        cancelable: true,
        ctrlKey: true,
        deltaY: -100,
      }),
    );
  });

  await expect
    .poll(async () => {
      return await page.evaluate(() =>
        getComputedStyle(document.documentElement).getPropertyValue("--mdv-zoom").trim(),
      );
    })
    .toBe("1.1");

  const zoomedIn = await svg.boundingBox();
  const zoomedInLayout = await diagram.evaluate((el) => ({
    height: el.scrollHeight,
    width: el.scrollWidth,
  }));
  expect(zoomedIn).not.toBeNull();
  expect(zoomedIn!.width).toBeGreaterThan(before!.width * 1.05);
  expect(zoomedIn!.height).toBeGreaterThan(before!.height * 1.05);
  expect(zoomedInLayout.width).toBeGreaterThan(beforeLayout.width * 1.05);
  expect(zoomedInLayout.height).toBeGreaterThan(beforeLayout.height * 1.05);

  if (await following.count()) {
    const diagramBox = await diagram.boundingBox();
    const followingBox = await following.boundingBox();
    expect(diagramBox).not.toBeNull();
    expect(followingBox).not.toBeNull();
    expect(followingBox!.y).toBeGreaterThanOrEqual(diagramBox!.y + diagramBox!.height);
  }

  await page.evaluate(() => {
    for (let i = 0; i < 2; i += 1) {
      window.dispatchEvent(
        new WheelEvent("wheel", {
          bubbles: true,
          cancelable: true,
          ctrlKey: true,
          deltaY: 100,
        }),
      );
    }
  });

  await expect
    .poll(async () => {
      return await page.evaluate(() =>
        getComputedStyle(document.documentElement).getPropertyValue("--mdv-zoom").trim(),
      );
    })
    .toBe("0.9");

  const zoomedOut = await svg.boundingBox();
  expect(zoomedOut).not.toBeNull();
  expect(zoomedOut!.width).toBeLessThan(before!.width * 0.95);
  expect(zoomedOut!.height).toBeLessThan(before!.height * 0.95);
});
