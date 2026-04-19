import { chromium } from "playwright";
import { mkdirSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const url = process.env.MDVIEW_URL ?? "http://127.0.0.1:7682/";
const outDir = process.env.MDVIEW_ARTIFACTS_DIR
  ? resolve(process.env.MDVIEW_ARTIFACTS_DIR)
  : resolve(here, "../../crates/mdview-ext-highlight/artifacts");
const outFile = resolve(outDir, "unit-06.png");

mkdirSync(outDir, { recursive: true });

const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await page.goto(url, { waitUntil: "networkidle" });
  await page.screenshot({ path: outFile, fullPage: true });
} finally {
  await browser.close();
}

const size = statSync(outFile).size;
console.log(`wrote ${outFile} (${size} bytes)`);
if (size <= 10 * 1024) {
  console.error(`assertion failed: ${outFile} is only ${size} bytes (<= 10 KB)`);
  process.exit(1);
}
