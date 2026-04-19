// Fallback dev server used when the Rust toolchain is unavailable.
// Mirrors the HTML that `examples/demo_serve.rs` would emit for
// fixtures/plotly.md by inlining the vendored plotly.min.js + init script.
import { createServer } from "node:http";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const vendor = resolve(here, "..", "..", "crates", "mdview-ext-plotly", "assets", "vendor");
const fixturePath = resolve(here, "..", "..", "fixtures", "plotly.md");

const plotlyJs = readFileSync(resolve(vendor, "plotly.min.js"), "utf8");
const initJs = readFileSync(resolve(vendor, "mdv-plotly-init.js"), "utf8");
const md = readFileSync(fixturePath, "utf8");

function extractPlotlyBlock(source) {
  const match = source.match(/```plotly\r?\n([\s\S]*?)\r?\n```/);
  if (!match) return "null";
  try {
    const obj = JSON.parse(match[1]);
    return JSON.stringify(obj);
  } catch {
    return "null";
  }
}

function escapeAttr(s) {
  return s
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

const spec = extractPlotlyBlock(md);
const body = `<h1>Plotly fixture</h1><p>A simple scatter plot served via the mdview plotly extension.</p>` +
  `<div class="plotly-chart" data-spec="${escapeAttr(spec)}"></div>`;

const html =
  `<!doctype html><html><head><meta charset="utf-8"><title>mdview plotly demo</title>` +
  `<style>body{font-family:ui-sans-serif,system-ui;padding:2rem;line-height:1.7}` +
  `.plotly-chart{border-radius:14px;box-shadow:0 1px 3px rgba(0,0,0,.08);padding:1rem;margin:1rem 0;min-height:320px}</style>` +
  `</head><body>${body}<script>${plotlyJs}</script><script>${initJs}</script></body></html>`;

const addr = process.env.MDV_PLOTLY_ADDR || "127.0.0.1:7686";
const [host, portStr] = addr.split(":");
const server = createServer((_req, res) => {
  res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
  res.end(html);
});
server.listen(Number(portStr), host, () => {
  console.log(`demo fallback server listening on http://${addr}`);
});
