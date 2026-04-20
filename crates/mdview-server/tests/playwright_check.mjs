import { chromium } from "playwright";

const url = process.env.MDV_URL;
if (!url) {
  console.error("MDV_URL not set");
  process.exit(2);
}

const browser = await chromium.launch({ channel: "chromium", headless: true });
const context = await browser.newContext();
const page = await context.newPage();

await page.goto(url, { waitUntil: "load", timeout: 15000 });

await page.waitForFunction(() => window.__mdv_ws_connected === true, null, {
  timeout: 10000,
});

const title = await page.title();
const connected = await page.evaluate(() => window.__mdv_ws_connected);
const bodyState = await page.evaluate(() => document.body.getAttribute("data-mdv-live"));
const hasH1 = (await page.locator("h1").count()) > 0;

console.log(JSON.stringify({ title, connected, bodyState, hasH1 }));

const outPath = process.env.MDV_SCREENSHOT;
if (outPath) {
  await page.screenshot({ path: outPath, fullPage: true });
}

await browser.close();

if (!connected || !hasH1) {
  console.error("assertion failed");
  process.exit(1);
}
