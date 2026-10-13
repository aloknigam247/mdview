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
The bot keeps local-package versions in `Cargo.lock` synchronized with the
root Cargo version. No separate version file or release manifest is needed.

`.github/release/release.config.cjs` configures semantic-release for `main`.
It calculates the next version from Git tags and conventional commits across
the repository, including Rust, Lua, and TypeScript. With no existing version
tag, the first automatic version is `1.0.0`. The checked-in `0.1.0` baseline
changes when that first versioning run commits its updates.

Standard SemVer applies: `feat:` bumps minor; `fix:` and `perf:` bump patch;
breaking changes (`!` or a `BREAKING CHANGE:` footer) bump major. For example,
from `1.2.3`, those changes produce `1.3.0`, `1.2.4`, and `2.0.0` respectively.
Qualifying revert commits bump patch and appear under "Reverted".
Non-breaking `docs:`, `chore:`, and `refactor:` commits alone do not trigger
version updates or changelog entries.

The standard plugins generate `CHANGELOG.md`, commit version files directly
to `main`, and create `vX.Y.Z` Git tags. The npm and GitHub publishing plugins
are explicitly excluded: no release PR, GitHub Release, registry upload, or
binary publication is created.

`scripts/set_version.py` only writes the version supplied by semantic-release;
it does not calculate bumps, merge PRs, or manage tags. It preserves member
inheritance and third-party lockfile entries, and fails if the workspace or
lockfile layout would make the update ambiguous.

### Daily version updates

`.github/workflows/semantic-release.yml` runs only in `aloknigam247/mdview` on
`main`, daily at 00:00 IST (`30 18 * * *` UTC) or by manual dispatch. It does
not run on pushes or pull requests, and does not install or run tests.
GitHub schedules can be delayed; this is not an exact-time guarantee.
Version jobs are serialized without canceling an in-progress run.

The workflow installs the locked Node versioning tools from `.github/release`;
these are CI tooling, not application or test dependencies. Cargo file updates
use Python's standard library, without a Rust toolchain installation or build.
Only `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md` are included
in generated `chore(version): ... [skip ci]` commits.
If there are no qualifying commits since the last version tag, no version
commit or tag is created. Do not delete or move version tags.
The tool's generic "Published release" log message refers to a completed
version/tag cycle here, not a GitHub Release or registry publication.

For an earlier version update, dispatch on `main`:

```sh
gh workflow run semantic-release.yml --repo aloknigam247/mdview --ref main
```

### Release credentials

A repository owner must add `RELEASE_PLEASE_TOKEN` under **Settings > Secrets
and variables > Actions**. Use a fine-grained PAT scoped only to
`aloknigam247/mdview`, with **Contents** read/write. The existing secret name is
retained even though release-please is no longer used. No Issues or Pull requests
permission is needed by this version-only workflow.
Choose an expiry, track its owner and expiry, rotate before expiration, and
revoke replaced tokens. Never put the token in chat, a token file, or source
control. The workflow does not provision credentials or change repository
permissions.

Only the version job receives this PAT; the default workflow token has read-only
Contents permission for checkout, and checkout does not persist credentials.
A missing secret fails explicitly before versioning runs, with
no `GITHUB_TOKEN` fallback.
An expired, revoked, or under-scoped PAT fails the workflow; replace the secret
and rerun on `main`. The token's identity must be permitted to push directly to
`main`. The repository's PR-only ruleset grants administrators that exception;
force-push, deletion, and linear-history protections remain separate and enforced.
Normal non-force pushes are used; no branch protections are changed by the job.

This workflow does not emit `release.published`. Binary packaging, GitHub Releases,
and winget publishing remain outside it. There are no registry-publish commands
or registry tokens.

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
