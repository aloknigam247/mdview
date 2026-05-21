use std::path::{Path, PathBuf};

use comrak::nodes::{AstNode, NodeValue};

use crate::htmlesc::{escape_html, escape_url};

pub enum ImgResolution {
    DataUri(String),
    Local(PathBuf),
    Remote(String),
    UnresolvableRelative,
}

pub fn resolve_image_url(url: &str, source_dir: Option<&Path>) -> ImgResolution {
    let trimmed = url.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return ImgResolution::Remote(trimmed.to_string());
    }
    if lower.starts_with("data:") {
        return ImgResolution::DataUri(trimmed.to_string());
    }
    let path = Path::new(trimmed);
    if path.is_absolute() {
        return ImgResolution::Local(path.to_path_buf());
    }
    match source_dir {
        Some(dir) => ImgResolution::Local(dir.join(path)),
        None => ImgResolution::UnresolvableRelative,
    }
}

pub fn collect_alt_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut out = String::new();
    for child in node.children() {
        push_alt(child, &mut out);
    }
    out
}

fn push_alt<'a>(node: &'a AstNode<'a>, out: &mut String) {
    match &node.data.borrow().value {
        NodeValue::Text(t) => out.push_str(t),
        NodeValue::Code(c) => out.push_str(&c.literal),
        NodeValue::LineBreak | NodeValue::SoftBreak => out.push(' '),
        _ => {
            for child in node.children() {
                push_alt(child, out);
            }
        }
    }
}

pub fn render_image_html(url: &str, alt: &str, title: &str) -> String {
    let url_attr = escape_url(url);
    let alt_attr = escape_html(alt);
    if title.is_empty() {
        format!("<img src=\"{url_attr}\" alt=\"{alt_attr}\" loading=\"lazy\">")
    } else {
        let title_attr = escape_html(title);
        let cap = escape_html(title);
        format!(
            "<figure><img src=\"{url_attr}\" alt=\"{alt_attr}\" title=\"{title_attr}\" loading=\"lazy\"><figcaption>{cap}</figcaption></figure>"
        )
    }
}

pub fn render_placeholder_html(alt: &str, fallback_name: Option<&str>) -> String {
    let label = if !alt.is_empty() {
        alt.to_string()
    } else {
        fallback_name.unwrap_or("image").to_string()
    };
    let label = escape_html(&label);
    format!("<span class=\"mdv-img-missing\">[image: {label}]</span>")
}

pub fn local_to_mdview_url(abs: &Path) -> String {
    let s = abs.to_string_lossy();
    let encoded = urlencoding::encode(&s);
    format!("mdview://localhost/{encoded}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_url_passes_through() {
        match resolve_image_url("https://example.com/x.png", None) {
            ImgResolution::Remote(u) => assert_eq!(u, "https://example.com/x.png"),
            _ => panic!("expected remote"),
        }
    }

    #[test]
    fn data_uri_passes_through() {
        match resolve_image_url("data:image/png;base64,AAA", None) {
            ImgResolution::DataUri(u) => assert!(u.starts_with("data:image/png")),
            _ => panic!("expected data uri"),
        }
    }

    #[test]
    fn relative_without_source_dir_is_unresolvable() {
        match resolve_image_url("./x.png", None) {
            ImgResolution::UnresolvableRelative => {}
            _ => panic!("expected unresolvable"),
        }
    }

    #[test]
    fn relative_with_source_dir_resolves_local() {
        let dir = std::path::PathBuf::from("/tmp/docs");
        match resolve_image_url("img/x.png", Some(&dir)) {
            ImgResolution::Local(p) => {
                assert!(p.ends_with("img/x.png") || p.ends_with("img\\x.png"))
            }
            _ => panic!("expected local"),
        }
    }
}
