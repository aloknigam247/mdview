# AGENTS.md — mdview

## Project
`mdview` is a lightweight, extensible markdown renderer built in **Rust** (core,
renderers, Tauri shell, CLI) with a tiny **Bun-compiled TypeScript sidecar**
used only when rendering mermaid / drawio / plotly diagrams to the terminal.

### Output surfaces
1. **Tauri desktop app** (default). `mdview FILE.md` launches the window and detaches.
2. **Terminal** (`mdview --terminal FILE.md`). ANSI + sixel + built-in pager.
3. **Neovim live preview** via a Lua plugin (`:MdView`). Streams the buffer
   debounced (~100 ms) on `TextChanged` / `TextChangedI`. Theme syncs from the
   current colorscheme and is cached on disk.

### Look and feel
Modern, sleek, **curved** — CSS border-radius in HTML, `╭╮╰╯` box-drawing for
tables / code blocks in the terminal.

## Tech stack
- **Rust** everywhere in the core. `comrak` for parsing. `syntect` for
  highlighting. `latex2mathml` for terminal math. `ratatui` + `crossterm` for
  the pager. `axum` + `tokio-tungstenite` for the embedded HTTP+WS server
  (loopback only, used by the Tauri webview). `resvg` for SVG→PNG.
  `sixel-rs` for sixel encoding. `rmpv` + `tokio` for the nvim pipe.
- **Tauri v2** shell; single binary combines CLI + GUI.
- **Webview client bundle** (inside Tauri): vanilla TS bundling `mermaid`,
  `plotly.js-dist-min`, `drawio-viewer`, `katex`, plus a tiny live-reload
  WS client. Built with `esbuild`. Embedded as static assets at compile time.
- **Sidecar** (`sidecar/`): Bun-compiled TS binary. Reads JSON jobs from stdin
  (`{kind: "mermaid"|"drawio"|"plotly", source, opts}`), writes SVG to stdout.
  Spawned lazily by extension crates for the terminal path. If missing,
  extensions emit an ASCII placeholder and log a warning.

## Monorepo layout
Cargo workspace. Every crate owns a disjoint directory. Crates depend on
`mdview-core` (trait + types) and optionally `mdview-theme`; extensions never
depend on each other.

## Plugin contract (Rust trait — never break)
```rust
pub trait MdViewExtension: Send + Sync {
    fn name(&self) -> &'static str;
    fn register_parser(&self, _opts: &mut comrak::ComrakOptions) {}
    fn pre_parse(&self, _src: &mut String) {}            // for brand-new syntax
    fn transform(&self, _ast: &mut comrak::nodes::AstNode) {}
    fn render_html(&self, _n: &AstNode, _ctx: &RenderCtx) -> Option<Html> { None }
    fn render_terminal(&self, _n: &AstNode, _ctx: &RenderCtx) -> Option<TermChunks> { None }
    fn client_assets(&self) -> &'static [Asset] { &[] }
}
```

## Theme contract (never break)
```rust
pub struct Theme {
    pub name: &'static str,
    pub colors: BTreeMap<&'static str, &'static str>,  // fg, bg, accent, muted, code.bg, link, …
    pub styles: BTreeMap<&'static str, StyleSpec>,     // heading.1, blockquote, table.header, …
    pub radii: Radii,                                  // sm, md, lg
    pub typography: Typography,                        // body, mono, headings
}
```
All themes are built-in presets in `crates/mdview-theme/src/themes/`. Nvim sync
synthesises a `Theme` at runtime and caches it by colorscheme name.

## Commands
- `cargo build --workspace`
- `cargo test --workspace`
- `cargo run -p mdview -- fixtures/everything.md`            # Tauri
- `cargo run -p mdview -- --terminal fixtures/everything.md` # pager
- `cd sidecar && bun build --compile ./src/index.ts --outfile ../target/release/mdview-sidecar`
- `cd apps/mdview/webview && bun run build`                  # esbuild → dist/
- `cd tests/e2e && bun run test:e2e`                         # Playwright
- `$env:MDV_PROFILE=1; cargo run -p mdview -- fixtures\showcase.md` — prints launch-path timings to stderr.

## Conventions
- Rust 2021 edition; stable toolchain; `#![deny(unsafe_code)]` unless justified.
- `cargo fmt` + `cargo clippy -- -D warnings` must pass.
- After implementing a feature and all tests pass, always run
  `cargo build --release -p mdview` so the release binary stays current.
- Conventional-commit messages (`feat(core): …`).
- Alphabetical ordering for new items; never reorder existing entries.
- No comments unless the *why* is non-obvious.
- Never reinvent: prefer `comrak`, `syntect`, `latex2mathml`, `resvg`,
  `ratatui`, `crossterm`, `axum`, `sixel-rs`.
- Each crate must pass `cargo test` in isolation. If a crate needs a type
  from a sibling, depend on that sibling crate and import the real type. Do
  not duplicate sibling types behind local stubs.

## Where to extend
- **Bundle a new language syntax (not in `load_defaults_newlines`)** →
  vendor the `.sublime-syntax` file under
  `crates/mdview-ext-highlight/assets/<Lang>.sublime-syntax`, attribute the
  source license under `LICENSES/`, and register it in `SYNTAX_SET`'s
  builder. See the PowerShell entry as a template.
- **New codeblock language alias** → add a match arm in
  `canonical_lang` in `crates/mdview-ext-highlight/src/lib.rs` mapping the
  lowercase fenced-block hint to syntect's canonical syntax name. Inspect
  the bundled set via `SyntaxSet::find_syntax_by_name` to discover the right
  name. Keep entries alphabetical. Add a unit test in the same file.
- **New extension** → new crate under `crates/mdview-ext-<name>/`, implement
  `MdViewExtension`, register in `apps/mdview/src/builtins.rs` — **not**
  `crates/mdview-core/src/builtins.rs`, which is an unused empty stub. This
  registry is an exception to the alphabetical-ordering convention: it is
  ordered semantically. Add selective-match extensions (those gated on a
  specific fenced-block info-string) anywhere **before** the catch-all
  `Highlight`, which must stay **last** — it matches any code block and would
  otherwise shadow the diagram extensions.
- **New theme** → `crates/mdview-theme/src/themes/<name>.rs`, register in
  `crates/mdview-theme/src/presets.rs`.
- **New CLI flag** → `apps/mdview/src/cli.rs` (clap-based).
- **New output surface** → new `crates/mdview-render-<name>/` consuming
  `(AstNode, RenderCtx)`; wire into `apps/mdview/src/main.rs`.

## Non-goals
- Dynamic / user-file / npm extensions — built-ins only.
- User JSON theme loader — built-ins only.
- OS file-association handler — out of scope.
- In-app file menu, Ctrl+O, drag-drop — `mdview FILE` is the only entry point.
- Full HTML/CSS fidelity beyond GFM + shipped extensions.
- Wysiwyg editing, collaborative editing.

## Configuration

User configuration lives at `$XDG_CONFIG_HOME/mdview/config.toml` (with
`$HOME/.config/mdview/config.toml` as the fallback; on Windows `%USERPROFILE%`
substitutes for `$HOME`). If the file is not present, defaults are used silently.
The config is never auto-created.

**Bindings are opt-in — there are no built-in defaults. If an action is not
listed in the user's config, it has no binding.**

### Sections

- `[code]` — `tab_width` (integer 1..=8; default 4). Controls how tab characters
  render in code blocks on both GUI (CSS `tab-size`) and terminal (expanded to spaces).
- `[toc]` — `position` (one of `floating-right`, `floating-left`, `floating-center`,
  `fixed-right`, `fixed-left`, `inline`; default `floating-right`), `depth` (1..=6; default 3).
- `[codemap]` — `enabled` (bool; default `true`).
- `[theme]` — `mode` (`auto` | `light` | `dark`; default `auto`; follows OS at launch on Windows, otherwise dark), `light` (theme preset name; default `catppuccin-latte`),
  `dark` (theme preset name; default `catppuccin-mocha`). Manual toggles via
  the `toggle-theme` action are ephemeral (not persisted back to config).
- `[keymap]` — opt-in bindings. Available actions:
  - `quit` — close the mdview window / exit the pager.
  - `toggle-bionic` — toggle bionic-reading transform (bolden first half of each word).
  - `toggle-codemap` — show / hide the right-edge minimap.
  - `toggle-theme` — flip between the configured light and dark themes.
  - `toggle-toc` — show / hide the floating table of contents.

**Update this list on every config change.**

### Error reporting

Every key/value error in `config.toml` is **non-fatal**: mdview always launches
with sensible defaults and merely records what went wrong. Errors are surfaced
both on `stderr` (one line per error, prefixed `mdview-config-error:`) and
**in-app**:

- **Pager** (`--terminal`): a yellow top-of-screen banner shows the first error
  (plus a count when there's more than one); press `Esc` to dismiss.
- **Nvim** (`--nvim-socket`): the Lua plugin's `on_stderr` callback strips the
  prefix and calls `vim.notify(..., vim.log.levels.WARN)`.
- **GUI** (wry webview): a dismissible amber banner pinned to the top of the
  page; shows the first error (plus a count when there's more than one) with a
  "Show all" expander; close button or `Esc` dismisses.

Each error names the culprit (key like `keymap[quit]`, the raw offending value,
or `config.toml line L:C` when TOML span info is available), states what's
wrong, and lists what was expected. Programmatic access is via
`Config::load_full() -> LoadResult { config, errors }`; the existing
`Config::load() -> Config` shim is preserved for backwards compatibility.

## Architecture reality (read before editing the app)

The aspirational lines higher up describe a Tauri shell and a separate
`apps/mdview/webview/` esbuild bundle. The **actual code** does not match:

- **Shell**: plain `wry` + `tao` (not Tauri). `apps/mdview/src/pipeline.rs::run_gui_event_loop`
  builds the `tao::Window` + `wry::WebView` directly. There are no Tauri
  commands; host ↔ webview communication goes through `with_ipc_handler` +
  `EventLoopBuilder::<MdvUserEvent>::with_user_event` + a tao `EventLoopProxy`.
- **Webview UI**: a single huge inline `<style>` + `<script>` block inside a
  Rust `format!` raw-string template in `apps/mdview/src/render.rs::wrap_page`.
  `apps/mdview/webview/` exists but its build script fails silently (you'll see
  `warning: mdview@0.1.0: webview build failed; using placeholder bundle` on
  every build — that's normal). Treat `render.rs` as the source of truth for
  CSS and JS in the GUI surface.
- **Most "webview" features live in `render.rs`** as embedded JS/CSS — TOC
  builder, codemap minimap, right-click menu, zoom, bionic, theme toggle,
  context menu, keymap dispatcher, config-error banner. Do not assume a `dist/`
  bundle is loaded.
- The `mdview-render-html::base_stylesheet()` function uses `.mdv-doc`
  selectors; the app emits `<article class="mdv">`. The two CSS surfaces are
  not unified — copy needed rules into `render.rs` rather than importing
  `base_stylesheet()` wholesale.
- **Extension init scripts auto-scan on `DOMContentLoaded`.**
  `crates/mdview-ext-{mermaid,plotly,drawio}` each ship a `vendor/mdv-<name>-init.js`
  asset that walks the DOM (`querySelectorAll('.mermaid')`, etc.) and calls
  the library's renderer immediately. To lazy-load the CDN library tag
  without crashing those inits, each ext needs a contract: the init script
  exposes `window.__mdv_render_<name>(elements)` and **does not** auto-run on
  DOMContentLoaded; a central lazy loader in `render.rs` invokes it via
  `IntersectionObserver` after the library script has finished loading. Not
  yet implemented — the scaled-back phase 2 of perf work only added `<img
  loading="lazy">` and head minification.
- **Server serves a snapshot.** `apps/mdview/src/server.rs` holds the HTML in
  `HtmlStore = Arc<RwLock<Arc<String>>>` exposed as `Server.html`. `webview.reload()`
  alone just re-fetches the same snapshot — it does **not** pick up file or
  config changes. Any live-update feature (file-watcher reload, manual reload,
  hot theme swap, etc.) must re-render and swap the inner `Arc<String>` *before*
  calling `webview.reload()`. The serve loop clones the inner `Arc` per request,
  so swaps are lock-free for readers.

## Working with parallel agents (worktree workflow)

When firing background agents that touch the same hot file (especially
`apps/mdview/src/render.rs` or `apps/mdview/src/pipeline.rs`), use
`isolation: "worktree"` so each agent operates on its own branch. After each
agent completes, **verify its work landed in main** before treating it as done.

Lessons from the field:

- Auto-merge from worktree back to the main worktree's working tree is **leaky
  on hot files**: when multiple worktree agents touch the same file, later
  merges silently drop earlier work. Verify by grepping for sentinel
  identifiers (`load_icon`, `pre_render_html`, `setupToc`, etc.) — see
  `scripts/verify-features.ps1`.
- An agent that uses a full-file `create` (or `Set-Content` / `WriteAllText`
  fallback) can erase any change between its initial read and its write.
  **Prefer targeted `edit` calls** in agent prompts; only allow full-file writes when
  the agent has just read the full file and preserves everything it isn't
  changing.
- Agent self-reports occasionally describe what was intended, not what
  survived. Grep before declaring victory.
- Shell CWD persists across powershell tool calls and can drift into an agent's
  worktree directory after the agent finishes. Always `cd D:\mdview` before
  `git merge --squash <branch>` on main. Symptom: merge reports "Already up
  to date" despite divergent commits — that's the branch being merged into
  itself.
- Diff a stale-base worktree branch against main with **three dots**:
  `git diff main...worktree-agent-XXX --stat` shows what the branch *added*
  since the merge-base. Two dots (`main..branch`) shows the symmetric set
  and surfaces main's recent commits as "removals", which is misleading.
- Worktree agents will sometimes pick up unrelated `cargo fmt` / `clippy
  --fix` cleanups across the workspace to make `-D warnings` pass on a
  stale base. Benign but inflates diffs; review before merging.
- On Windows, a running `mdview.exe` from a previous `cargo run` (e.g. the
  GUI you opened to validate the last task) keeps `target/debug/mdview.exe`
  locked and blocks `cargo build`. Kill the process before rebuilding.
- `MDV_PROFILE=1` skips the daemonize step so profile lines logged after
  the would-be detach still reach the parent terminal's stderr. Pattern
  for env-gated debug modes: bypass detachment so post-fork output isn't
  swallowed.

## Platform-specific notes

### wry custom URI scheme (Windows)

The `with_custom_protocol("<scheme>", ...)` handler on Windows WebView2
responds to URLs of the form `http://<scheme>.localhost/<path>`, **not**
`<scheme>://localhost/<path>`. On Linux/macOS the bare scheme works. Use a
`cfg`-gated constant:

```rust
#[cfg(windows)]
const MDVIEW_PROTOCOL_BASE: &str = "http://mdview.localhost/";
#[cfg(not(windows))]
const MDVIEW_PROTOCOL_BASE: &str = "mdview://localhost/";
```

For images, pre-walk the AST and rewrite `NodeValue::Image(link).url` to an
absolute path under this base before letting `comrak::format_html` serialize.

### Windows titlebar via DWM

To recolor the native titlebar, call `DwmSetWindowAttribute` with
`DWMWA_CAPTION_COLOR`, `DWMWA_TEXT_COLOR`, `DWMWA_BORDER_COLOR`, and
`DWMWA_USE_IMMERSIVE_DARK_MODE`. The Windows `COLORREF` is `0x00BBGGRR` — byte-
swap from the usual `0x00RRGGBB` hex. The FFI block needs `#[allow(unsafe_code)]`
because the crate has `#![deny(unsafe_code)]` at the root.

### `std::fs::canonicalize` on Windows

Returns `\\?\` UNC paths. Strip the prefix before URL-encoding or
embedding in any user-facing string.
