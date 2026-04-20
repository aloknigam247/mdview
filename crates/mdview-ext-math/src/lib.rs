#![forbid(unsafe_code)]

//! Math extension for `mdview`.
//!
//! - HTML: emits placeholder spans/divs carrying the raw TeX in `data-tex`; a
//!   bundled KaTeX script renders them client-side.
//! - Terminal: uses `latex2mathml` and converts the resulting MathML tree to a
//!   Unicode expression.

mod mathml_unicode;

use comrak::nodes::NodeValue;

pub use comrak::nodes::AstNode;
pub use mdview_core::{
    Asset, Html, MdViewExtension, RenderCtx, StyleSpec, TermChunk, TermChunks, Theme,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct Math;

const ASSETS: &[Asset] = &[
    Asset {
        path: "vendor/katex.min.css",
        mime: "text/css",
    },
    Asset {
        path: "vendor/katex.min.js",
        mime: "application/javascript",
    },
    Asset {
        path: "vendor/mdv-math-init.js",
        mime: "application/javascript",
    },
];

impl MdViewExtension for Math {
    fn name(&self) -> &'static str {
        "math"
    }

    fn register_parser(&self, opts: &mut comrak::ComrakOptions) {
        opts.extension.math_code = true;
        opts.extension.math_dollars = true;
    }

    fn render_html<'a>(&self, node: &'a AstNode<'a>, _ctx: &RenderCtx<'_>) -> Option<Html> {
        let data = node.data.borrow();
        let (tex, display) = match &data.value {
            NodeValue::Math(m) => (m.literal.clone(), m.display_math),
            NodeValue::CodeBlock(cb) if is_math_info(&cb.info) => (cb.literal.clone(), true),
            _ => return None,
        };
        let attr = escape_html(&tex, true);
        let body = escape_html(&tex, false);
        Some(Html(if display {
            format!("<div class=\"mdv-math-block\" data-tex=\"{attr}\">\\[{body}\\]</div>")
        } else {
            format!("<span class=\"mdv-math\" data-tex=\"{attr}\">\\({body}\\)</span>")
        }))
    }

    fn render_terminal<'a>(
        &self,
        node: &'a AstNode<'a>,
        _ctx: &RenderCtx<'_>,
    ) -> Option<TermChunks> {
        let data = node.data.borrow();
        let tex = match &data.value {
            NodeValue::Math(m) => m.literal.clone(),
            NodeValue::CodeBlock(cb) if is_math_info(&cb.info) => cb.literal.clone(),
            _ => return None,
        };
        let text = tex_to_unicode(&tex)
            .filter(|u| !u.trim().is_empty())
            .unwrap_or_else(|| fallback_box(&tex));
        Some(vec![TermChunk::plain(text)])
    }

    fn client_assets(&self) -> &'static [Asset] {
        ASSETS
    }
}

fn is_math_info(info: &str) -> bool {
    info.split_whitespace()
        .next()
        .map(|tok| tok.eq_ignore_ascii_case("math"))
        .unwrap_or(false)
}

fn tex_to_unicode(tex: &str) -> Option<String> {
    let mathml = latex2mathml::latex_to_mathml(tex, latex2mathml::DisplayStyle::Inline).ok()?;
    mathml_unicode::mathml_to_unicode(&mathml)
}

fn fallback_box(tex: &str) -> String {
    let trimmed = tex.trim();
    format!("╭ {} ╮", trimmed)
}

fn escape_html(s: &str, in_attr: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if in_attr => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use comrak::{parse_document, Arena, ComrakOptions};

    fn find_math_nodes<'a>(root: &'a AstNode<'a>, out: &mut Vec<&'a AstNode<'a>>) {
        if matches!(root.data.borrow().value, NodeValue::Math(_)) {
            out.push(root);
        }
        for child in root.children() {
            find_math_nodes(child, out);
        }
    }

    fn render_opts() -> ComrakOptions<'static> {
        let mut opts = ComrakOptions::default();
        Math.register_parser(&mut opts);
        opts
    }

    #[test]
    fn inline_dollar_math_renders_html_and_terminal() {
        let arena = Arena::new();
        let opts = render_opts();
        let root = parse_document(&arena, "inline $a^2+b^2=c^2$ math", &opts);

        let mut math = Vec::new();
        find_math_nodes(root, &mut math);
        assert_eq!(math.len(), 1, "expected one math node");

        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        let ext = Math;

        let html = ext.render_html(math[0], &ctx).expect("html");
        assert!(html.0.contains("data-tex="), "html: {}", html.0);
        assert!(html.0.contains("a^2+b^2=c^2"), "html: {}", html.0);
        assert!(html.0.starts_with("<span"), "html: {}", html.0);

        let term = ext.render_terminal(math[0], &ctx).expect("term");
        let text = term[0].text.clone();
        assert!(
            text.contains('\u{00B2}'),
            "expected superscript 2 in terminal output, got {text:?}"
        );
    }

    #[test]
    fn display_dollar_math_renders_block() {
        let arena = Arena::new();
        let opts = render_opts();
        let root = parse_document(&arena, "$$\\int_0^1 x dx$$", &opts);
        let mut math = Vec::new();
        find_math_nodes(root, &mut math);
        assert!(!math.is_empty(), "expected display math node");

        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        let ext = Math;

        let html = ext.render_html(math[0], &ctx).expect("html");
        assert!(
            html.0.starts_with("<div class=\"mdv-math-block\""),
            "html: {}",
            html.0
        );
        assert!(html.0.contains("\\int_0^1"), "html: {}", html.0);

        let term = ext.render_terminal(math[0], &ctx).expect("term");
        assert!(
            !term[0].text.is_empty(),
            "terminal output should be non-empty"
        );
    }

    #[test]
    fn client_assets_contains_katex_and_init() {
        let ext = Math;
        let assets = ext.client_assets();
        let paths: Vec<&str> = assets.iter().map(|a| a.path).collect();
        assert!(paths.contains(&"vendor/katex.min.js"));
        assert!(paths.contains(&"vendor/katex.min.css"));
        assert!(paths.contains(&"vendor/mdv-math-init.js"));
    }

    #[test]
    fn register_parser_enables_math_flags() {
        let mut opts = ComrakOptions::default();
        Math.register_parser(&mut opts);
        assert!(opts.extension.math_dollars);
        assert!(opts.extension.math_code);
    }
}
