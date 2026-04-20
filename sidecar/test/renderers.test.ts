import { describe, expect, test } from "bun:test";
import { dispatch } from "../src/index.ts";

const MERMAID_TIMEOUT_MS = 30_000;
const PLOTLY_TIMEOUT_MS = 30_000;

describe("mermaid renderer", () => {
  test("returns non-empty SVG for a simple graph", async () => {
    const res = await dispatch({ kind: "mermaid", source: "graph TD; A-->B;" });
    expect(res.ok).toBe(true);
    if (res.ok) {
      expect(res.svg.length).toBeGreaterThan(0);
      expect(res.svg).toContain("<svg");
      expect(res.svg).toContain("</svg>");
    }
  }, MERMAID_TIMEOUT_MS);

  test("reports errors on invalid input", async () => {
    const res = await dispatch({ kind: "mermaid", source: "!!not a mermaid diagram!!" });
    expect(res.ok).toBe(false);
  }, MERMAID_TIMEOUT_MS);
});

describe("plotly renderer", () => {
  test("renders a scatter spec to SVG", async () => {
    const spec = JSON.stringify({
      data: [{ x: [1, 2, 3, 4], y: [10, 15, 13, 17], type: "scatter" }],
      layout: { title: { text: "demo" }, width: 320, height: 240 },
    });
    const res = await dispatch({ kind: "plotly", source: spec });
    expect(res.ok).toBe(true);
    if (res.ok) {
      expect(res.svg.length).toBeGreaterThan(0);
      expect(res.svg).toContain("<svg");
    }
  }, PLOTLY_TIMEOUT_MS);
});

describe("drawio renderer", () => {
  test("renders a minimal mxGraphModel to SVG", async () => {
    const xml = `<?xml version="1.0" encoding="UTF-8"?>
<mxGraphModel>
  <root>
    <mxCell id="0"/>
    <mxCell id="1" parent="0"/>
    <mxCell id="2" value="Start" style="rounded=1;fillColor=#dae8fc;strokeColor=#6c8ebf" vertex="1" parent="1">
      <mxGeometry x="40" y="40" width="120" height="40" as="geometry"/>
    </mxCell>
    <mxCell id="3" value="End" style="rounded=1;fillColor=#d5e8d4;strokeColor=#82b366" vertex="1" parent="1">
      <mxGeometry x="240" y="40" width="120" height="40" as="geometry"/>
    </mxCell>
    <mxCell id="4" style="edgeStyle=orthogonalEdgeStyle" edge="1" source="2" target="3" parent="1">
      <mxGeometry relative="1" as="geometry"/>
    </mxCell>
  </root>
</mxGraphModel>`;
    const res = await dispatch({ kind: "drawio", source: xml });
    expect(res.ok).toBe(true);
    if (res.ok) {
      expect(res.svg).toContain("<svg");
      expect(res.svg).toContain("Start");
      expect(res.svg).toContain("End");
    }
  });

  test("rejects malformed XML", async () => {
    const res = await dispatch({ kind: "drawio", source: "<not-xml" });
    expect(res.ok).toBe(false);
  });
});

describe("dispatcher", () => {
  test("rejects unknown kinds", async () => {
    const res = await dispatch({ kind: "wat" as never, source: "x" });
    expect(res.ok).toBe(false);
  });
});
