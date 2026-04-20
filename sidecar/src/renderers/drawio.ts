// Renderer choice: pure in-process XML → SVG conversion.
//
// Rationale: both alternatives hit dealbreakers for a self-contained Bun binary:
//   * @drawio/viewer is not published as an ESM/CJS package suitable for
//     embedding headlessly; it expects a browser window plus CSS/sprite
//     resources loaded from mxgraph assets.
//   * puppeteer-core would require a system Chrome to be present at runtime,
//     which defeats the point of a standalone sidecar binary.
// So we parse the mxGraphModel XML ourselves and emit a minimal SVG that
// preserves shape geometry, labels, and edge routing. This is sufficient for
// the terminal path (which downsamples the SVG to sixel via resvg) and avoids
// shipping a headless browser.

import { getDom } from "../dom.js";

type Cell = {
  id: string;
  value: string;
  style: string;
  vertex: boolean;
  edge: boolean;
  source?: string;
  target?: string;
  parent?: string;
  geometry?: { x: number; y: number; width: number; height: number };
};

function parseStyle(style: string): Record<string, string> {
  const out: Record<string, string> = {};
  if (!style) return out;
  for (const part of style.split(";")) {
    if (!part) continue;
    const eq = part.indexOf("=");
    if (eq < 0) { out[part] = "1"; continue; }
    out[part.slice(0, eq)] = part.slice(eq + 1);
  }
  return out;
}

function escapeXml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}

function findRootMxCells(doc: Document): Element[] {
  const roots: Element[] = [];
  const all = doc.getElementsByTagName("mxCell");
  for (let i = 0; i < all.length; i++) roots.push(all[i]!);
  return roots;
}

function readCells(doc: Document): Cell[] {
  const cells: Cell[] = [];
  for (const el of findRootMxCells(doc)) {
    const id = el.getAttribute("id") ?? "";
    const value = el.getAttribute("value") ?? "";
    const style = el.getAttribute("style") ?? "";
    const vertex = el.getAttribute("vertex") === "1";
    const edge = el.getAttribute("edge") === "1";
    const source = el.getAttribute("source") ?? undefined;
    const target = el.getAttribute("target") ?? undefined;
    const parent = el.getAttribute("parent") ?? undefined;
    const geomEl = el.getElementsByTagName("mxGeometry")[0];
    let geometry: Cell["geometry"];
    if (geomEl) {
      geometry = {
        x: Number(geomEl.getAttribute("x") ?? 0),
        y: Number(geomEl.getAttribute("y") ?? 0),
        width: Number(geomEl.getAttribute("width") ?? 0),
        height: Number(geomEl.getAttribute("height") ?? 0),
      };
    }
    cells.push({ id, value, style, vertex, edge, source, target, parent, geometry });
  }
  return cells;
}

function bounds(cells: Cell[]): { w: number; h: number } {
  let w = 200;
  let h = 200;
  for (const c of cells) {
    if (!c.geometry) continue;
    const right = c.geometry.x + c.geometry.width;
    const bottom = c.geometry.y + c.geometry.height;
    if (right > w) w = right;
    if (bottom > h) h = bottom;
  }
  return { w: w + 40, h: h + 40 };
}

function renderVertex(c: Cell): string {
  if (!c.geometry) return "";
  const { x, y, width, height } = c.geometry;
  const s = parseStyle(c.style);
  const fill = s.fillColor ?? "#ffffff";
  const stroke = s.strokeColor ?? "#333333";
  const rx = s.rounded === "1" ? 8 : 0;
  const label = escapeXml(c.value);
  const tx = x + width / 2;
  const ty = y + height / 2 + 4;
  return (
    `<g>` +
    `<rect x="${x}" y="${y}" width="${width}" height="${height}" rx="${rx}" ry="${rx}" ` +
    `fill="${fill}" stroke="${stroke}" stroke-width="1"/>` +
    `<text x="${tx}" y="${ty}" text-anchor="middle" ` +
    `font-family="ui-sans-serif, system-ui, sans-serif" font-size="12" fill="#111">${label}</text>` +
    `</g>`
  );
}

function centerOf(c: Cell | undefined): { x: number; y: number } | null {
  if (!c?.geometry) return null;
  return { x: c.geometry.x + c.geometry.width / 2, y: c.geometry.y + c.geometry.height / 2 };
}

function renderEdge(c: Cell, byId: Map<string, Cell>): string {
  const src = c.source ? byId.get(c.source) : undefined;
  const dst = c.target ? byId.get(c.target) : undefined;
  const a = centerOf(src);
  const b = centerOf(dst);
  if (!a || !b) return "";
  const s = parseStyle(c.style);
  const stroke = s.strokeColor ?? "#555";
  return (
    `<line x1="${a.x}" y1="${a.y}" x2="${b.x}" y2="${b.y}" ` +
    `stroke="${stroke}" stroke-width="1.5" marker-end="url(#mdview-arrow)"/>`
  );
}

export async function renderDrawio(source: string, _opts?: Record<string, unknown>): Promise<string> {
  getDom();
  const parser = new (globalThis as unknown as { DOMParser: typeof DOMParser }).DOMParser();
  const doc = parser.parseFromString(source, "application/xml");
  const errNode = doc.getElementsByTagName("parsererror")[0];
  if (errNode) throw new Error("drawio: invalid XML: " + errNode.textContent);

  const cells = readCells(doc);
  if (cells.length === 0) throw new Error("drawio: no <mxCell> elements found");

  const byId = new Map<string, Cell>();
  for (const c of cells) byId.set(c.id, c);

  const { w, h } = bounds(cells);
  const vertices = cells.filter((c) => c.vertex).map(renderVertex).join("");
  const edges = cells.filter((c) => c.edge).map((c) => renderEdge(c, byId)).join("");

  return (
    `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">` +
    `<defs><marker id="mdview-arrow" viewBox="0 0 10 10" refX="10" refY="5" ` +
    `markerWidth="6" markerHeight="6" orient="auto-start-reverse">` +
    `<path d="M 0 0 L 10 5 L 0 10 z" fill="#555"/></marker></defs>` +
    edges +
    vertices +
    `</svg>`
  );
}
