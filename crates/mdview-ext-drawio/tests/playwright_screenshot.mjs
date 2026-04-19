// Standalone screenshot harness: serves the rendered HTML and captures a PNG via Playwright.
// Invoke with: node tests/playwright_screenshot.mjs
import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import url from "node:url";
import { chromium } from "playwright";

const here = path.dirname(url.fileURLToPath(import.meta.url));
const crateRoot = path.resolve(here, "..");
const repoRoot = path.resolve(crateRoot, "../..");

const VIEWER = fs.readFileSync(path.join(crateRoot, "vendor/drawio-viewer.js"), "utf8");
const INIT = fs.readFileSync(path.join(crateRoot, "vendor/mdv-drawio-init.js"), "utf8");
const FIXTURE = fs.readFileSync(path.join(repoRoot, "fixtures/drawio.md"), "utf8");

function extractDrawio(md) {
  const lines = md.split(/\r?\n/);
  const out = [];
  let inBlock = false;
  let info = "";
  let buf = [];
  for (const ln of lines) {
    const m = /^```(.*)$/.exec(ln);
    if (m) {
      if (!inBlock) {
        info = m[1].trim();
        inBlock = true;
        buf = [];
      } else {
        if (info.split(/[:\s]/)[0].toLowerCase() === "drawio") {
          out.push({ info, body: buf.join("\n") + "\n" });
        }
        inBlock = false;
      }
    } else if (inBlock) {
      buf.push(ln);
    }
  }
  return out;
}

const blocks = extractDrawio(FIXTURE);
const divs = blocks
  .map(
    (b) =>
      `<div class="drawio-viewer" data-xml-b64="${Buffer.from(b.body).toString("base64")}"></div>`,
  )
  .join("\n");

const html = `<!doctype html><html><head><meta charset="utf-8"><title>drawio demo</title>
<style>body{font-family:ui-sans-serif,system-ui;padding:32px;background:#f8fafc;}
.drawio-viewer{border-radius:12px;box-shadow:0 4px 12px rgba(15,23,42,0.1);padding:16px;background:white;margin:16px 0;}
</style></head><body><h1>drawio fixture</h1>${divs}
<script>${VIEWER}</script><script>${INIT}</script></body></html>`;

const server = http.createServer((req, res) => {
  res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
  res.end(html);
});
await new Promise((resolve) => server.listen(7685, "127.0.0.1", resolve));
console.log("serving on http://127.0.0.1:7685");

try {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  await page.goto("http://127.0.0.1:7685/");
  await page.waitForSelector(".drawio-viewer svg, .drawio-viewer iframe", { timeout: 10000 });
  const outDir = path.join(repoRoot, "artifacts");
  fs.mkdirSync(outDir, { recursive: true });
  const outPath = path.join(outDir, "unit-09.png");
  await page.screenshot({ path: outPath, fullPage: true });
  const size = fs.statSync(outPath).size;
  console.log(`screenshot ${outPath} size=${size}`);
  if (size < 10 * 1024) {
    throw new Error(`screenshot too small: ${size} bytes`);
  }
  await browser.close();
} finally {
  server.close();
}
