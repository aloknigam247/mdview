# Diagram sidecar

## Entrypoints

| File | Responsibility |
|------|----------------|
| [src/dom.ts](src/dom.ts) | Cached JSDOM environment and SVG/canvas shims |
| [src/index.ts](src/index.ts) | Command/response types, dispatch, stdin framing, ordered responses |
| [src/renderers/drawio.ts](src/renderers/drawio.ts) | In-process mxGraph XML-to-SVG conversion |
| [src/renderers/mermaid.ts](src/renderers/mermaid.ts) | Lazy Mermaid import and SVG rendering |
| [src/renderers/plotly.ts](src/renderers/plotly.ts) | Plot specification parsing, SVG export, DOM cleanup |
| [test/renderers.test.ts](test/renderers.test.ts) | Dispatcher and renderer behavior tests |

## Process boundary

The JSON-lines protocol is defined by `Command`, `Response`, and `writeLine` in `src/index.ts`.
Stdout contains JSON success/error responses, not bare SVG. Keep diagnostics off stdout and preserve ordered processing
and the final unterminated-line handling.

Before changing framing or payloads, compare the Rust callers:

- [Draw.io](../crates/mdview-ext-drawio/src/sidecar.rs)
- [Mermaid](../crates/mdview-ext-mermaid/src/sidecar.rs)
- [Plotly](../crates/mdview-ext-plotly/src/lib.rs)

These callers read stdout as bare SVG and check only exit status, so error responses (which exit 0) bypass their
fallbacks; change both sides together.

Each caller owns executable discovery and its terminal fallback. Do not infer native GUI behavior from these paths;
see the [app page guide](../apps/mdview/AGENTS.md#native-host-and-page).

## Commands

From `sidecar\`, use the scripts in [package.json](package.json):

```ps1
bun run build
bun test
```

For a workspace release with the sidecar beside the app, run `pwsh -NoProfile -File .\scripts\build.ps1` from the
workspace root; [that script](../scripts/build.ps1) owns the output location and dependency setup.
