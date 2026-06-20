//! D2 diagram extension for mdview.
//!
//! Renders fenced ```d2 code blocks by shelling out to the local `d2` CLI.
//! No remote API. No wasm. If `d2` is not on PATH at render time we emit an
//! inline error block with installation guidance in place of the diagram.
#![forbid(unsafe_code)]

mod _stubs;
pub mod cli;
pub mod scan;

pub use _stubs::{
    Asset, AstNode, Html, MdViewExtension, RenderCtx, SixelRenderer, StyleSpec, TermChunk,
    TermChunks, Theme,
};

use comrak::nodes::NodeValue;

pub struct D2;

impl D2 {
    fn extract_source(node: &AstNode) -> Option<String> {
        match &node.data.borrow().value {
            NodeValue::CodeBlock(cb) if is_d2_info(&cb.info) => Some(cb.literal.clone()),
            _ => None,
        }
    }

    /// Render a d2 diagram via the local CLI to inline SVG plus a sixel
    /// representation for terminals. Exposed so callers can supply a real
    /// `SixelRenderer` (e.g. from `mdview-sixel`).
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

impl MdViewExtension for D2 {
    fn name(&self) -> &'static str {
        "d2"
    }

    fn render_html<'a>(&self, node: &'a AstNode<'a>, _ctx: &RenderCtx<'_>) -> Option<Html> {
        let source = Self::extract_source(node)?;
        Some(Html(render_html_impl(&source)))
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
        &[]
    }
}

fn is_d2_info(info: &str) -> bool {
    info.split_whitespace()
        .next()
        .map(|tok| tok.eq_ignore_ascii_case("d2"))
        .unwrap_or(false)
}

fn render_html_impl(source: &str) -> String {
    match cli::render_svg(source) {
        Ok(svg) => {
            let svg_str = String::from_utf8_lossy(&svg);
            format!("<div class=\"d2\">{}</div>", inline_svg(&svg_str))
        }
        Err(cli::D2Error::NotFound) => format_error_html(cli::INSTALL_HINT, source),
        Err(e) => format_error_html(&format!("d2 render error: {e}"), source),
    }
}

fn inline_svg(svg: &str) -> String {
    // `d2 --omit-xml-tag` already strips the XML declaration; pass through as-is.
    svg.trim().to_string()
}

fn format_error_html(message: &str, source: &str) -> String {
    let mut s = String::from("<div class=\"d2 d2-error\">");
    s.push_str("<p class=\"d2-error-message\">");
    s.push_str(&escape_html(message));
    s.push_str("</p>");
    s.push_str("<pre class=\"d2-source\"><code>");
    s.push_str(&escape_html(source));
    s.push_str("</code></pre>");
    s.push_str("</div>");
    s
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
    match cli::render_svg(source) {
        Ok(svg) => match renderer.render_image(&svg) {
            Ok(bytes) => vec![TermChunk::plain(
                String::from_utf8_lossy(&bytes).into_owned(),
            )],
            Err(_) => ascii_placeholder(source),
        },
        Err(cli::D2Error::NotFound) => error_placeholder(cli::INSTALL_HINT, source),
        Err(_) => ascii_placeholder(source),
    }
}

fn ascii_placeholder(source: &str) -> TermChunks {
    boxed_message("d2", "(install d2 CLI to see rendered diagram)", source)
}

fn error_placeholder(message: &str, source: &str) -> TermChunks {
    boxed_message("d2", message, source)
}

fn boxed_message(header: &str, hint: &str, source: &str) -> TermChunks {
    let first_line = source
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim();
    let inner_width = [
        first_line.chars().count(),
        hint.chars().count(),
        header.len() + 6,
    ]
    .into_iter()
    .max()
    .unwrap_or(header.len() + 6);

    let top = {
        let mut s = String::from("╭──[ ");
        s.push_str(header);
        s.push_str(" ]");
        let pad = inner_width.saturating_sub(header.len() + 6);
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

    /// Force `locate_d2` to fail by clearing PATH and the override env var.
    /// Returns a guard that restores the previous values on drop.
    struct PathGuard {
        prev_env: Option<std::ffi::OsString>,
        prev_path: Option<std::ffi::OsString>,
    }

    impl PathGuard {
        fn isolate() -> Self {
            let prev_env = std::env::var_os(cli::D2_ENV);
            let prev_path = std::env::var_os("PATH");
            std::env::remove_var(cli::D2_ENV);
            std::env::set_var("PATH", "");
            Self {
                prev_env,
                prev_path,
            }
        }
    }

    impl Drop for PathGuard {
        fn drop(&mut self) {
            if let Some(p) = self.prev_path.take() {
                std::env::set_var("PATH", p);
            } else {
                std::env::remove_var("PATH");
            }
            if let Some(v) = self.prev_env.take() {
                std::env::set_var(cli::D2_ENV, v);
            }
        }
    }

    #[test]
    fn parser_detects_d2_fenced_block() {
        let arena = Arena::new();
        let root = parse_document(&arena, "```d2\na -> b\n```\n", &ComrakOptions::default());
        let node = first_code_block(root).expect("code block");
        // The extension's source extractor recognizes the fence.
        let src = D2::extract_source(node).expect("d2 source extracted");
        assert!(src.contains("a -> b"));
    }

    #[test]
    fn parser_ignores_non_d2_fences() {
        let arena = Arena::new();
        let root = parse_document(
            &arena,
            "```rust\nfn main(){}\n```\n",
            &ComrakOptions::default(),
        );
        let node = first_code_block(root).expect("code block");
        assert!(D2::extract_source(node).is_none());
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        assert!(D2.render_html(node, &ctx).is_none());
    }

    #[test]
    fn parser_accepts_info_with_trailing_words() {
        let arena = Arena::new();
        let root = parse_document(
            &arena,
            "```d2 theme=default\nx -> y\n```\n",
            &ComrakOptions::default(),
        );
        let node = first_code_block(root).expect("code block");
        assert!(D2::extract_source(node).is_some());
    }

    #[test]
    fn html_without_cli_emits_install_hint() {
        let _guard = PathGuard::isolate();
        let arena = Arena::new();
        let root = parse_document(&arena, "```d2\na -> b\n```\n", &ComrakOptions::default());
        let node = first_code_block(root).expect("code block");
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        let html = D2.render_html(node, &ctx).expect("html");
        assert!(html.0.contains("d2 CLI not found on PATH"));
        assert!(html.0.contains("https://d2lang.com/tour/install"));
        assert!(html.0.contains("d2-error"));
        // Original source should be preserved in a <pre> for the user.
        assert!(html.0.contains("a -&gt; b"));
    }

    #[test]
    fn terminal_without_cli_emits_install_hint() {
        let _guard = PathGuard::isolate();
        let arena = Arena::new();
        let root = parse_document(&arena, "```d2\na -> b\n```\n", &ComrakOptions::default());
        let node = first_code_block(root).expect("code block");
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        // `render_terminal` is the default path used by registrants that
        // don't have a `SixelRenderer` wired up — it should still be the
        // ASCII fallback.
        let chunks = D2.render_terminal(node, &ctx).expect("chunks");
        let text = joined_text(&chunks);
        assert!(text.contains("d2"));
        assert!(text.starts_with("╭"));
    }

    #[test]
    fn render_terminal_with_without_cli_emits_install_hint() {
        let _guard = PathGuard::isolate();
        struct NeverRenderer;
        impl SixelRenderer for NeverRenderer {
            fn render_image(&self, _svg: &[u8]) -> Result<Vec<u8>, String> {
                panic!("must not be invoked when CLI missing");
            }
        }
        let arena = Arena::new();
        let root = parse_document(&arena, "```d2\na -> b\n```\n", &ComrakOptions::default());
        let node = first_code_block(root).expect("code block");
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        let chunks = D2
            .render_terminal_with(node, &ctx, &NeverRenderer)
            .expect("chunks");
        let text = joined_text(&chunks);
        assert!(text.contains("d2 CLI not found on PATH"));
        assert!(text.contains("https://d2lang.com/tour/install"));
    }

    #[test]
    fn client_assets_is_empty() {
        let assets = D2.client_assets();
        assert!(assets.is_empty());
    }

    #[test]
    fn extension_name_is_d2() {
        assert_eq!(D2.name(), "d2");
    }

    // Real-render test: only runs when the `d2-cli-tests` feature is enabled
    // AND the local environment has `d2` on PATH. This keeps CI green on
    // machines without `d2` installed.
    #[cfg(feature = "d2-cli-tests")]
    #[test]
    fn real_render_produces_svg() {
        // Bail out if `d2 --version` doesn't work — we don't want to fail CI.
        let ok = std::process::Command::new("d2")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !ok {
            eprintln!("skipping: d2 CLI not available");
            return;
        }
        let svg = cli::render_svg("a -> b\n").expect("d2 svg");
        let text = String::from_utf8_lossy(&svg);
        assert!(text.contains("<svg"));
    }
}
