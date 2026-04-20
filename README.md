# mdview

> A lightweight, modern markdown renderer for your terminal, your desktop, and your Neovim buffer.

![mdview screenshot](docs/screenshots/hero.png)

`mdview` renders GitHub-flavored markdown with built-in support for syntax
highlighting, math, mermaid, draw.io, and plotly — all bundled into a single
Rust binary. Three surfaces share the same core:

- **Desktop** (default): a Tauri window with a curvy, modern look and WS
  live-reload.
- **Terminal**: a `ratatui` + `crossterm` pager with rounded box-drawing,
  truecolor ANSI, and sixel-rendered diagrams.
- **Neovim**: a Lua plugin streams your buffer over a pipe, debounced on
  `TextChanged` / `TextChangedI`, with theme sync from your current
  colorscheme.

## Install

### With cargo

```sh
cargo install --locked --path apps/mdview
```

### Prebuilt binaries

Grab the latest archive from the [releases page](https://github.com/aloknigam247/mdview/releases):

| Platform | Archive |
|---|---|
| Windows (x86_64) | `mdview-x86_64-pc-windows-msvc.zip` |
| macOS (Apple Silicon) | `mdview-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `mdview-x86_64-apple-darwin.tar.gz` |
| Linux (x86_64, glibc) | `mdview-x86_64-unknown-linux-gnu.tar.gz` |
| Linux (x86_64, musl) | `mdview-x86_64-unknown-linux-musl.tar.gz` |

Extract and drop `mdview` (or `mdview.exe`) somewhere on your `PATH`.

### From source

```sh
git clone https://github.com/aloknigam247/mdview
cd mdview
cargo build --workspace --release
# Optional: terminal sidecar for mermaid / drawio / plotly in the terminal surface
cd sidecar && bun build --compile ./src/index.ts --outfile ../target/release/mdview-sidecar
```

The resulting binary is `target/release/mdview` (`.exe` on Windows).

## Quick start

```sh
# Desktop (default): opens a Tauri window and detaches
mdview README.md

# Terminal pager with sixel diagrams
mdview --terminal README.md

# Pipe-friendly (no pager)
mdview --terminal --no-pager README.md | less -R

# Watch a file for changes in either mode
mdview --watch README.md
mdview --terminal --watch README.md
```

Inside Neovim:

```vim
:MdView                   " start live preview
:MdViewStop               " close the preview window
:MdViewRebuildThemeCache  " rebuild cached theme after colorscheme changes
```

Install the plugin with your package manager of choice. With
[lazy.nvim](https://github.com/folke/lazy.nvim):

```lua
{
  "aloknigam247/mdview",
  build = "cargo install --locked --path apps/mdview",
  cmd = { "MdView", "MdViewStop", "MdViewRebuildThemeCache" },
  ft = { "markdown" },
  opts = { debounce_ms = 100 },
}
```

## Gallery

All of the following are rendered from a single file —
[`fixtures/showcase.md`](./fixtures/showcase.md) — which you can open with
`mdview fixtures/showcase.md` to see them live.

### GFM core

| | |
|---|---|
| **Inline styles** — bold, italic, strikethrough, inline code, links, autolinks, `<sub>` / `<sup>` | ![inline styles](docs/screenshots/inline-styles.png) |
| **Lists** — bullet, ordered, deeply nested, task-list checkboxes | ![lists](docs/screenshots/lists.png) |
| **Tables** — GFM with left / center / right alignment and rounded corners | ![tables](docs/screenshots/tables.png) |
| **Blockquote** — accent bar + rounded container | ![blockquote](docs/screenshots/blockquote.png) |

### Extensions

| | |
|---|---|
| **Syntax highlight** — via `syntect`, server-rendered into inline-styled spans | ![code highlight](docs/screenshots/code-highlight.png) |
| **Math** — `$…$` inline, `$$…$$` display, ` ```math ` fenced blocks — rendered by KaTeX | ![math](docs/screenshots/math.png) |
| **Mermaid** — flowcharts, sequence, class, state diagrams | ![mermaid](docs/screenshots/mermaid.png) |
| **Draw.io** — embedded `<mxfile>` XML in a fenced block | ![drawio](docs/screenshots/drawio.png) |
| **Plotly** — JSON chart specs in a fenced block, interactive | ![plotly](docs/screenshots/plotly.png) |

### Layout

No fixed page width — mdview grows horizontally for long lines / wide content:

![wide content](docs/screenshots/wide-content.png)

## Theme customisation

### Built-in presets

`mdview` ships four presets: `dark`, `dracula`, `light`, `solarized`. Select
one on the command line:

```sh
mdview --theme dracula README.md
mdview --terminal --theme solarized README.md
```

Inside Tauri, use the theme picker in the bottom-right corner of the window to
cycle presets at runtime — the choice is persisted per-user.

### Neovim colorscheme sync

When invoked from Neovim the bridge synthesises a theme from your current
highlight groups (`Normal`, `Function`, `Comment`, `String`,
`@markup.heading.*`, …) and caches the result at:

```
<data-dir>/mdview/theme-cache/<colorscheme>-<nvim-version>.json
```

On subsequent launches mdview reads the cache entry instead of re-querying
Neovim. If your colorscheme changes or you tweak highlight groups, force a
rebuild with `:MdViewRebuildThemeCache`.

### Adding a new preset

Built-in themes only — no JSON loader, no dotfile. Add a file under
`crates/mdview-theme/src/themes/<name>.rs`:

```rust
use crate::{Radii, StyleSpec, Theme, Typography};
use std::collections::BTreeMap;

pub fn mytheme() -> Theme {
    Theme {
        name: "mytheme",
        colors: BTreeMap::from([
            ("fg", "#e0def4"),
            ("bg", "#191724"),
            ("accent", "#c4a7e7"),
            ("muted", "#6e6a86"),
            ("code.bg", "#1f1d2e"),
            ("link", "#9ccfd8"),
        ]),
        styles: BTreeMap::from([
            ("heading.1", StyleSpec::bold("#eb6f92")),
            ("heading.2", StyleSpec::bold("#f6c177")),
            ("blockquote", StyleSpec::italic("#908caa")),
        ]),
        radii: Radii { sm: 6, md: 10, lg: 16 },
        typography: Typography::default_modern(),
    }
}
```

Register it alphabetically in `crates/mdview-theme/src/presets.rs`:

```rust
pub fn presets() -> Vec<Theme> {
    vec![
        themes::dark::dark(),
        themes::dracula::dracula(),
        themes::light::light(),
        themes::mytheme::mytheme(),
        themes::solarized::solarized(),
    ]
}
```

Rebuild and select with `mdview --theme mytheme`.

## Extension authoring

Every markdown extension is a Rust crate that implements the
`MdViewExtension` trait and is registered in the core's `builtins.rs`. There
is exactly **one channel** — the compile-in built-in — so authoring a new
extension means adding a new crate to the workspace.

### Step 1 — scaffold a crate

```sh
cargo new --lib crates/mdview-ext-callout
```

Add it to the workspace in `Cargo.toml`:

```toml
[workspace]
members = [
  # … existing crates, alphabetical …
  "crates/mdview-ext-callout",
]
```

### Step 2 — implement the trait

```rust
// crates/mdview-ext-callout/src/lib.rs
use mdview_core::{Asset, AstNode, Html, MdViewExtension, RenderCtx, TermChunks};

pub struct Callout;

impl MdViewExtension for Callout {
    fn name(&self) -> &'static str { "callout" }

    fn register_parser(&self, opts: &mut comrak::ComrakOptions) {
        opts.extension.block_directives = true;
    }

    fn pre_parse(&self, src: &mut String) {
        // optionally rewrite a bespoke syntax into a comrak-friendly form
    }

    fn transform(&self, _node: &mut AstNode) {}

    fn render_html(&self, n: &AstNode, ctx: &RenderCtx) -> Option<Html> {
        // return Some(Html(...)) for nodes your extension owns
        None
    }

    fn render_terminal(&self, _n: &AstNode, _ctx: &RenderCtx) -> Option<TermChunks> {
        None
    }

    fn client_assets(&self) -> &'static [Asset] { &[] }
}
```

### Step 3 — register it

In `crates/mdview-core/src/builtins.rs`, alphabetical order:

```rust
pub fn builtin_extensions() -> Vec<Box<dyn MdViewExtension>> {
    vec![
        Box::new(mdview_ext_callout::Callout),
        Box::new(mdview_ext_drawio::Drawio),
        Box::new(mdview_ext_highlight::Highlight),
        Box::new(mdview_ext_math::Math),
        Box::new(mdview_ext_mermaid::Mermaid),
        Box::new(mdview_ext_plotly::Plotly),
    ]
}
```

### Step 4 — add a fixture and test

Drop a `fixtures/callout.md` file showcasing the extension and add it to
`tests/e2e/tests/fixtures.spec.ts`. `cargo test --workspace` + `npx playwright
test` will cover your new surface.

### What extensions may and may not do

- **May**: toggle comrak parser flags, walk the AST in `transform`, emit
  HTML/ANSI for nodes they own, declare static client-side JS/CSS assets
  (bundled into the webview at build time).
- **May not**: depend on other extension crates, load user-supplied code,
  register themselves from outside the workspace. Built-ins only.

## Architecture

See [CLAUDE.md](./CLAUDE.md) for the full architecture reference (crate
matrix, interaction sequences, trait + theme contracts, extensibility model,
and non-goals).

## License

MIT © mdview contributors. See [LICENSE](./LICENSE).
