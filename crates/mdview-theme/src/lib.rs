#![forbid(unsafe_code)]

pub mod _stubs;
pub mod ansi;
pub mod cache;
pub mod css;
pub mod nvim;
pub mod presets;
pub mod themes;

pub use _stubs::{Radii, StyleSpec, Theme, Typography};
pub use ansi::{style_for, style_for_depth, AnsiStyle, ColorDepth};
pub use cache::{cache_dir, cache_key, clear_all, load, store, CacheError};
pub use css::emit_css;
pub use nvim::{theme_from_nvim_highlights, NvimHl};
pub use presets::{builtin_themes, find};
