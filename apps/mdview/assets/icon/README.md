# mdview icon

Source: `icon.svg` (256x256, rounded-square Dracula purple #BD93F9, black M glyph).

Build-time rasterized into multi-resolution PNGs and packed into `mdview.ico` for:
- Runtime window icon (loaded via `include_bytes!` in `apps/mdview/src/pipeline.rs`).
- Windows .exe embedded resource (via `winresource` in `build.rs`).

To regenerate after editing `icon.svg`, just `cargo build -p mdview` — the build script does the rest.