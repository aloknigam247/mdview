# Theme

## Theme contract

Do not break the shared theme contract in [mdview-core/src/types.rs](../mdview-core/src/types.rs).
[src/_stubs.rs](src/_stubs.rs) re-exports those types; do not introduce a second definition.

## Entrypoints

| File | Responsibility |
|------|----------------|
| [src/ansi.rs](src/ansi.rs) | Style lookup and terminal color-depth conversion |
| [src/cache.rs](src/cache.rs) | Cache directory/key derivation, load, store, and clearing |
| [src/css.rs](src/css.rs) | `emit_css`: theme variables and library page styles |
| [src/nvim.rs](src/nvim.rs) | Synthesize a theme from Neovim highlight groups |
| [src/presets.rs](src/presets.rs) | Built-in enumeration and name lookup |
| [src/themes/_build.rs](src/themes/_build.rs) | Palette-to-theme construction and style keys |
| [src/themes/mod.rs](src/themes/mod.rs) | Preset modules |
| [tests/integration.rs](tests/integration.rs) | Required keys, preset ordering, CSS, ANSI, and cache checks |

## Adding a preset

1. Use [catppuccin_mocha.rs](src/themes/catppuccin_mocha.rs) as the preset template and reuse the palette builder.
2. Declare the module in `src/themes/mod.rs` and register its getter in `src/presets.rs`.
3. Update the relevant preset expectations in `tests/integration.rs`; preserve its required-key checks.

The app extracts the `:root` variables from `emit_css` in
[render.rs::emit_theme_blocks](../../apps/mdview/src/render.rs).
Keep that boundary intact when changing CSS output; app selectors belong in the
[app page guide](../../apps/mdview/AGENTS.md#native-host-and-page).

Theme-slot selection and OS-mode resolution belong to
[mdview-config](../mdview-config/AGENTS.md), not the preset registry.
Run the package isolation check from the [crate guide](../AGENTS.md#dependencies-and-validation).
