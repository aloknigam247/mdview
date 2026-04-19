import { getDom } from "../dom.js";

type PlotlyLike = {
  newPlot: (gd: HTMLElement, data: unknown, layout?: unknown, config?: unknown) => Promise<HTMLElement>;
  toImage: (gd: HTMLElement, opts: { format: string; width?: number; height?: number }) => Promise<string>;
  purge: (gd: HTMLElement) => void;
};

let plotlyModule: PlotlyLike | null = null;

async function loadPlotly(): Promise<PlotlyLike> {
  if (plotlyModule) return plotlyModule;
  getDom();
  const mod = (await import("plotly.js-dist-min")) as unknown as { default: PlotlyLike } | PlotlyLike;
  const m = (mod as { default?: PlotlyLike }).default ?? (mod as PlotlyLike);
  plotlyModule = m;
  return m;
}

type Spec = { data?: unknown; layout?: unknown; config?: unknown };

function parseSpec(source: string): Spec {
  const trimmed = source.trim();
  if (!trimmed) return {};
  const parsed = JSON.parse(trimmed) as unknown;
  if (Array.isArray(parsed)) return { data: parsed };
  if (parsed && typeof parsed === "object") return parsed as Spec;
  throw new Error("plotly: spec must be a JSON object or array of traces");
}

export async function renderPlotly(source: string, opts?: Record<string, unknown>): Promise<string> {
  const p = await loadPlotly();
  const spec = parseSpec(source);
  const doc = (globalThis as unknown as { document: Document }).document;
  const gd = doc.createElement("div");
  gd.style.width = "640px";
  gd.style.height = "480px";
  doc.body.appendChild(gd);

  try {
    const data = (spec.data ?? []) as unknown;
    const layout = (spec.layout ?? {}) as Record<string, unknown>;
    await p.newPlot(gd, data, layout, { displayModeBar: false, staticPlot: true });
    const width = Number(opts?.width ?? 640);
    const height = Number(opts?.height ?? 480);
    const dataUrl = await p.toImage(gd, { format: "svg", width, height });
    if (!dataUrl.startsWith("data:image/svg+xml")) throw new Error("plotly: toImage did not return SVG");
    const comma = dataUrl.indexOf(",");
    const payload = dataUrl.slice(comma + 1);
    const svg = decodeURIComponent(payload);
    return svg;
  } finally {
    try { p.purge(gd); } catch { /* ignore */ }
    gd.remove();
  }
}
