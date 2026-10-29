// Screenshot script for Unit 7. Run with:
//   node tests/playwright_screenshot.mjs
// Requires a running `cargo run --example demo_serve` on 127.0.0.1:7683.

import { chromium } from "playwright";
import { mkdirSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const outDir = process.env.MDVIEW_ARTIFACTS_DIR
  ? resolve(process.env.MDVIEW_ARTIFACTS_DIR)
  : resolve(__dirname, "..", "artifacts", "standalone", "math");
mkdirSync(outDir, { recursive: true });
const outPath = resolve(outDir, "unit-07.png");

const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 960, height: 720 } });
  await page.goto("http://127.0.0.1:7683/", { waitUntil: "networkidle" });
  await page.waitForSelector(".katex", { state: "visible", timeout: 10_000 });
  await page.screenshot({ path: outPath, fullPage: true });
} finally {
  await browser.close();
}

const size = statSync(outPath).size;
if (size <= 10_240) {
  console.error(`screenshot too small: ${size} bytes`);
  process.exit(1);
}
console.log(`screenshot OK (${size} bytes): ${outPath}`);
