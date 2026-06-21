#![forbid(unsafe_code)]

mod _stubs;
mod sidecar;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use comrak::nodes::{AstNode, NodeValue};

pub use _stubs::{
    Asset, Html, MdViewExtension, RenderCtx, StyleSpec, TermChunk, TermChunks, Theme,
};

/// The literal prefix line shown when the `mdview-sidecar` binary is missing
/// or otherwise unavailable. Surfaced in terminal output with ANSI red SGR.
pub const MISSING_CLI_PREFIX: &str = "missing drawio command";

const ANSI_RED: &str = "\x1b[31m";
const ANSI_RESET: &str = "\x1b[0m";

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
        Some(render_terminal_impl(&body, None))
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

/// Render the drawio body to terminal chunks. Tests pass `Some(p)` to inject
/// a fake or non-existent sidecar binary; production callers pass `None`.
fn render_terminal_impl(body: &str, override_path: Option<&std::path::Path>) -> TermChunks {
    match sidecar::run_sidecar("drawio", body, override_path) {
        Ok(svg) => vec![TermChunk::plain(sixel_wrap(&svg))],
        Err(sidecar::SidecarError::NotFound) => missing_cli_terminal(body),
        Err(_) => fallback_code_terminal(body),
    }
}

/// Render the block as a normal code block with a single ANSI-red prefix line
/// reading `missing drawio command`. Used when the sidecar binary cannot be
/// located.
fn missing_cli_terminal(source: &str) -> TermChunks {
    let mut text = String::new();
    text.push_str(ANSI_RED);
    text.push_str(MISSING_CLI_PREFIX);
    text.push_str(ANSI_RESET);
    text.push('\n');
    text.push_str(source);
    if !source.ends_with('\n') {
        text.push('\n');
    }
    vec![TermChunk::plain(text)]
}

/// Plain code-block fallback used when the sidecar is present but fails to
/// render. Surfaces the user's source verbatim without claiming the CLI is
/// missing.
fn fallback_code_terminal(source: &str) -> TermChunks {
    let mut text = source.to_string();
    if !text.ends_with('\n') {
        text.push('\n');
    }
    vec![TermChunk::plain(text)]
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

    /// A path that is guaranteed not to exist on the host. Passing this to
    /// the override parameter forces `locate_sidecar` to return `NotFound` —
    /// indistinguishable from "no `mdview-sidecar` on PATH" in the production
    /// code path, without touching the process environment.
    fn missing_path() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push("mdview-drawio-tests");
        p.push("definitely-not-a-real-sidecar-binary");
        p
    }

    #[test]
    fn terminal_renders_code_with_ansi_red_prefix_when_sidecar_missing() {
        let body = "<mxfile/>\n";
        let missing = missing_path();
        let chunks = render_terminal_impl(body, Some(&missing));
        assert_eq!(chunks.len(), 1);
        let text = &chunks[0].text;
        assert!(
            text.starts_with("\x1b[31mmissing drawio command\x1b[0m\n"),
            "expected ANSI red prefix at start, got: {text:?}"
        );
        assert!(text.contains("<mxfile/>"), "source missing: {text:?}");
        assert!(
            !text.contains("drawio diagram") && !text.contains("sidecar unavailable"),
            "old placeholder leaked: {text:?}"
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
