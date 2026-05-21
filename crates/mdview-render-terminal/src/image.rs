use std::path::{Path, PathBuf};

use comrak::nodes::{AstNode, NodeValue};

const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;
const RASTER_WIDTH_PX: u32 = 800;

pub enum ImgResolution {
    Local(PathBuf),
    Remote,
    Unresolvable,
}

pub fn resolve(url: &str, source_dir: Option<&Path>) -> ImgResolution {
    let trimmed = url.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("data:") {
        return ImgResolution::Remote;
    }
    let path = Path::new(trimmed);
    if path.is_absolute() {
        return ImgResolution::Local(path.to_path_buf());
    }
    match source_dir {
        Some(dir) => ImgResolution::Local(dir.join(path)),
        None => ImgResolution::Unresolvable,
    }
}

pub fn collect_alt_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut out = String::new();
    for child in node.children() {
        push(child, &mut out);
    }
    out
}

fn push<'a>(node: &'a AstNode<'a>, out: &mut String) {
    match &node.data.borrow().value {
        NodeValue::Text(t) => out.push_str(t),
        NodeValue::Code(c) => out.push_str(&c.literal),
        NodeValue::LineBreak | NodeValue::SoftBreak => out.push(' '),
        _ => {
            for child in node.children() {
                push(child, out);
            }
        }
    }
}

pub fn placeholder_label(alt: &str, url: &str, path: Option<&Path>) -> String {
    if !alt.is_empty() {
        return format!("[image: {alt}]");
    }
    let name = path
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| url.to_string());
    if name.is_empty() {
        "[image]".to_string()
    } else {
        format!("[image: {name}]")
    }
}

pub fn encode_local_to_sixel(path: &Path, sixel_supported: bool) -> Option<String> {
    if !sixel_supported {
        return None;
    }
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > MAX_IMAGE_BYTES {
        tracing::warn!(
            path = %path.display(),
            bytes = meta.len(),
            "mdview: image exceeds 10 MB cap; emitting placeholder"
        );
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    let png = if ext == "svg" {
        let svg = std::str::from_utf8(&bytes).ok()?;
        match mdview_sixel::svg_to_png(svg, RASTER_WIDTH_PX) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(error = %e, "mdview: svg rasterise failed");
                return None;
            }
        }
    } else {
        match mdview_sixel::raster_to_png(&bytes, RASTER_WIDTH_PX) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(error = %e, "mdview: raster decode failed");
                return None;
            }
        }
    };
    match mdview_sixel::encode_png(&png) {
        Ok(s) => Some(s),
        Err(e) => {
            tracing::warn!(error = %e, "mdview: sixel encode failed");
            None
        }
    }
}
