import { test, expect } from "@playwright/test";

test("ctrl+wheel zoom scales plotly and drawio crisply via geometry", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });

  const response = await page.goto("/fixtures/showcase.md");
  expect(response?.ok(), "fixture should be served").toBeTruthy();
  await page.waitForLoadState("networkidle");

  const plotly = page.locator(".plotly-chart").first();
  const plotlyMain = plotly.locator(".main-svg").first();
  const drawioSvg = page.locator(".mxgraph > svg").first();
  await expect(plotlyMain).toBeVisible({ timeout: 15_000 });
  await expect(drawioSvg).toBeVisible({ timeout: 15_000 });
  await page.waitForTimeout(500);

  const plotlyWidthAttr = () => plotlyMain.evaluate((el) => Number(el.getAttribute("width")));
  const beforePlotlyAttr = await plotlyWidthAttr();
  const beforeDrawio = await drawioSvg.boundingBox();
  expect(beforePlotlyAttr).toBeGreaterThan(0);
  expect(beforeDrawio).not.toBeNull();

  // roots must never be transform-scaled (blurry); geometry drives the size
  const transforms = () =>
    page.evaluate(() => ({
      plotly: getComputedStyle(document.querySelector(".plotly-chart")!).transform,
      drawio: getComputedStyle(document.querySelector(".mxgraph")!).transform,
    }));
  const beforeTransforms = await transforms();
  expect(beforeTransforms.plotly).toBe("none");
  expect(beforeTransforms.drawio).toBe("none");

  for (let i = 0; i < 3; i += 1) {
    await page.evaluate(() =>
      window.dispatchEvent(
        new WheelEvent("wheel", { bubbles: true, cancelable: true, ctrlKey: true, deltaY: -100 }),
      ),
    );
    await page.waitForTimeout(150);
  }

  await expect
    .poll(async () =>
      page.evaluate(() =>
        getComputedStyle(document.documentElement).getPropertyValue("--mdv-zoom").trim(),
      ),
    )
    .toBe("1.3");

  // plotly re-lays out: its inner main-svg width ATTRIBUTE grows (crisp), not a css transform
  await expect.poll(plotlyWidthAttr).toBeGreaterThan(beforePlotlyAttr * 1.2);

  const zoomedDrawio = await drawioSvg.boundingBox();
  expect(zoomedDrawio).not.toBeNull();
  expect(zoomedDrawio!.width).toBeGreaterThan(beforeDrawio!.width * 1.2);
  expect(zoomedDrawio!.height).toBeGreaterThan(beforeDrawio!.height * 1.2);

  const afterTransforms = await transforms();
  expect(afterTransforms.plotly).toBe("none");
  expect(afterTransforms.drawio).toBe("none");

  // cycle back out to base and confirm plotly re-lays out down again
  for (let i = 0; i < 3; i += 1) {
    await page.evaluate(() =>
      window.dispatchEvent(
        new WheelEvent("wheel", { bubbles: true, cancelable: true, ctrlKey: true, deltaY: 100 }),
      ),
    );
    await page.waitForTimeout(150);
  }
  await expect
    .poll(async () =>
      page.evaluate(() =>
        getComputedStyle(document.documentElement).getPropertyValue("--mdv-zoom").trim(),
      ),
    )
    .toBe("1.0");
  await expect.poll(plotlyWidthAttr).toBeLessThan(beforePlotlyAttr * 1.05);
});
