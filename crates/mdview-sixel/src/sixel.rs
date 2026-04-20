use crate::{Error, Result};
use color_quant::NeuQuant;
use icy_sixel::{sixel_string, DiffusionMethod, PixelFormat};
use image::GenericImageView;

/// Decode `png_bytes` and encode the image as a sixel escape sequence.
/// The output begins with the DCS introducer (`\x1bP…q`) and ends with the
/// string terminator (`\x1b\\`).
pub fn encode_png(png_bytes: &[u8]) -> Result<String> {
    let img = image::load_from_memory_with_format(png_bytes, image::ImageFormat::Png)
        .map_err(|e| Error::Image(e.to_string()))?;
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8();
    let rgba_bytes = rgba.as_raw();

    let nq = NeuQuant::new(10, 256, rgba_bytes);
    let palette = nq.color_map_rgb();

    let pixel_count = (w as usize) * (h as usize);
    let mut rgb = Vec::with_capacity(pixel_count * 3);
    for i in 0..pixel_count {
        let idx = nq.index_of(&rgba_bytes[i * 4..i * 4 + 4]);
        let base = idx * 3;
        rgb.push(palette[base]);
        rgb.push(palette[base + 1]);
        rgb.push(palette[base + 2]);
    }

    sixel_string(
        &rgb,
        w as i32,
        h as i32,
        PixelFormat::RGB888,
        DiffusionMethod::Stucki,
    )
    .map_err(|e| Error::Sixel(format!("{e:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn red_square_png(side: u32) -> Vec<u8> {
        let mut img = image::RgbaImage::new(side, side);
        for px in img.pixels_mut() {
            *px = image::Rgba([255, 0, 0, 255]);
        }
        let dyn_img = image::DynamicImage::ImageRgba8(img);
        let mut out = std::io::Cursor::new(Vec::new());
        dyn_img
            .write_to(&mut out, image::ImageFormat::Png)
            .expect("encode png");
        out.into_inner()
    }

    #[test]
    fn encodes_red_square_to_sixel() {
        let png = red_square_png(16);
        let s = encode_png(&png).expect("sixel encode");
        assert!(s.starts_with('\u{1b}'), "missing ESC prefix");
        assert!(s.starts_with("\u{1b}P"), "missing DCS introducer");
        assert!(s.ends_with("\u{1b}\\"), "missing ST terminator");
    }
}
