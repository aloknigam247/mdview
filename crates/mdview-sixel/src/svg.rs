use crate::{Error, Result};

/// Rasterise `svg` to a PNG byte vector, scaling uniformly so the rendered
/// width matches `width_px` and height preserves the source aspect ratio.
pub fn svg_to_png(svg: &str, width_px: u32) -> Result<Vec<u8>> {
    let width_px = width_px.max(1);
    let opts = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &opts).map_err(|e| Error::Svg(e.to_string()))?;

    let size = tree.size();
    let src_w = size.width();
    let src_h = size.height();
    if src_w <= 0.0 || src_h <= 0.0 {
        return Err(Error::Svg("svg has zero dimension".into()));
    }
    let scale = width_px as f32 / src_w;
    let out_w = width_px;
    let out_h = ((src_h * scale).round() as u32).max(1);

    let mut pixmap = tiny_skia::Pixmap::new(out_w, out_h)
        .ok_or_else(|| Error::Render("pixmap allocation failed".into()))?;
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    pixmap
        .encode_png()
        .map_err(|e| Error::Render(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_svg_produces_png_with_expected_width() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="5" viewBox="0 0 10 5"><rect width="10" height="5" fill="red"/></svg>"#;
        let png = svg_to_png(svg, 40).expect("rasterise");
        assert!(!png.is_empty());
        assert_eq!(&png[0..8], b"\x89PNG\r\n\x1a\n");
        let w = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
        let h = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
        assert_eq!(w, 40);
        assert_eq!(h, 20);
    }
}
