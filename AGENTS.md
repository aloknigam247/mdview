# AGENTS.md — mdview

## Code review

Complete the [code review](.github/agents/code-review.md) before reporting changes ready, opening a PR, or merging it.
Run it from the current session on the worktree containing the change; never change the session's directory or start
another session for it. For a PR, review its head against its target branch.
Stop and report if the review covered no changed files, covered the wrong diff, failed, was incomplete, or was
unavailable. The implementer fixes accepted findings within scope; ask the user before skipping one or expanding scope.
Rerun after any change to the diff, including review fixes. Proceed only when a complete review covers the final diff
and every accepted finding is fixed or explicitly waived by the user.

## Project

`mdview` is a Rust Markdown viewer with desktop, terminal, and Neovim entrypoints.
Keep the visual style rounded: CSS border-radius in HTML and curved box-drawing in terminal tables and code blocks.

## Architecture reality

Read the owning guide before editing; verify the entrypoint for the affected output surface.
No Neovim renderer is wired: `run_nvim` in [pipeline.rs](apps/mdview/src/pipeline.rs) binds the socket and drops its
events.

| Area | Navigation |
|------|------------|
| `apps/mdview/` | [App guide](apps/mdview/AGENTS.md): CLI, native host, page generation, servers |
| `crates/` | [Crate guide](crates/AGENTS.md): shared contracts, extension registration, renderer boundaries |
| `lua/mdview/` | [Plugin entrypoint](lua/mdview/init.lua) |
| `sidecar/` | [Sidecar guide](sidecar/AGENTS.md): TypeScript diagram renderers and process protocol |
| `tests/` | [Test guide](tests/AGENTS.md): browser suites and harness ownership |

## Crate index

Workspace membership and dependency versions live in [Cargo.toml](Cargo.toml) and each package manifest.

| Crate | Entry point or scoped guide |
|-------|-----------------------------|
| `mdview-config` | [Configuration guide](crates/mdview-config/AGENTS.md) |
| `mdview-core` | [Parser, registry, extension trait, shared types](crates/mdview-core/src/lib.rs) |
| `mdview-ext-d2` | [Local D2 CLI integration](crates/mdview-ext-d2/src/lib.rs) |
| `mdview-ext-drawio` | [Draw.io blocks](crates/mdview-ext-drawio/src/lib.rs) |
| `mdview-ext-frontmatter` | [Frontmatter parsing and cards](crates/mdview-ext-frontmatter/src/lib.rs) |
| `mdview-ext-highlight` | [Syntax highlighting guide](crates/mdview-ext-highlight/AGENTS.md) |
| `mdview-ext-math` | [TeX blocks and inline math](crates/mdview-ext-math/src/lib.rs) |
| `mdview-ext-mermaid` | [Mermaid blocks](crates/mdview-ext-mermaid/src/lib.rs) |
| `mdview-ext-plotly` | [Plotly blocks](crates/mdview-ext-plotly/src/lib.rs) |
| `mdview-nvim` | [Neovim transport and theme events](crates/mdview-nvim/src/lib.rs) |
| `mdview-pager` | [Terminal interaction and search](crates/mdview-pager/src/lib.rs) |
| `mdview-render-html` | [Library HTML renderer](crates/mdview-render-html/src/lib.rs) |
| `mdview-render-terminal` | [Library terminal renderer](crates/mdview-render-terminal/src/lib.rs) |
| `mdview-server` | [Library HTTP/WebSocket server](crates/mdview-server/src/lib.rs) |
| `mdview-sixel` | [Image conversion and terminal graphics](crates/mdview-sixel/src/lib.rs) |
| `mdview-theme` | [Theme guide](crates/mdview-theme/AGENTS.md) |

## Where to extend

- CLI flags and host behavior: [app entrypoint map](apps/mdview/AGENTS.md#entrypoints).
- Extensions and output surfaces: [crate extension points](crates/AGENTS.md#extension-points).
- Languages, settings, and themes: use the corresponding scoped guide in the crate index.

### Plugin contract

Preserve the [extension contract](crates/AGENTS.md#plugin-contract); read its source signatures.

### Theme contract

Preserve the [theme contract](crates/mdview-theme/AGENTS.md#theme-contract).

## Commands

Run from the workspace root. The complete quality gate is [ci.yml](.github/workflows/ci.yml).

```ps1
cargo build --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
cargo test --workspace --locked --no-fail-fast
```

After implementing a feature and passing its tests, always run `cargo build --release --locked -p mdview`.
App launch, Bun build, and browser commands live in their scoped guides.

## Conventions

- [Code style](CONTRIBUTING.md#code-style) covers ordering and comments.
- Commit scopes: [.commitlint-scopes.json](.commitlint-scopes.json).
- Dependency reuse and crate-isolation requirements: [crate guide](crates/AGENTS.md#dependencies-and-validation).
- Formatting and lint checks above must pass.
- Rust edition and toolchain: [Cargo.toml](Cargo.toml), [rust-toolchain.toml](rust-toolchain.toml).
  Keep `#![deny(unsafe_code)]` or stricter; scope and justify any necessary exception.
- Triage and surface-specific acceptance requirements: [.github/triage.md](.github/triage.md).

## Working with parallel agents

- Compare stale-base branches with `git diff main...<branch>` to inspect changes since the merge-base.
- Integrate worktree changes deliberately and verify they landed in main before treating integration as complete.
  Check the actual diff and feature identifiers; [verify-features.ps1](scripts/verify-features.ps1) lists sentinels.
- Isolate parallel edits to shared hot files, especially app `render.rs` and `pipeline.rs`, in separate worktrees.
- Patch shared files narrowly; full-file replacement requires a fresh full read and preservation of unrelated content.
- Review unrelated formatter or autofix changes before merging; agent self-reports are not proof of the final tree.
- Use explicit checkout paths for Git operations; confirm the destination branch before integrating.

## Non-goals

- Dynamic, user-file, or npm extensions: built-ins only.
- Full HTML/CSS fidelity beyond GFM and shipped extensions.
- In-app file menus, Ctrl+O, and drag-drop opening: use the CLI.
- OS file-association handlers.
- User JSON theme loaders: built-in presets and Neovim synthesis only.
- WYSIWYG or collaborative editing.
