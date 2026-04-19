import { chromium } from "playwright";
import { mkdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const crateDir = join(__dirname, "..", "..");
const artifactsDir = join(crateDir, "artifacts");
mkdirSync(artifactsDir, { recursive: true });

const url = "http://127.0.0.1:7684/";
const screenshotPath = join(artifactsDir, "unit-08.png");

const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1024, height: 768 } });
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.waitForSelector(".mermaid svg", { timeout: 30000 });
  // give mermaid a moment to finish layout
  await page.waitForTimeout(500);
  await page.screenshot({ path: screenshotPath, fullPage: true });
} finally {
  await browser.close();
}

const size = statSync(screenshotPath).size;
if (size <= 10 * 1024) {
  console.error(`screenshot too small: ${size} bytes`);
  process.exit(1);
}
console.log(`screenshot ${screenshotPath} = ${size} bytes`);
