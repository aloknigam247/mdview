# CLAUDE.md — mdview

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

## Conventions
- Rust 2021 edition; stable toolchain; `#![deny(unsafe_code)]` unless justified.
- `cargo fmt` + `cargo clippy -- -D warnings` must pass.
- Conventional-commit messages (`feat(core): …`).
- Alphabetical ordering for new items; never reorder existing entries.
- No comments unless the *why* is non-obvious.
- Never reinvent: prefer `comrak`, `syntect`, `latex2mathml`, `resvg`,
  `ratatui`, `crossterm`, `axum`, `sixel-rs`.
- Each crate must pass `cargo test` in isolation. If a crate needs a type
  from a sibling that doesn't exist in the worktree, add a local
  `src/_stubs.rs` with a minimal definition and
  `// TODO: replace with mdview_<sibling> after integration`.

## Where to extend
- **New extension** → new crate under `crates/mdview-ext-<name>/`, implement
  `MdViewExtension`, register in `crates/mdview-core/src/builtins.rs` (alphabetical).
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
substitutes for `$HOME`). On first run mdview writes a fully-commented template;
failures to read or write are non-fatal.

**Bindings are opt-in — there are no built-in defaults. If an action is not
listed in the user's config, it has no binding.**

### Sections

- `[toc]` — `position` (one of `floating-right`, `floating-left`, `floating-center`,
  `fixed-right`, `fixed-left`, `inline`; default `floating-right`), `depth` (1..=6; default 3).
- `[codemap]` — `enabled` (bool; default `true`).
- `[theme]` — `mode` (`auto` | `light` | `dark`; default `auto`; v1 resolves
  `auto` to `dark`), `light` (theme preset name; default `catppuccin-latte`),
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
- **GUI** (Tauri webview): TODO — a small follow-up will read
  `LoadResult.errors` in `apps/mdview/src/render.rs` and render a dismissible
  toast at the top of the page.

Each error names the culprit (key like `keymap[quit]`, the raw offending value,
or `config.toml line L:C` when TOML span info is available), states what's
wrong, and lists what was expected. Programmatic access is via
`Config::load_full() -> LoadResult { config, errors }`; the existing
`Config::load() -> Config` shim is preserved for backwards compatibility.
