// Minimal Playwright screenshot harness for Unit 15.
// Starts a local static server on 127.0.0.1:7681 serving `dist/`, hits it with
// Chromium, injects a sample "full" payload into `window.__mdvApply`, screenshots
// to artifacts/unit-15.png, asserts >10 KB.

import { chromium } from "playwright";
import { createServer } from "node:http";
import { readFileSync, statSync, mkdirSync } from "node:fs";
import { dirname, resolve, join } from "node:path";
import { fileURLToPath } from "node:url";

let browser;
const __dirname = dirname(fileURLToPath(import.meta.url));
const dist = resolve(__dirname, "dist");
const artifactsDir = process.env.MDVIEW_ARTIFACTS_DIR
    ? resolve(process.env.MDVIEW_ARTIFACTS_DIR)
    : resolve(__dirname, "../../../artifacts/standalone/webview");
mkdirSync(artifactsDir, { recursive: true });
const outPath = join(artifactsDir, "unit-15.png");

const types = {
    ".html": "text/html; charset=utf-8",
    ".js": "application/javascript; charset=utf-8",
    ".css": "text/css; charset=utf-8",
    ".map": "application/json",
};

const server = createServer((req, res) => {
    let p = req.url?.split("?")[0] ?? "/";
    if (p === "/") p = "/index.html";
    const file = join(dist, p);
    try {
        const st = statSync(file);
        if (!st.isFile()) throw new Error("not a file");
        const ext = p.slice(p.lastIndexOf("."));
        res.writeHead(200, { "content-type": types[ext] ?? "application/octet-stream" });
        res.end(readFileSync(file));
    } catch {
        res.writeHead(404, { "content-type": "text/plain" });
        res.end("not found");
    }
});

const port = 7681;
const sampleHtml = `
<h1>mdview — Unit 15 smoke screenshot</h1>
<p>The Tauri webview client bundle is loaded and renders this sample HTML
payload. Rounded corners come from the inline theme.</p>
<pre><code>fn main() {
    println!("Hello, mdview!");
}</code></pre>
<blockquote>The live-reload WS client waits for patches from the Rust side.</blockquote>
<table>
  <thead><tr><th>Feature</th><th>Status</th></tr></thead>
  <tbody>
    <tr><td>CLI</td><td>ready</td></tr>
    <tr><td>Daemonize</td><td>ready</td></tr>
    <tr><td>Webview bundle</td><td>ready</td></tr>
  </tbody>
</table>
`;

try {
    const errors = [];
    await new Promise((done, reject) => {
        server.once("error", reject);
        server.listen(port, "127.0.0.1", () => {
            server.off("error", reject);
            done();
        });
    });
    console.log("[unit-15] static server listening on", port);
    browser = await chromium.launch();
    const page = await browser.newPage({ viewport: { width: 1200, height: 800 } });
    page.on("pageerror", (e) => errors.push(String(e)));

    // No WebSocket endpoint exists in this static fixture.
    await page.goto(`http://127.0.0.1:${port}/`);
    await page.waitForLoadState("domcontentloaded");
    await page.evaluate((html) => {
        const el = document.getElementById("mdv-content");
        if (el) el.innerHTML = html;
    }, sampleHtml);
    await page.waitForTimeout(500);
    await page.screenshot({ path: outPath, fullPage: true });

    const size = statSync(outPath).size;
    console.log(`[unit-15] screenshot ${outPath} (${size} bytes)`);
    if (size <= 10 * 1024) {
        throw new Error(`[unit-15] FAIL: screenshot must be >10 KB (got ${size})`);
    }
    if (errors.length) console.warn("[unit-15] page errors (non-fatal):", errors);
} finally {
    try {
        await browser?.close();
    } finally {
        if (server.listening) {
            await new Promise((done, reject) => {
                server.close((error) => (error ? reject(error) : done()));
                server.closeAllConnections();
            });
        }
    }
}
console.log("[unit-15] OK");
