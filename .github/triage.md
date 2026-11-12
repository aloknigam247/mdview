# mdview: repository triage policy

## Categories

Use applicable categories from `bug`, `chore`, `docs`, `feature`, `performance`, `refactor`, `spike`, `tech-debt`,
and `test`, usually one primary category with a secondary only when needed.
For findings-only work, `spike` is normally the only category; never combine it with `bug`, `feature`, or `refactor`.

## Investigation

Use `AGENTS.md` Architecture reality and Where to extend. Identify the affected crates and output surfaces:
GUI, terminal pager, or Neovim preview. Verify which renderer actually serves that surface.

Reference Plugin contract, Theme contract, and Non-goals when applicable; do not copy their definitions or substitute
aspirational architecture for current source.

## Validation

Implementation leaves include relevant in-crate/integration test targets, affected existing tests, and precise new
behavior cases. Use `AGENTS.md` Commands and Conventions for build, formatting, lint, and crate-isolation requirements.

GUI changes require Playwright DOM visibility and, for visual deltas, computed-style assertions plus the existing
full-page screenshot check. String-only HTML assertions are insufficient. Keep terminal and Neovim validation
surface-specific; a browser test does not exercise native host persistence or terminal rendering.
