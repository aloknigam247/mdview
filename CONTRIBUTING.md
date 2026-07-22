# Contributing to mdview

Thanks for taking the time to contribute. This guide covers the workflow,
expectations for patches, and the test matrix every change must pass.

## Workflow

1. **Open an issue first** for anything larger than a typo fix or a trivial
   bug. Describe the user-facing problem (or feature) and, when relevant,
   which surface it affects (Tauri desktop, terminal pager, Neovim bridge).
2. **Fork + branch** from `main`. Use short, descriptive branch names:
   `feat/callout-extension`, `fix/sixel-probe-timeout`.
3. **Stay within your crate.** Each crate under `crates/` owns a disjoint
   directory. If you need a type from a sibling that doesn't yet exist,
   add a minimal `src/_stubs.rs` with a
   `// TODO: replace with mdview_<sibling> after integration` marker rather
   than reaching across crate boundaries.
4. **Open a pull request** against `main`. Fill out the PR template; link
   the issue; attach screenshots for any HTML-surface change.

## Commit style

Use [conventional commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>
```

Common types:

| Type | Use for |
|---|---|
| `build` | Build-system or dependency changes |
| `chore` | Tooling, cleanup, no behaviour change |
| `ci` | CI pipeline changes |
| `docs` | Documentation-only changes |
| `feat` | New user-facing capability |
| `fix` | Bug fixes |
| `perf` | Performance improvements |
| `refactor` | Non-behavioural code restructuring |
| `style` | Formatting, whitespace, lint-only |
| `test` | Test additions or fixes |

Examples:

```
feat(ext-mermaid): render on idle via requestIdleCallback
fix(sixel): clamp PNG dimensions to terminal cell grid
refactor(core): simplify registry dispatch via BTreeMap
```

Keep commits focused; squash fixup commits before review.

## Code style

- **Rust 2021**, stable toolchain. `#![deny(unsafe_code)]` unless justified in
  a comment adjacent to the `unsafe` block.
- **Alphabetical ordering** for new items in enums, `use` groups, match arms,
  `BTreeMap` literals, and similar sequences where order is semantically
  neutral. Never reorder existing entries — keep diffs minimal.
- No comments unless the *why* is non-obvious.
- No emoji in code or docs unless a fixture explicitly contains them.
- No new dependencies without a justification in the PR description; prefer
  the incumbent crates (`comrak`, `syntect`, `latex2mathml`, `resvg`,
  `ratatui`, `crossterm`, `axum`, `tokio`, `tokio-tungstenite`, `rmpv`,
  `sixel-rs`, `clap`, `serde`, `anyhow`).

## Required checks

Run all of these locally before you push:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For HTML-producing crates (`mdview-render-html`, `mdview-server`,
`mdview-ext-*`, `apps/mdview`), also run the Playwright end-to-end suite:

```sh
# Unix / macOS / WSL
./tests/e2e/scripts/run.sh

# Windows (pwsh 7)
pwsh -NoProfile -File ./tests/e2e/scripts/run.ps1
```

The script builds the release binary, starts it with `--serve-only`, runs
`npx playwright test`, and tears everything down. Screenshots land in
`tests/e2e/artifacts/`.

For the Bun-compiled sidecar (`sidecar/`):

```sh
cd sidecar
bun test
bun build --compile ./src/index.ts --outfile ./mdview-sidecar
```

## Pull request checklist

- [ ] Conventional-commit messages.
- [ ] `cargo fmt --check` clean.
- [ ] `cargo clippy -- -D warnings` clean on the touched crates.
- [ ] `cargo test` passes on the touched crates (and `--workspace` if your
      change crosses crate boundaries).
- [ ] Playwright e2e passes locally (for HTML-surface changes).
- [ ] Screenshots attached for HTML-surface changes.
- [ ] Docs updated (`README.md`, `AGENTS.md`, crate-level docs) when
      public behaviour changes.
- [ ] No new direct dependencies between extension crates.
- [ ] Owned paths only — no collateral edits in other crates.

## Code of conduct

Be kind, be patient, assume good faith. Harassment, discrimination, and
personal attacks are not tolerated. Reports go to
[aloknigam@microsoft.com](mailto:aloknigam@microsoft.com); we follow the
[Contributor Covenant v2.1](https://www.contributor-covenant.org/version/2/1/code_of_conduct/).
