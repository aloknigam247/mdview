#![forbid(unsafe_code)]

mod _stubs;

use comrak::nodes::{NodeCodeBlock, NodeValue};
use once_cell::sync::Lazy;
use std::fmt::Write as _;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme as SyntectTheme, ThemeSet};
use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

pub use crate::_stubs::{
    Asset, AstNode, Html, MdViewExtension, RenderCtx, StyleSpec, TermChunk, TermChunks, Theme,
};

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

#[derive(Debug, thiserror::Error)]
pub enum HighlightError {
    #[error("syntect error: {0}")]
    Syntect(String),
}

const SPAN_OPEN: &str = "<span class=\"";

pub struct Highlight;

impl Highlight {
    fn theme_for(theme_name: &str) -> &'static SyntectTheme {
        let key = match theme_name.to_ascii_lowercase().as_str() {
            "dark" => "base16-ocean.dark",
            "dracula" => "base16-mocha.dark",
            "solarized" => "Solarized (dark)",
            _ => "InspiredGitHub",
        };
        THEME_SET
            .themes
            .get(key)
            .or_else(|| THEME_SET.themes.get("InspiredGitHub"))
            .expect("bundled syntect theme missing")
    }

    fn syntax_for(lang: &str) -> &'static SyntaxReference {
        let ss: &'static SyntaxSet = &SYNTAX_SET;
        if lang.is_empty() {
            return ss.find_syntax_plain_text();
        }
        ss.find_syntax_by_token(lang)
            .or_else(|| ss.find_syntax_by_name(lang))
            .or_else(|| ss.find_syntax_by_extension(lang))
            .unwrap_or_else(|| ss.find_syntax_plain_text())
    }

    fn code_info(node: &AstNode) -> Option<(String, String)> {
        match &node.data.borrow().value {
            NodeValue::CodeBlock(NodeCodeBlock { info, literal, .. }) => {
                let lang = info.split_whitespace().next().unwrap_or("").to_string();
                Some((lang, literal.clone()))
            }
            _ => None,
        }
    }
}

impl MdViewExtension for Highlight {
    fn name(&self) -> &'static str {
        "highlight"
    }

    fn render_html(&self, node: &AstNode, _ctx: &RenderCtx) -> Option<Html> {
        let (lang, source) = Self::code_info(node)?;
        let syntax = Self::syntax_for(&lang);
        let mut gen =
            ClassedHTMLGenerator::new_with_class_style(syntax, &SYNTAX_SET, ClassStyle::Spaced);
        let mut inner = String::new();
        for line in LinesWithEndings::from(&source) {
            match gen.parse_html_for_line_which_includes_newline(line) {
                Ok(()) => {}
                Err(_) => {
                    inner.push_str(&html_escape(line));
                }
            }
        }
        inner.push_str(&gen.finalize());

        let inline = classed_to_inline(&inner);
        let lang_attr = if lang.is_empty() {
            String::new()
        } else {
            format!(" data-lang=\"{}\"", html_escape(&lang))
        };
        Some(Html(format!(
            "<pre class=\"mdv-code\"{lang_attr}><code>{inline}</code></pre>"
        )))
    }

    fn render_terminal(&self, node: &AstNode, ctx: &RenderCtx) -> Option<TermChunks> {
        let (lang, source) = Self::code_info(node)?;
        let syntax = Self::syntax_for(&lang);
        let theme = Self::theme_for(ctx.theme.name);
        let mut h = HighlightLines::new(syntax, theme);
        let mut out = String::new();
        for line in LinesWithEndings::from(&source) {
            let ranges = h.highlight_line(line, &SYNTAX_SET).ok()?;
            out.push_str(&as_24_bit_terminal_escaped(&ranges[..], false));
        }
        out.push_str("\x1b[0m");
        Some(TermChunks {
            chunks: vec![TermChunk {
                text: out,
                style: None,
                ansi: None,
            }],
        })
    }
}

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn classed_to_inline(classed: &str) -> String {
    let mut body = String::with_capacity(classed.len());
    let mut rest = classed;
    while let Some(open) = rest.find(SPAN_OPEN) {
        body.push_str(&rest[..open]);
        rest = &rest[open + SPAN_OPEN.len()..];
        let Some(end_quote) = rest.find('"') else {
            break;
        };
        let classes = &rest[..end_quote];
        rest = &rest[end_quote + 1..];
        let Some(gt) = rest.find('>') else { break };
        rest = &rest[gt + 1..];
        body.push_str("<span style=\"");
        append_style_for_classes(&mut body, classes);
        body.push_str("\">");
    }
    body.push_str(rest);
    body
}

fn append_style_for_classes(out: &mut String, classes: &str) {
    let mut color: Option<&str> = None;
    let mut weight: Option<&str> = None;
    let mut style: Option<&str> = None;
    for c in classes.split_ascii_whitespace() {
        match c {
            "comment" => {
                color = Some("#7f848e");
                style = Some("italic");
            }
            "constant" | "numeric" => color = Some("#d19a66"),
            "entity" | "function" | "variable" => color = Some("#61afef"),
            "keyword" | "storage" => {
                color = Some("#c678dd");
                weight = Some("bold");
            }
            "string" => color = Some("#98c379"),
            "type" | "support" => color = Some("#e5c07b"),
            _ => {}
        }
    }
    let start = out.len();
    if let Some(c) = color {
        let _ = write!(out, "color:{c};");
    }
    if let Some(w) = weight {
        let _ = write!(out, "font-weight:{w};");
    }
    if let Some(s) = style {
        let _ = write!(out, "font-style:{s};");
    }
    if out.len() == start {
        out.push_str("color:inherit;");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use comrak::{parse_document, Arena, ComrakOptions};

    fn first_code_block<'a>(root: &'a AstNode<'a>) -> Option<&'a AstNode<'a>> {
        root.children()
            .find(|child| matches!(child.data.borrow().value, NodeValue::CodeBlock(_)))
    }

    fn make_ctx(theme_name: &'static str) -> RenderCtx {
        RenderCtx {
            theme: Theme {
                name: theme_name,
                ..Theme::default()
            },
            truecolor: true,
        }
    }

    fn render_html_for(src: &str, theme_name: &'static str) -> String {
        let arena = Arena::new();
        let opts = ComrakOptions::default();
        let root = parse_document(&arena, src, &opts);
        let node = first_code_block(root).expect("has code block");
        Highlight
            .render_html(node, &make_ctx(theme_name))
            .expect("html")
            .0
    }

    fn render_term_for(src: &str, theme_name: &'static str) -> String {
        let arena = Arena::new();
        let opts = ComrakOptions::default();
        let root = parse_document(&arena, src, &opts);
        let node = first_code_block(root).expect("has code block");
        Highlight
            .render_terminal(node, &make_ctx(theme_name))
            .expect("terminal")
            .chunks
            .into_iter()
            .map(|c| c.text)
            .collect()
    }

    #[test]
    fn name_is_highlight() {
        assert_eq!(Highlight.name(), "highlight");
    }

    #[test]
    fn html_rust_has_span_style() {
        let html = render_html_for("```rust\nfn main() { println!(\"hi\"); }\n```\n", "light");
        assert!(
            html.contains("<pre class=\"mdv-code\""),
            "pre wrapper: {html}"
        );
        assert!(html.contains("data-lang=\"rust\""));
        assert!(html.contains("<code>"));
        assert!(html.contains("<span style="), "expected span style: {html}");
    }

    #[test]
    fn html_python_has_span_style() {
        let html = render_html_for("```python\ndef f(x):\n    return x + 1\n```\n", "dark");
        assert!(html.contains("data-lang=\"python\""));
        assert!(html.contains("<span style="));
    }

    #[test]
    fn html_js_has_span_style() {
        let html = render_html_for(
            "```javascript\nconst a = 1;\nconsole.log(a);\n```\n",
            "dracula",
        );
        assert!(html.contains("data-lang=\"javascript\""));
        assert!(html.contains("<span style="));
    }

    #[test]
    fn terminal_contains_ansi_escape() {
        let ansi = render_term_for("```rust\nfn main() {}\n```\n", "dark");
        assert!(ansi.contains('\x1b'), "expected ESC in output");
        assert!(
            ansi.contains("[38;2;"),
            "expected 24-bit color seq: {ansi:?}"
        );
    }

    #[test]
    fn unknown_language_falls_through_to_plaintext() {
        let src = "```flibbertigibbet\njust words\nmore words\n```\n";
        let html = render_html_for(src, "light");
        assert!(html.contains("just words"));
        let ansi = render_term_for(src, "light");
        assert!(ansi.contains("just words"));
        assert!(ansi.ends_with("\x1b[0m"));
    }

    #[test]
    fn theme_mapping_resolves_all_known_names() {
        for name in ["light", "dark", "dracula", "solarized", "unknown-fallback"] {
            let _ = Highlight::theme_for(name);
        }
    }

    #[test]
    fn non_code_block_returns_none() {
        let arena = Arena::new();
        let opts = ComrakOptions::default();
        let root = parse_document(&arena, "just a paragraph\n", &opts);
        let p = root.first_child().expect("has child");
        assert!(Highlight.render_html(p, &make_ctx("light")).is_none());
        assert!(Highlight.render_terminal(p, &make_ctx("light")).is_none());
    }

    #[test]
    fn client_assets_empty() {
        assert!(Highlight.client_assets().is_empty());
    }
}
