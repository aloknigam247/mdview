# mdview-sidecar

Standalone Bun-compiled binary that renders diagram sources to SVG for the
terminal path of `mdview`. Only used when the Rust crates need to rasterise
mermaid / drawio / plotly diagrams for the sixel pipeline; the Tauri webview
renders the same diagrams client-side and does not invoke this binary.

## Protocol

* **Transport:** stdin / stdout, one JSON value per line.
* **Request:**
  ```json
  { "kind": "mermaid" | "drawio" | "plotly",
    "source": "<diagram source>",
    "opts":   { /* optional, kind-specific */ } }
  ```
* **Response (one line per request):**
  ```json
  { "ok": true,  "svg": "<svg>...</svg>" }
  { "ok": false, "error": "<message>" }
  ```
* **Shutdown:** the process exits on stdin EOF. The Rust extension crates
  close the child's stdin to terminate it.

Requests are processed sequentially in the order received; responses come out
in the same order, so callers can pipeline multiple diagrams over a single
long-lived sidecar process.

## Spawning from Rust

The extension crates (`mdview-ext-mermaid`, `mdview-ext-drawio`,
`mdview-ext-plotly`) locate the binary via, in order:

1. `MDVIEW_SIDECAR` environment variable.
2. A sibling of the `mdview` executable named `mdview-sidecar` (or
   `mdview-sidecar.exe` on Windows).
3. `target/release/mdview-sidecar` during development.

If the binary is missing or fails to spawn, the extension falls back to an
ASCII placeholder and logs a warning — diagrams are never a hard error.

A typical Rust spawn sequence:

```rust
let mut child = std::process::Command::new(sidecar_path)
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()?;

let stdin = child.stdin.take().unwrap();
writeln!(stdin, "{}", serde_json::to_string(&job)?)?;
// ...read one JSON line from child.stdout...
```

Drop the `stdin` handle when finished so the child sees EOF and exits
cleanly.

## Renderer notes

* **Mermaid:** runs the real `mermaid` npm package inside `jsdom`. Supports
  the full mermaid grammar because the module is untouched.
* **Plotly:** runs `plotly.js-dist-min` inside `jsdom` and calls
  `Plotly.toImage(gd, { format: "svg" })`. The `opts` object may pass
  `{ "width": N, "height": N }`; defaults to 640x480.
* **Drawio:** parses the `<mxGraphModel>` XML in-process and emits an SVG
  directly. `@drawio/viewer` is not an embeddable module, and pulling in
  `puppeteer-core` would require a system Chrome at runtime, which defeats
  the point of a standalone binary. The in-process converter covers the
  geometry + labels + edges needed for the terminal path (which downsamples
  to sixel anyway); the Tauri webview uses the real drawio viewer client
  asset for high-fidelity rendering.

## Build

```sh
bun install
bun test
bun build --compile ./src/index.ts --outfile ./mdview-sidecar
```

Smoke test:

```sh
echo '{"kind":"mermaid","source":"graph TD; A-->B;"}' | ./mdview-sidecar
```

The output is a single JSON line containing `"ok":true` and the rendered SVG.
