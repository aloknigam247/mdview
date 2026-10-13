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
   directory. If you need a type from a sibling, depend on that sibling crate
   and import the real type rather than duplicating it locally.
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

## Release automation

### Versioning

mdview has one product version: `workspace.package.version` in root
`Cargo.toml`. All 17 Cargo packages inherit that version and `publish = false`;
none is published to a crate registry. Do not hand-bump member versions.
The bot keeps `version.txt`, local-package versions in `Cargo.lock`, and
`.release-please-manifest.json` synchronized with the root Cargo version.
`version.txt` is a mirror, not a second Cargo version authority.

`release-please-config.json` uses the `simple` strategy for root package `"."`,
with built-in TOML extra-file updates for the Cargo version and lockfile.
The native Rust strategy does not support this virtual workspace's inherited
versions. Root scope covers all repository code surfaces, including Rust,
Lua, and TypeScript. Each product release has one root `CHANGELOG.md` entry,
one `vX.Y.Z` tag, and one published (not draft) GitHub Release.

Bootstrap starts with `initial-version: "0.1.0"`, an empty `{}` release
manifest, and a blank `CHANGELOG.md`. Installing the workflow does not tag or
publish anything. The first bot PR supplies the real historical `0.1.0`
changelog, its heading, and the manifest's `"."` version entry.

After bootstrap, conventional `feat:` commits bump minor and `fix:` commits
bump patch. Breaking changes (`!` or a `BREAKING CHANGE:` footer) bump minor
while on `0.x` (`bump-minor-pre-major: true`), then major on `1.x` and later.
`bump-patch-for-minor-pre-major: false` keeps pre-1.0 features as minor bumps.
Non-breaking `docs:` or `chore:` commits alone do not trigger a release.

### Daily release PR

`.github/workflows/release-please.yml` runs only in `aloknigam247/mdview` on
`main`, daily at 00:00 IST (`30 18 * * *` UTC) or by manual dispatch. It does
not run on pushes or pull requests, and does not install or run tests.
GitHub schedules can be delayed; this is not an exact-time guarantee.
Release jobs are serialized without canceling an in-progress run.

Each daily/manual run publishes any unreleased, merged bot release PR and
creates or refreshes one pending release PR for releasable commits. Review its
version, changelog, and metadata changes. Leave it open to accumulate
changes, and merge only when ready to ship. The next daily run publishes the
merged release; an ordinary PR or an unmerged release PR cannot publish.
For earlier publication or recovery, dispatch on `main`:

```sh
gh workflow run release-please.yml --repo aloknigam247/mdview --ref main
```

### Release credentials

A repository owner must add `RELEASE_PLEASE_TOKEN` under **Settings > Secrets
and variables > Actions**. Use a fine-grained PAT scoped only to
`aloknigam247/mdview`, with **Contents**, **Issues** (for labels), and **Pull
requests** read/write, following the
[official action guidance](https://github.com/googleapis/release-please-action/blob/5c625bfb5d1ff62eadeeb3772007f7f66fdcf071/README.md#workflow-permissions).
Choose an expiry, track its owner and expiry, rotate before expiration, and
revoke replaced tokens. Never put the token in chat, a token file, or source
control. The workflow does not provision credentials or change repository
permissions.

Only the release job receives this PAT; the default workflow token has no
permissions. A missing secret fails explicitly before the action runs, with
no `GITHUB_TOKEN` fallback.
An expired, revoked, or under-scoped PAT fails the action; replace the secret
and rerun on `main`. The default `GITHUB_TOKEN` suppresses downstream workflow
events, so a PAT is needed for downstream workflows and the `release.published`
contract for [#47](https://github.com/aloknigam247/mdview/issues/47).
Binary packaging and winget publishing belong to #47, not this workflow;
there are no registry-publish commands or registry tokens here.

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
