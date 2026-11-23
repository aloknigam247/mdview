# Configuration

## Entrypoints

| File | Responsibility |
|------|----------------|
| [src/action.rs](src/action.rs) | `Action`, `ALL`, and action-name parsing/formatting |
| [src/error.rs](src/error.rs) | Diagnostic sources, offending values, and file/line presentation |
| [src/keymap.rs](src/keymap.rs) | Binding parsing, normalization, matching, and lookup |
| [src/lib.rs](src/lib.rs) | Config types/defaults, TOML validation, path resolution, loaders, and template |
| [src/os_theme.rs](src/os_theme.rs) | OS light/dark preference |
| [tests/config.rs](tests/config.rs) | Config, binding, path, and validation cases |

## Behavioral boundaries

- Bindings are opt-in: preserve the empty `Keymap::defaults()`; actions absent from config have no binding.
- Configuration paths are resolved by `Config::config_path`. Missing files yield defaults without creating a file;
  `DEFAULT_CONFIG_TOML` is a template, not evidence of an automatic write.
- Diagnostics must retain the offending key/value, expectation, and available source location through `ConfigError`.
  `report` owns the `mdview-config-error:` stderr prefix parsed by `on_stderr` in
  [the Lua entrypoint](../../lua/mdview/init.lua).
- Loaders return recovered values and diagnostics in `LoadResult`; `Config::load` discards the returned diagnostics.
  The application separately rejects reported errors before UI startup or detachment in
  [main.rs::preflight_config](../../apps/mdview/src/main.rs).
  [App config-error tests](../../apps/mdview/tests/config_errors.rs) assert non-zero exits and stderr diagnostics
  with detachment disabled.
- Theme toggles do not write config. Follow the consumers in
  [app rendering](../../apps/mdview/src/render.rs) and [pager interaction](../mdview-pager/src/lib.rs).

## Changing configuration

Keep types, defaults, parsing/validation, and `DEFAULT_CONFIG_TOML` aligned in `src/lib.rs`.
Read the parser for accepted keys and bounds rather than copying a settings table; TOC heading selection uses `levels`.
`config.toml`'s `[keymap]` drives the GUI. For action changes, update `Action::ALL`, string conversions, and
`keymap_json` in [app rendering](../../apps/mdview/src/render.rs). Keep `KeyBinding::matches` and the GUI dispatcher
consistent with binding normalization. The pager has a separate [`keymap.toml`](../mdview-pager/src/keymap.rs).

Update the relevant configuration documentation and `tests/config.rs` for each config change.
For startup-policy changes, also cover the app integration target linked above.
Run the package isolation check from the [crate guide](../AGENTS.md#dependencies-and-validation).
