import { getDom } from "../dom.js";

type MermaidLike = {
  initialize: (cfg: Record<string, unknown>) => void;
  render: (id: string, src: string) => Promise<{ svg: string }>;
};

let mermaidModule: MermaidLike | null = null;

async function loadMermaid(): Promise<MermaidLike> {
  if (mermaidModule) return mermaidModule;
  getDom();
  const mod = (await import("mermaid")) as unknown as { default: MermaidLike };
  const m = mod.default;
  m.initialize({ startOnLoad: false, securityLevel: "loose", theme: "default" });
  mermaidModule = m;
  return m;
}

export async function renderMermaid(source: string, opts?: Record<string, unknown>): Promise<string> {
  const m = await loadMermaid();
  if (opts && typeof opts === "object") {
    try { m.initialize({ startOnLoad: false, securityLevel: "loose", ...opts }); } catch { /* ignore */ }
  }
  const id = "mdview-mermaid-" + Math.random().toString(36).slice(2);
  const { svg } = await m.render(id, source);
  return svg;
}
