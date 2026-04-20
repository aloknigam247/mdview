//! Mermaid diagram extension for mdview.
#![forbid(unsafe_code)]

mod _stubs;
pub mod scan;
pub mod sidecar;

pub use _stubs::{
    Asset, AstNode, Html, MdViewExtension, RenderCtx, SixelRenderer, StyleSpec, TermChunk,
    TermChunks, Theme,
};

use comrak::nodes::NodeValue;
use serde::Serialize;

const CLIENT_ASSETS: &[Asset] = &[
    Asset {
        path: "vendor/mdv-mermaid-init.js",
        mime: "application/javascript",
    },
    Asset {
        path: "vendor/mermaid.min.js",
        mime: "application/javascript",
    },
];

pub struct Mermaid;

#[derive(Debug, Serialize)]
struct SidecarJob<'a> {
    kind: &'a str,
    source: &'a str,
}

impl Mermaid {
    fn extract_source(node: &AstNode) -> Option<String> {
        match &node.data.borrow().value {
            NodeValue::CodeBlock(cb) if is_mermaid_info(&cb.info) => Some(cb.literal.clone()),
            _ => None,
        }
    }

    /// Render a mermaid diagram via the sidecar and a sixel renderer.
    ///
    /// Exposed for integration where the real `mdview-sixel` crate supplies
    /// a `SixelRenderer` implementation.
    pub fn render_terminal_with<'a, R: SixelRenderer>(
        &self,
        node: &'a AstNode<'a>,
        _ctx: &RenderCtx<'_>,
        renderer: &R,
    ) -> Option<TermChunks> {
        let source = Self::extract_source(node)?;
        Some(render_terminal_impl(&source, renderer))
    }
}

impl MdViewExtension for Mermaid {
    fn name(&self) -> &'static str {
        "mermaid"
    }

    fn render_html<'a>(&self, node: &'a AstNode<'a>, _ctx: &RenderCtx<'_>) -> Option<Html> {
        let source = Self::extract_source(node)?;
        Some(Html(format!(
            "<div class=\"mermaid\">{}</div>",
            escape_html(&source)
        )))
    }

    fn render_terminal<'a>(
        &self,
        node: &'a AstNode<'a>,
        _ctx: &RenderCtx<'_>,
    ) -> Option<TermChunks> {
        let source = Self::extract_source(node)?;
        Some(ascii_placeholder(&source))
    }

    fn client_assets(&self) -> &'static [Asset] {
        CLIENT_ASSETS
    }
}

fn is_mermaid_info(info: &str) -> bool {
    info.split_whitespace()
        .next()
        .map(|tok| tok.eq_ignore_ascii_case("mermaid"))
        .unwrap_or(false)
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn render_terminal_impl<R: SixelRenderer>(source: &str, renderer: &R) -> TermChunks {
    let Some(bin) = sidecar::locate_sidecar() else {
        return ascii_placeholder(source);
    };
    let job = SidecarJob {
        kind: "mermaid",
        source,
    };
    let payload = match serde_json::to_vec(&job) {
        Ok(p) => p,
        Err(_) => return ascii_placeholder(source),
    };
    let svg = match sidecar::run_sidecar(&bin, &payload) {
        Ok(bytes) if !bytes.is_empty() => bytes,
        _ => return ascii_placeholder(source),
    };
    match renderer.render_image(&svg) {
        Ok(bytes) => vec![TermChunk::plain(
            String::from_utf8_lossy(&bytes).into_owned(),
        )],
        Err(_) => ascii_placeholder(source),
    }
}

fn ascii_placeholder(source: &str) -> TermChunks {
    const HEADER: &str = "mermaid";
    let first_line = source
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim();
    let hint = "(install sidecar to see rendered diagram)";
    let inner_width = [
        first_line.chars().count(),
        hint.chars().count(),
        HEADER.len() + 6,
    ]
    .into_iter()
    .max()
    .unwrap_or(HEADER.len() + 6);

    let top = {
        let mut s = String::from("╭──[ ");
        s.push_str(HEADER);
        s.push_str(" ]");
        let pad = inner_width.saturating_sub(HEADER.len() + 6);
        for _ in 0..pad {
            s.push('─');
        }
        s.push('╮');
        s
    };
    let bottom = {
        let mut s = String::from("╰");
        for _ in 0..(inner_width + 2) {
            s.push('─');
        }
        s.push('╯');
        s
    };
    let content = format_line(first_line, inner_width);
    let hint_line = format_line(hint, inner_width);

    let text = format!("{top}\n{content}\n{hint_line}\n{bottom}\n");
    vec![TermChunk::plain(text)]
}

fn format_line(content: &str, inner_width: usize) -> String {
    let chars = content.chars().count();
    let pad = inner_width.saturating_sub(chars);
    let mut s = String::from("│ ");
    s.push_str(content);
    for _ in 0..pad {
        s.push(' ');
    }
    s.push_str(" │");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use comrak::{parse_document, Arena, ComrakOptions};

    fn first_code_block<'a>(root: &'a AstNode<'a>) -> Option<&'a AstNode<'a>> {
        root.descendants()
            .find(|n| matches!(n.data.borrow().value, NodeValue::CodeBlock(_)))
    }

    fn joined_text(chunks: &TermChunks) -> String {
        chunks
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn html_emits_mermaid_div_with_source() {
        let arena = Arena::new();
        let root = parse_document(
            &arena,
            "```mermaid\ngraph TD; A-->B;\n```\n",
            &ComrakOptions::default(),
        );
        let node = first_code_block(root).expect("code block");
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        let html = Mermaid.render_html(node, &ctx).unwrap();
        assert!(html.0.contains("class=\"mermaid\""));
        assert!(html.0.contains("graph TD; A--&gt;B;"));
    }

    #[test]
    fn html_ignores_non_mermaid_fences() {
        let arena = Arena::new();
        let root = parse_document(
            &arena,
            "```rust\nfn main(){}\n```\n",
            &ComrakOptions::default(),
        );
        let node = first_code_block(root).expect("code block");
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        assert!(Mermaid.render_html(node, &ctx).is_none());
    }

    #[test]
    fn html_accepts_info_with_trailing_classes() {
        let arena = Arena::new();
        let root = parse_document(
            &arena,
            "```mermaid theme=default\ngraph LR; X-->Y;\n```\n",
            &ComrakOptions::default(),
        );
        let node = first_code_block(root).expect("code block");
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        assert!(Mermaid.render_html(node, &ctx).is_some());
    }

    #[test]
    fn terminal_without_sidecar_returns_ascii_placeholder() {
        let prev_env = std::env::var_os(sidecar::SIDECAR_ENV);
        std::env::remove_var(sidecar::SIDECAR_ENV);
        let prev_path = std::env::var_os("PATH");
        std::env::set_var("PATH", "");

        let arena = Arena::new();
        let root = parse_document(
            &arena,
            "```mermaid\ngraph TD; A-->B;\n```\n",
            &ComrakOptions::default(),
        );
        let node = first_code_block(root).expect("code block");
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        let chunks = Mermaid.render_terminal(node, &ctx).expect("chunks");
        let text = joined_text(&chunks);
        assert!(text.starts_with("╭"));
        assert!(text.contains("mermaid"));
        assert!(text.contains("(install sidecar to see rendered diagram)"));
        assert!(text.contains("╰"));

        if let Some(p) = prev_path {
            std::env::set_var("PATH", p);
        }
        if let Some(v) = prev_env {
            std::env::set_var(sidecar::SIDECAR_ENV, v);
        }
    }

    struct EchoRenderer;
    impl SixelRenderer for EchoRenderer {
        fn render_image(&self, svg: &[u8]) -> Result<Vec<u8>, String> {
            let mut v = b"SIXEL:".to_vec();
            v.extend_from_slice(svg);
            Ok(v)
        }
    }

    #[test]
    #[cfg(unix)]
    fn terminal_with_fake_sidecar_returns_image_chunk() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("mdview-sidecar");
        std::fs::write(&script, "#!/bin/sh\nread LINE\nprintf '<svg>ok</svg>'\n").unwrap();
        let mut p = std::fs::metadata(&script).unwrap().permissions();
        p.set_mode(0o755);
        std::fs::set_permissions(&script, p).unwrap();
        std::env::set_var(sidecar::SIDECAR_ENV, &script);

        let arena = Arena::new();
        let root = parse_document(
            &arena,
            "```mermaid\ngraph TD; A-->B;\n```\n",
            &ComrakOptions::default(),
        );
        let node = first_code_block(root).expect("code block");
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        let chunks = Mermaid
            .render_terminal_with(node, &ctx, &EchoRenderer)
            .unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "SIXEL:<svg>ok</svg>");
        std::env::remove_var(sidecar::SIDECAR_ENV);
    }

    #[test]
    #[cfg(windows)]
    fn terminal_with_fake_sidecar_returns_image_chunk() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("mdview-sidecar.bat");
        std::fs::write(
            &script,
            "@echo off\r\nset /p _=\r\n<nul set /p =<svg>ok</svg>\r\n",
        )
        .unwrap();
        std::env::set_var(sidecar::SIDECAR_ENV, &script);

        let arena = Arena::new();
        let root = parse_document(
            &arena,
            "```mermaid\ngraph TD; A-->B;\n```\n",
            &ComrakOptions::default(),
        );
        let node = first_code_block(root).expect("code block");
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        let result = Mermaid.render_terminal_with(node, &ctx, &EchoRenderer);
        let chunks = result.expect("chunks");
        let text = joined_text(&chunks);
        let has_sixel = text.contains("SIXEL:");
        let has_placeholder = text.contains("mermaid");
        assert!(has_sixel || has_placeholder);
        std::env::remove_var(sidecar::SIDECAR_ENV);
    }

    #[test]
    fn client_assets_declares_mermaid_bundle() {
        let assets = Mermaid.client_assets();
        assert_eq!(assets.len(), 2);
        let paths: Vec<&str> = assets.iter().map(|a| a.path).collect();
        assert!(paths.contains(&"vendor/mermaid.min.js"));
        assert!(paths.contains(&"vendor/mdv-mermaid-init.js"));
        for a in assets {
            assert_eq!(a.mime, "application/javascript");
        }
    }
}
