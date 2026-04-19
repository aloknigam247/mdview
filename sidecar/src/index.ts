import { renderDrawio } from "./renderers/drawio.js";
import { renderMermaid } from "./renderers/mermaid.js";
import { renderPlotly } from "./renderers/plotly.js";

type Kind = "drawio" | "mermaid" | "plotly";

type Command = {
  kind: Kind;
  source: string;
  opts?: Record<string, unknown>;
};

type Response =
  | { ok: true; svg: string }
  | { ok: false; error: string };

function writeLine(obj: Response): void {
  process.stdout.write(JSON.stringify(obj) + "\n");
}

export async function dispatch(cmd: Command): Promise<Response> {
  try {
    if (!cmd || typeof cmd !== "object") throw new Error("command must be a JSON object");
    if (typeof cmd.source !== "string") throw new Error("command.source must be a string");
    let svg: string;
    switch (cmd.kind) {
      case "drawio":
        svg = await renderDrawio(cmd.source, cmd.opts);
        break;
      case "mermaid":
        svg = await renderMermaid(cmd.source, cmd.opts);
        break;
      case "plotly":
        svg = await renderPlotly(cmd.source, cmd.opts);
        break;
      default:
        throw new Error(`unknown kind: ${String((cmd as { kind: unknown }).kind)}`);
    }
    if (typeof svg !== "string" || svg.length === 0) throw new Error("renderer returned empty SVG");
    return { ok: true, svg };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return { ok: false, error: msg };
  }
}

async function processLine(line: string): Promise<void> {
  const trimmed = line.trim();
  if (!trimmed) return;
  let cmd: Command;
  try {
    cmd = JSON.parse(trimmed) as Command;
  } catch (e) {
    writeLine({ ok: false, error: "invalid JSON: " + (e instanceof Error ? e.message : String(e)) });
    return;
  }
  const res = await dispatch(cmd);
  writeLine(res);
}

async function main(): Promise<void> {
  const stdin = process.stdin;
  stdin.setEncoding("utf8");

  let buffer = "";
  let chain: Promise<void> = Promise.resolve();

  stdin.on("data", (chunk: string) => {
    buffer += chunk;
    let idx: number;
    while ((idx = buffer.indexOf("\n")) >= 0) {
      const line = buffer.slice(0, idx);
      buffer = buffer.slice(idx + 1);
      chain = chain.then(() => processLine(line));
    }
  });

  await new Promise<void>((resolve) => {
    stdin.once("end", () => {
      if (buffer.length > 0) {
        const last = buffer;
        buffer = "";
        chain = chain.then(() => processLine(last));
      }
      chain.then(() => resolve());
    });
  });
}

const isMain = import.meta.main ?? (import.meta.url === `file://${process.argv[1]}`);
if (isMain) {
  main().catch((err) => {
    writeLine({ ok: false, error: err instanceof Error ? err.message : String(err) });
    process.exit(1);
  });
}
