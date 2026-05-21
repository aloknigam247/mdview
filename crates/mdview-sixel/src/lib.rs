#![forbid(unsafe_code)]

//! Terminal capability probe plus SVG→PNG→sixel/kitty/ascii rendering utilities
//! for mdview. Terminal-oriented image pipeline: the caller feeds an SVG string
//! and terminal capabilities; [`render_image`] picks the best output format.

pub mod detect;
pub mod fallback;
pub mod raster;
pub mod sixel;
pub mod svg;

pub use detect::{probe, TerminalCaps};
pub use fallback::{ascii_placeholder, kitty_graphics};
pub use raster::raster_to_png;
pub use sixel::encode_png;
pub use svg::svg_to_png;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("svg parse error: {0}")]
    Svg(String),
    #[error("image decode error: {0}")]
    Image(String),
    #[error("sixel encode error: {0}")]
    Sixel(String),
    #[error("render error: {0}")]
    Render(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Pick the best available encoding (sixel > kitty > ascii) and render `svg`.
///
/// Width is chosen from `caps.width` (terminal cells). Height preserves the
/// SVG aspect ratio. When no graphics protocol is available the label is used
/// for an ascii placeholder.
pub fn render_image(svg: &str, caps: &TerminalCaps, label: &str) -> String {
    let width_cells = caps.width.max(10);
    let width_px = u32::from(width_cells) * 8;
    let png = match svg_to_png(svg, width_px) {
        Ok(p) => p,
        Err(_) => return ascii_placeholder(label, width_cells),
    };
    if caps.sixel {
        if let Ok(s) = encode_png(&png) {
            return s;
        }
    }
    if caps.kitty {
        return kitty_graphics(&png);
    }
    ascii_placeholder(label, width_cells)
}
