use crate::{Error, Result};
use image::GenericImageView;

/// Decode an arbitrary raster image (png/jpg/jpeg/gif/webp) from `bytes`,
/// rescale to `max_width_px` preserving aspect ratio (no upscale beyond
/// source width), and re-encode as PNG.
pub fn raster_to_png(bytes: &[u8], max_width_px: u32) -> Result<Vec<u8>> {
    let img = image::load_from_memory(bytes).map_err(|e| Error::Image(e.to_string()))?;
    let (src_w, src_h) = img.dimensions();
    if src_w == 0 || src_h == 0 {
        return Err(Error::Image("zero-sized image".into()));
    }
    let target = max_width_px.max(1);
    let scaled = if src_w <= target {
        img
    } else {
        let new_h = ((src_h as u64 * target as u64) / src_w as u64).max(1) as u32;
        img.resize_exact(target, new_h, image::imageops::FilterType::Lanczos3)
    };
    let mut out = std::io::Cursor::new(Vec::new());
    scaled
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| Error::Render(e.to_string()))?;
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blue_png(w: u32, h: u32) -> Vec<u8> {
        let mut img = image::RgbaImage::new(w, h);
        for px in img.pixels_mut() {
            *px = image::Rgba([0, 0, 255, 255]);
        }
        let dyn_img = image::DynamicImage::ImageRgba8(img);
        let mut out = std::io::Cursor::new(Vec::new());
        dyn_img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn raster_to_png_resizes_when_wider() {
        let src = blue_png(200, 100);
        let png = raster_to_png(&src, 80).expect("re-encode");
        let w = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
        let h = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
        assert_eq!(w, 80);
        assert_eq!(h, 40);
    }

    #[test]
    fn raster_to_png_preserves_when_smaller() {
        let src = blue_png(50, 30);
        let png = raster_to_png(&src, 800).expect("re-encode");
        let w = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
        assert_eq!(w, 50);
    }
}
