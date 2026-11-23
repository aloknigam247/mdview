# Syntax highlighting

[src/lib.rs](src/lib.rs) owns syntax loading, aliases, fenced/inline highlighting, and its unit tests.
[src/_stubs.rs](src/_stubs.rs) re-exports the core contract.

## Entrypoints

| Symbol in `src/lib.rs` | Read for |
|-----------------------|----------|
| `balanced_line_html`, `classed_to_tokens` | Balanced per-line HTML and semantic token classes |
| `canonical_lang` | Fence aliases mapped to syntect syntax names |
| `Highlight::render_html`, `Highlight::render_terminal` | Surface-specific code-block output |
| `parse_hl_lines` | Highlighted-line metadata |
| `render_inline_html_literal`, `render_inline_terminal_literal` | Inline hashbang syntax |
| `syntax_for`, `syntax_for_strict` | Fence fallback versus strict inline lookup |
| `SYNTAX_SET`, `add_vendored` | Bundled syntax construction and load failures |

## Adding language support

- Aliases: add an alphabetical `canonical_lang` arm for the lowercase hint. Verify the canonical name with
  `SyntaxSet::find_syntax_by_name` and add a unit test in the same file.
- Syntaxes absent from `load_defaults_newlines`: vendor the `.sublime-syntax` under `assets/`, attribute its source
  license under [LICENSES/](../../LICENSES/), and add it to the `SYNTAX_SET` builder.
  Follow the existing PowerShell entry, preserving upstream filenames.

## Integration boundaries

- HTML emits `mdv-tok-*` spans; keep token mapping aligned with styles in the
  [app page](../../apps/mdview/src/render.rs).
- Inline lookup is strict; unsupported hints must not consume ordinary inline-code text.
- Registration follows the [extension-order rule](../AGENTS.md#extension-points).

## Validation

Use the [crate isolation check](../AGENTS.md#dependencies-and-validation) and the existing unit cases for the changed
language, fallback, inline, or line-marking behavior. Snapshot/demo entrypoints live in [Cargo.toml](Cargo.toml);
expected outputs live in `__snapshots__/`.
Browser integration for [fixtures/code.md](../../fixtures/code.md) is in
[fixtures.spec.ts](../../tests/e2e/tests/fixtures.spec.ts); harness commands are in the
[test guide](../../tests/AGENTS.md).
