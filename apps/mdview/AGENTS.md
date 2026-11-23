# mdview app

## Entrypoints

| File | Responsibility |
|------|----------------|
| [build.rs](build.rs) | Icon generation and webview build invocation |
| [src/builtins.rs](src/builtins.rs) | App extension registry; follow the [registration rules](../../crates/AGENTS.md#extension-points) |
| [src/cli.rs](src/cli.rs) | Clap flags and `Cli::mode` selection |
| [src/daemonize.rs](src/daemonize.rs) | Platform detachment and child marker |
| [src/main.rs](src/main.rs) | File/config preflight, runtime creation, mode dispatch |
| [src/pipeline.rs](src/pipeline.rs) | Surface orchestration, native window, IPC, reload, DWM |
| [src/profile.rs](src/profile.rs) | Launch timing output |
| [src/render.rs](src/render.rs) | `render_page_full`, `wrap_page`, GUI CSS and JavaScript |
| [src/render_terminal.rs](src/render_terminal.rs) | App ANSI rendering and block layout |
| [src/server.rs](src/server.rs) | `serve_html` snapshots and `serve_dir` request-time rendering; keep both listeners loopback-only |
| [tests/](tests/) | Binary-input, config-error, detachment, and terminal-theme integration tests |
| [webview/src/client.ts](webview/src/client.ts) | Separate bundled WebSocket client |

## Native host and page

The default `gui` feature in [Cargo.toml](Cargo.toml) uses **wry + tao**, not Tauri.
`Mode::Tauri` and `run_tauri_child` are identifiers for that path, not evidence of a Tauri runtime.
`pipeline.rs::run_gui_event_loop` creates the window and webview; `with_ipc_handler` forwards messages through
`MdvUserEvent` and the tao event-loop proxy.

The page comes from `render.rs::wrap_page`, including bionic reading, codemap, context menu, keymaps, theme toggles,
TOC, and zoom. Edit its inline CSS/JS rather than assuming `webview/dist/client.js` serves the native window.
`Features::detect` and `build_lib_scripts` select CDN libraries; keep their loading paired with the inline diagram
initializers before changing eager or lazy loading.

The library HTML stylesheet targets `.mdv-doc`; this app emits `article.mdv`. Adapt needed rules here rather than
importing `mdview-render-html::base_stylesheet()` wholesale.
See [renderer boundaries](../../crates/AGENTS.md#renderers).

For GUI reload, replace `Server.html` with newly rendered HTML **before** calling `webview.reload()`.
`serve_html` clones the snapshot under a read lock; reloading alone cannot update it.
`ReloadCtx` retains launch configuration, whereas `run_serve` reloads configuration for each document request.

## Builds and launches

[webview/package.json](webview/package.json) defines `bun run build` from `apps\mdview\webview`.
[webview/build.mjs](webview/build.mjs) bundles `src/client.ts` with esbuild into `dist/client.js`.
The Cargo build script invokes Bun or npm and can create placeholders when skipped or unsuccessful;
`MDVIEW_SKIP_WEBVIEW_BUILD` controls the skip path. A placeholder is not evidence that the bundle was built.

Run from the workspace root:

```ps1
cargo run --locked -p mdview -- fixtures\everything.md
cargo run --locked -p mdview -- --terminal fixtures\everything.md
```

Set `MDV_PROFILE=1` for launch timings on stderr; it bypasses detachment. Clear it after profiling.
`MDVIEW_NO_DAEMONIZE` keeps GUI integration runs in-process. Browser harness commands belong to the
[test guide](../../tests/AGENTS.md).

## Windows boundaries

- Canonical paths can have a `\\?\` prefix; strip it before URL encoding or display, as in
  `render.rs::resolve_to_mdview_url`.
- DWM titlebar updates belong in `pipeline.rs::apply_dwm_theme`. Reuse `parse_hex_to_colorref` for Windows color
  packing and keep the unsafe exception scoped to the FFI helper.
- Executable locks: close the owned `mdview.exe` before rebuilding that binary; do not terminate unrelated instances.
- Image URLs must use the platform-specific `MDVIEW_PROTOCOL_BASE` in `render.rs` and be rewritten before comrak
  serializes the AST. Keep it paired with the custom-protocol handler in `pipeline.rs`.
