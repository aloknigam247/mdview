#![forbid(unsafe_code)]

mod _stubs;
mod sidecar;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use comrak::nodes::{AstNode, NodeValue};

pub use _stubs::{
    Asset, Html, MdViewExtension, RenderCtx, StyleSpec, TermChunk, TermChunks, Theme,
};

const CLIENT_ASSETS: &[Asset] = &[
    Asset {
        mime: "application/javascript",
        path: "vendor/drawio-viewer.js",
    },
    Asset {
        mime: "application/javascript",
        path: "vendor/mdv-drawio-init.js",
    },
];

pub struct Drawio;

impl MdViewExtension for Drawio {
    fn name(&self) -> &'static str {
        "drawio"
    }

    fn render_html<'a>(&self, node: &'a AstNode<'a>, _ctx: &RenderCtx<'_>) -> Option<Html> {
        let (info, body) = extract_fenced(node)?;
        if !is_drawio_info(&info) {
            return None;
        }
        let b64 = B64.encode(body.as_bytes());
        let options = parse_options(&info);
        let scale_attr = options
            .get("scale")
            .map(|v| format!(" data-scale=\"{}\"", escape_attr(v)))
            .unwrap_or_default();
        Some(Html(format!(
            "<div class=\"drawio-viewer\" data-xml-b64=\"{b64}\"{scale_attr}></div>"
        )))
    }

    fn render_terminal<'a>(
        &self,
        node: &'a AstNode<'a>,
        _ctx: &RenderCtx<'_>,
    ) -> Option<TermChunks> {
        let (info, body) = extract_fenced(node)?;
        if !is_drawio_info(&info) {
            return None;
        }
        match sidecar::run_sidecar("drawio", &body) {
            Ok(svg) => Some(vec![TermChunk::plain(sixel_wrap(&svg))]),
            Err(_) => Some(vec![TermChunk::plain(placeholder_ascii(&body))]),
        }
    }

    fn client_assets(&self) -> &'static [Asset] {
        CLIENT_ASSETS
    }
}

fn extract_fenced(node: &AstNode<'_>) -> Option<(String, String)> {
    match &node.data.borrow().value {
        NodeValue::CodeBlock(cb) => Some((cb.info.clone(), cb.literal.clone())),
        _ => None,
    }
}

fn is_drawio_info(info: &str) -> bool {
    let head = info.split(|c: char| c.is_whitespace()).next().unwrap_or("");
    let lang = head.split(':').next().unwrap_or("");
    lang.eq_ignore_ascii_case("drawio")
}

fn parse_options(info: &str) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    let head = info.split(|c: char| c.is_whitespace()).next().unwrap_or("");
    let mut parts = head.split(':');
    parts.next();
    for part in parts {
        if let Some((k, v)) = part.split_once('=') {
            out.insert(k.to_string(), v.to_string());
        }
    }
    out
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn sixel_wrap(svg: &str) -> String {
    format!("\x1bPq{svg}\x1b\\")
}

fn placeholder_ascii(body: &str) -> String {
    let lines = body.lines().count();
    format!(
        "╭─ drawio diagram ({lines} lines) ─╮\n│ (sidecar unavailable)        │\n╰──────────────────────────────╯"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use comrak::{parse_document, Arena, ComrakOptions};

    fn first_code_block<'a>(root: &'a AstNode<'a>) -> Option<&'a AstNode<'a>> {
        for node in root.descendants() {
            if matches!(node.data.borrow().value, NodeValue::CodeBlock(_)) {
                return Some(node);
            }
        }
        None
    }

    #[test]
    fn html_emits_div_with_base64() {
        let arena = Arena::new();
        let md = "```drawio\n<mxfile><hello/></mxfile>\n```\n";
        let root = parse_document(&arena, md, &ComrakOptions::default());
        let cb = first_code_block(root).expect("code block");
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        let html = Drawio.render_html(cb, &ctx).expect("html output");
        let body = "<mxfile><hello/></mxfile>\n";
        let expected = B64.encode(body.as_bytes());
        assert!(html.0.contains("drawio-viewer"));
        assert!(
            html.0.contains(&format!("data-xml-b64=\"{}\"", expected)),
            "got: {}",
            html.0
        );
    }

    #[test]
    fn html_emits_scale_option() {
        let arena = Arena::new();
        let md = "```drawio:scale=1.2\n<mxfile/>\n```\n";
        let root = parse_document(&arena, md, &ComrakOptions::default());
        let cb = first_code_block(root).expect("code block");
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        let html = Drawio.render_html(cb, &ctx).expect("html output");
        assert!(html.0.contains("data-scale=\"1.2\""), "got: {}", html.0);
    }

    #[test]
    fn ignores_non_drawio_fence() {
        let arena = Arena::new();
        let md = "```rust\nfn main() {}\n```\n";
        let root = parse_document(&arena, md, &ComrakOptions::default());
        let cb = first_code_block(root).expect("code block");
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        assert!(Drawio.render_html(cb, &ctx).is_none());
        assert!(Drawio.render_terminal(cb, &ctx).is_none());
    }

    #[test]
    fn terminal_placeholder_when_sidecar_missing() {
        let arena = Arena::new();
        let md = "```drawio\n<mxfile/>\n```\n";
        let root = parse_document(&arena, md, &ComrakOptions::default());
        let cb = first_code_block(root).expect("code block");
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        let chunks = Drawio.render_terminal(cb, &ctx).expect("chunks");
        assert_eq!(chunks.len(), 1);
        let text = &chunks[0].text;
        assert!(
            text.contains("drawio diagram") || text.starts_with("\x1bPq"),
            "got: {}",
            text
        );
    }

    #[test]
    fn client_assets_declares_viewer() {
        let assets = Drawio.client_assets();
        assert_eq!(assets.len(), 2);
        assert!(assets.iter().any(|a| a.path == "vendor/drawio-viewer.js"));
        assert!(assets.iter().any(|a| a.path == "vendor/mdv-drawio-init.js"));
    }
}
