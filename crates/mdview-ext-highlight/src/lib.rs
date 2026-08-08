#![forbid(unsafe_code)]

mod _stubs;

use comrak::nodes::{NodeCodeBlock, NodeValue};
use once_cell::sync::Lazy;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme as SyntectTheme, ThemeSet};
use syntect::html::{line_tokens_to_classed_spans, ClassStyle};
use syntect::parsing::{
    syntax_definition::SyntaxDefinition, ParseState, ScopeStack, SyntaxReference, SyntaxSet,
    SyntaxSetBuilder,
};
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

pub use crate::_stubs::{
    Asset, AstNode, Html, MdViewExtension, RenderCtx, StyleSpec, TermChunk, TermChunks, Theme,
};

fn load_vendored(yaml: &str, name: &'static str) -> Result<SyntaxDefinition, String> {
    SyntaxDefinition::load_from_str(yaml, true, Some(name)).map_err(|e| e.to_string())
}

fn add_vendored(builder: &mut SyntaxSetBuilder, yaml: &str, name: &'static str, fence: &str) {
    match load_vendored(yaml, name) {
        Ok(syn) => builder.add(syn),
        Err(e) => eprintln!(
            "mdview-ext-highlight: failed to load {name} syntax: {e}; {fence} fences will render as plain text"
        ),
    }
}

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| {
    let mut builder = SyntaxSet::load_defaults_newlines().into_builder();
    add_vendored(
        &mut builder,
        include_str!("../assets/http-request-response.sublime-syntax"),
        "HTTP Request and Response",
        "http",
    );
    add_vendored(
        &mut builder,
        include_str!("../assets/PowerShell.sublime-syntax"),
        "PowerShell",
        "powershell",
    );
    builder.build()
});
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
        if let Some(name) = canonical_lang(lang) {
            if let Some(syn) = ss.find_syntax_by_name(name) {
                return syn;
            }
        }
        ss.find_syntax_by_token(lang)
            .or_else(|| ss.find_syntax_by_name(lang))
            .or_else(|| ss.find_syntax_by_extension(lang))
            .unwrap_or_else(|| ss.find_syntax_plain_text())
    }

    fn code_info(node: &AstNode) -> Option<(String, String, BTreeSet<usize>)> {
        match &node.data.borrow().value {
            NodeValue::CodeBlock(NodeCodeBlock { info, literal, .. }) => {
                let lang = info.split_whitespace().next().unwrap_or("").to_string();
                let highlighted = parse_hl_lines(info);
                Some((lang, literal.clone(), highlighted))
            }
            _ => None,
        }
    }
}

impl MdViewExtension for Highlight {
    fn name(&self) -> &'static str {
        "highlight"
    }

    fn render_html<'a>(&self, node: &'a AstNode<'a>, ctx: &RenderCtx<'_>) -> Option<Html> {
        let (lang, source, highlighted) = Self::code_info(node)?;
        let tab_spaces = " ".repeat(ctx.tab_width as usize);
        let source = source.replace('\t', &tab_spaces);
        let syntax = Self::syntax_for(&lang);
        let mut parse_state = ParseState::new(syntax);
        let mut scope_stack = ScopeStack::new();
        let mut inner = String::new();
        for (idx, line) in LinesWithEndings::from(&source).enumerate() {
            let line_no = idx + 1;
            let ops = parse_state
                .parse_line(line, &SYNTAX_SET)
                .unwrap_or_default();
            let (html, _delta) =
                line_tokens_to_classed_spans(line, &ops, ClassStyle::Spaced, &mut scope_stack)
                    .unwrap_or_else(|_| (html_escape(line), 0));
            let inline = balanced_line_html(&html);
            let class = if highlighted.contains(&line_no) {
                "hl-line hl-line--mark"
            } else {
                "hl-line"
            };
            let _ = write!(inner, "<span class=\"{class}\">{inline}</span>");
        }
        let lang_attr = if lang.is_empty() {
            String::new()
        } else {
            format!(" data-lang=\"{}\"", html_escape(&lang))
        };
        Some(Html(format!(
            "<pre class=\"mdv-code\"{lang_attr}><code>{inner}</code></pre>"
        )))
    }

    fn render_terminal<'a>(
        &self,
        node: &'a AstNode<'a>,
        ctx: &RenderCtx<'_>,
    ) -> Option<TermChunks> {
        let (lang, source, highlighted) = Self::code_info(node)?;
        let syntax = Self::syntax_for(&lang);
        let theme = Self::theme_for(ctx.theme.name.as_str());
        let mut h = HighlightLines::new(syntax, theme);
        let mut out = String::new();
        for (idx, line) in LinesWithEndings::from(&source).enumerate() {
            let line_no = idx + 1;
            let ranges = h.highlight_line(line, &SYNTAX_SET).ok()?;
            let ansi = as_24_bit_terminal_escaped(&ranges[..], false);
            if highlighted.contains(&line_no) {
                out.push_str("\x1b[7m\u{258e}");
                out.push_str(&ansi);
                out.push_str("\x1b[27m");
            } else {
                out.push_str(&ansi);
            }
        }
        out.push_str("\x1b[0m");
        Some(vec![TermChunk::plain(out)])
    }
}

fn canonical_lang(lang: &str) -> Option<&'static str> {
    match lang.to_ascii_lowercase().as_str() {
        "csharp" => Some("C#"),
        "http" => Some("HTTP Request and Response"),
        "jsonc" => Some("JSON"),
        "powershell" | "ps1" | "pwsh" => Some("PowerShell"),
        _ => None,
    }
}

pub fn parse_hl_lines(info: &str) -> BTreeSet<usize> {
    let mut set = BTreeSet::new();
    let Some(after) = info.find("hl_lines=\"") else {
        return set;
    };
    let rest = &info[after + "hl_lines=\"".len()..];
    let Some(close) = rest.find('"') else {
        return set;
    };
    let value = &rest[..close];
    for token in value.split_whitespace() {
        if let Some((a, b)) = token.split_once('-') {
            if let (Ok(n), Ok(m)) = (a.parse::<usize>(), b.parse::<usize>()) {
                if n >= 1 && m >= n {
                    for i in n..=m {
                        set.insert(i);
                    }
                }
            }
        } else if let Ok(n) = token.parse::<usize>() {
            if n >= 1 {
                set.insert(n);
            }
        }
    }
    set
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

fn classed_to_tokens(classed: &str) -> String {
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
        body.push_str("<span class=\"");
        class_for_classes(&mut body, classes);
        body.push_str("\">");
    }
    body.push_str(rest);
    body
}

/// Convert syntect classed HTML to mdv-tok spans, ensuring the result is
/// self-contained (no orphaned `</span>` from previous-line scope carryover).
fn balanced_line_html(classed: &str) -> String {
    let converted = classed_to_tokens(classed);
    // Strip leading </span> tags (scope closers carried from previous line)
    let mut s = converted.as_str();
    while s.starts_with("</span>") {
        s = &s[7..];
    }
    // Also handle cases where text appears between orphaned close tags
    // e.g. "</span>text</span><span...>" — the close after text is also orphaned
    let opens = s.matches("<span ").count() + s.matches("<span>").count();
    let closes = s.matches("</span>").count();
    if closes > opens {
        // There are still orphaned closes after stripping leading ones.
        // Remove them by rebuilding the string without excess closes.
        let excess = closes - opens;
        let mut result = String::with_capacity(s.len());
        let mut remaining = s;
        let mut removed = 0;
        while removed < excess {
            if let Some(pos) = remaining.find("</span>") {
                // Check if this close is before any open
                let next_open = remaining.find("<span ");
                if next_open.is_none_or(|o| pos < o) {
                    result.push_str(&remaining[..pos]);
                    remaining = &remaining[pos + 7..];
                    removed += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        result.push_str(remaining);
        result
    } else if opens > closes {
        let mut result = s.to_string();
        for _ in 0..(opens - closes) {
            result.push_str("</span>");
        }
        result
    } else {
        s.to_string()
    }
}

fn class_for_classes(out: &mut String, classes: &str) {
    let mut tok: Option<&str> = None;
    for c in classes.split_ascii_whitespace() {
        match c {
            "comment" => {
                tok = Some("mdv-tok-comment");
            }
            "constant" | "numeric" => tok = Some("mdv-tok-constant"),
            "entity" | "function" | "variable" => tok = Some("mdv-tok-function"),
            "keyword" | "storage" => {
                tok = Some("mdv-tok-keyword");
            }
            "string" => tok = Some("mdv-tok-string"),
            "support" | "type" => tok = Some("mdv-tok-type"),
            _ => {}
        }
    }
    if let Some(t) = tok {
        let _ = write!(out, "mdv-tok {t}");
    } else {
        out.push_str("mdv-tok");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use comrak::{parse_document, Arena, ComrakOptions};

    #[test]
    fn parse_hl_lines_basic_range() {
        let result = parse_hl_lines("python hl_lines=\"2 5-7\"");
        assert_eq!(result, BTreeSet::from([2, 5, 6, 7]));
    }

    #[test]
    fn parse_hl_lines_single_with_spaces() {
        let result = parse_hl_lines("python hl_lines=\"  3  \"");
        assert_eq!(result, BTreeSet::from([3]));
    }

    #[test]
    fn parse_hl_lines_empty_value() {
        let result = parse_hl_lines("python hl_lines=\"\"");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_hl_lines_malformed_token() {
        let result = parse_hl_lines("python hl_lines=\"abc\"");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_hl_lines_invalid_range_reversed() {
        let result = parse_hl_lines("python hl_lines=\"2-1\"");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_hl_lines_no_attr() {
        let result = parse_hl_lines("python");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_hl_lines_other_attrs_ignored() {
        let result = parse_hl_lines("python linenums=\"1\" hl_lines=\"3\" title=\"foo.py\"");
        assert_eq!(result, BTreeSet::from([3]));
    }

    fn first_code_block<'a>(root: &'a AstNode<'a>) -> Option<&'a AstNode<'a>> {
        root.children()
            .find(|child| matches!(child.data.borrow().value, NodeValue::CodeBlock(_)))
    }

    fn make_theme(theme_name: &'static str) -> Theme {
        Theme {
            name: theme_name.to_string(),
            ..Theme::default()
        }
    }

    fn render_html_for(src: &str, theme_name: &'static str) -> String {
        let arena = Arena::new();
        let opts = ComrakOptions::default();
        let root = parse_document(&arena, src, &opts);
        let node = first_code_block(root).expect("has code block");
        let theme = make_theme(theme_name);
        let ctx = RenderCtx::new(&theme);
        Highlight.render_html(node, &ctx).expect("html").0
    }

    fn render_term_for(src: &str, theme_name: &'static str) -> String {
        let arena = Arena::new();
        let opts = ComrakOptions::default();
        let root = parse_document(&arena, src, &opts);
        let node = first_code_block(root).expect("has code block");
        let theme = make_theme(theme_name);
        let ctx = RenderCtx::new(&theme);
        Highlight
            .render_terminal(node, &ctx)
            .expect("terminal")
            .into_iter()
            .map(|c| c.text)
            .collect()
    }

    #[test]
    fn tabs_preserved_in_terminal_output() {
        let term = render_term_for("```rust\n\tlet x = 1;\n```\n", "dark");
        assert!(
            term.contains('\t'),
            "expected tab character preserved in terminal output, got: {:?}",
            &term[..term.len().min(200)]
        );
    }

    const HTTP_REQUEST: &str =
        "```http\nGET /users?limit=10 HTTP/1.1\nHost: example.com\nAccept: application/json\n```\n";

    const HTTP_RESPONSE: &str = "```http\nHTTP/1.1 200 OK\nContent-Type: application/json\nContent-Length: 27\n\n{\"ok\": true}\n```\n";

    #[test]
    fn probe_http_grammar_scopes() {
        let ss: &SyntaxSet = &SYNTAX_SET;
        let syn = ss
            .find_syntax_by_name("HTTP Request and Response")
            .expect("vendored HTTP grammar must load");

        let mut scopes = String::new();
        let mut state = ParseState::new(syn);
        let body = "GET /users?limit=10 HTTP/1.1\nHost: example.com\n\nHTTP/1.1 200 OK\nContent-Type: application/json\n\n{\"ok\": true}\n";
        for line in LinesWithEndings::from(body) {
            for (_, op) in state
                .parse_line(line, ss)
                .expect("parse_line must not fail")
            {
                scopes.push_str(&format!("{op:?} "));
            }
        }

        for expected in [
            "keyword.operator.word",
            "constant.language",
            "constant.numeric",
            "keyword.other",
        ] {
            assert!(
                scopes.contains(expected),
                "expected scope {expected} in HTTP parse, got: {scopes}"
            );
        }
    }

    #[test]
    fn alias_http_resolves() {
        let syn = Highlight::syntax_for("http");
        assert_eq!(syn.name, "HTTP Request and Response");
    }

    #[test]
    fn html_http_request_has_semantic_tokens() {
        let html = render_html_for(HTTP_REQUEST, "light");
        assert!(html.contains("data-lang=\"http\""), "{html}");
        assert!(
            html.contains("mdv-tok-keyword\">GET</span>"),
            "expected the HTTP verb scoped as a keyword: {html}"
        );
        assert!(
            html.contains("mdv-tok-constant\">HTTP/1.1</span>"),
            "expected the protocol version scoped as a constant: {html}"
        );
        assert!(
            html.contains("mdv-tok-keyword\">Host</span>"),
            "expected the header name scoped as a keyword: {html}"
        );
    }

    #[test]
    fn html_http_response_has_semantic_tokens() {
        let html = render_html_for(HTTP_RESPONSE, "light");
        assert!(html.contains("data-lang=\"http\""), "{html}");
        assert!(
            html.contains("mdv-tok-constant\">200</span>"),
            "expected the status code scoped as a constant: {html}"
        );
        assert!(
            html.contains("mdv-tok-string\">OK</span>"),
            "expected the status text scoped as a string: {html}"
        );
        assert!(
            html.contains("mdv-tok-keyword\">Content-Type</span>"),
            "expected the header name scoped as a keyword: {html}"
        );
    }

    #[test]
    fn html_http_json_body_is_embedded_highlighted() {
        let html = render_html_for(HTTP_RESPONSE, "light");
        assert!(
            html.contains("mdv-tok-constant\">true</span>"),
            "expected the JSON body to highlight via `embed: scope:source.json`: {html}"
        );
    }

    #[test]
    fn terminal_http_request_differs_from_plaintext() {
        let http = render_term_for(HTTP_REQUEST, "dark");
        let plain = render_term_for(&HTTP_REQUEST.replace("```http", "```"), "dark");
        assert!(
            http.contains("\x1b[38;2;"),
            "expected truecolor ANSI: {http:?}"
        );
        assert_ne!(
            http, plain,
            "http fence must be styled differently from the same text as a plain fence"
        );
    }

    #[test]
    fn terminal_http_response_differs_from_plaintext() {
        let http = render_term_for(HTTP_RESPONSE, "dark");
        let plain = render_term_for(&HTTP_RESPONSE.replace("```http", "```"), "dark");
        assert!(
            http.contains("\x1b[38;2;"),
            "expected truecolor ANSI: {http:?}"
        );
        assert_ne!(
            http, plain,
            "http fence must be styled differently from the same text as a plain fence"
        );
    }

    #[test]
    fn vendored_grammar_load_failure_is_graceful() {
        let err = load_vendored("%YAML 1.2\n---\nthis: [is, not, a, syntax", "Bogus")
            .expect_err("malformed grammar must not load");
        assert!(!err.is_empty(), "error must be reportable");

        let mut builder = SyntaxSet::load_defaults_newlines().into_builder();
        add_vendored(&mut builder, "not a grammar at all", "Bogus", "bogus");
        let set = builder.build();
        assert!(
            set.find_syntax_by_name("Bogus").is_none(),
            "a failed grammar must not be registered"
        );
        assert!(
            set.find_syntax_by_name("Rust").is_some(),
            "a failed grammar must not poison the rest of the set"
        );
    }

    #[test]
    fn alias_powershell_resolves() {
        let syn = Highlight::syntax_for("powershell");
        assert_eq!(syn.name, "PowerShell");
    }

    #[test]
    fn alias_ps1_resolves() {
        let syn = Highlight::syntax_for("ps1");
        assert_eq!(syn.name, "PowerShell");
    }

    #[test]
    fn alias_pwsh_resolves() {
        let syn = Highlight::syntax_for("pwsh");
        assert_eq!(syn.name, "PowerShell");
    }

    #[test]
    fn html_powershell_has_token_classes() {
        let html = render_html_for(
            "```powershell\nGet-Process | Where-Object { $_.CPU -gt 100 }\n```\n",
            "light",
        );
        assert!(
            html.contains("mdv-tok"),
            "expected mdv-tok class on PowerShell tokens: {html}"
        );
    }

    #[test]
    fn name_is_highlight() {
        assert_eq!(Highlight.name(), "highlight");
    }

    #[test]
    fn html_hl_lines_marks_correct_lines() {
        let src = "```python hl_lines=\"2 4\"\nline1\nline2\nline3\nline4\n```\n";
        let html = render_html_for(src, "light");
        let marks: Vec<_> = html.match_indices("hl-line--mark").collect();
        assert_eq!(marks.len(), 2, "expected exactly 2 marked lines: {html}");
        let pos2 = html.find("hl-line--mark").unwrap();
        let before2 = &html[..pos2];
        let plain_count = before2.matches("\"hl-line\"").count();
        assert_eq!(
            plain_count, 1,
            "line 1 should be plain hl-line before first mark: {html}"
        );
    }

    #[test]
    fn html_rust_has_token_classes() {
        let html = render_html_for("```rust\nfn main() { println!(\"hi\"); }\n```\n", "light");
        assert!(
            html.contains("<pre class=\"mdv-code\""),
            "pre wrapper: {html}"
        );
        assert!(html.contains("data-lang=\"rust\""));
        assert!(html.contains("<code>"));
        assert!(html.contains("mdv-tok"), "expected token class: {html}");
        assert!(
            html.contains("mdv-tok-function")
                || html.contains("mdv-tok-type")
                || html.contains("mdv-tok-string"),
            "expected at least one specific token class: {html}"
        );
    }

    #[test]
    fn html_python_has_token_classes() {
        let html = render_html_for("```python\ndef f(x):\n    return x + 1\n```\n", "dark");
        assert!(html.contains("data-lang=\"python\""));
        assert!(html.contains("mdv-tok-keyword"));
    }

    #[test]
    fn html_js_has_token_classes() {
        let html = render_html_for(
            "```javascript\nconst a = 1;\nconsole.log(a);\n```\n",
            "dracula",
        );
        assert!(html.contains("data-lang=\"javascript\""));
        assert!(html.contains("mdv-tok-keyword"));
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
        let theme = make_theme("light");
        let ctx = RenderCtx::new(&theme);
        assert!(Highlight.render_html(p, &ctx).is_none());
        assert!(Highlight.render_terminal(p, &ctx).is_none());
    }

    #[test]
    fn client_assets_empty() {
        assert!(Highlight.client_assets().is_empty());
    }

    #[test]
    fn alias_csharp_resolves() {
        assert_eq!(Highlight::syntax_for("csharp").name, "C#");
    }

    #[test]
    fn alias_jsonc_resolves() {
        assert_eq!(Highlight::syntax_for("jsonc").name, "JSON");
    }

    #[test]
    fn html_jsonc_uses_json_highlighting_with_raw_lang_attr() {
        let html = render_html_for("```jsonc\n{\"enabled\": true}\n```\n", "light");
        assert!(html.contains("data-lang=\"jsonc\""), "{html}");
        assert!(
            html.contains("mdv-tok-string"),
            "expected JSON string token span: {html}"
        );
    }

    #[test]
    fn terminal_jsonc_uses_json_highlighting() {
        let src = "```{lang}\n{\"enabled\": true}\n```\n";
        let jsonc = render_term_for(&src.replace("{lang}", "jsonc"), "dark");
        let plain = render_term_for(&src.replace("{lang}", "flibbertigibbet"), "dark");
        assert!(
            jsonc.contains("\x1b[38;2;"),
            "expected 24-bit ANSI color output: {jsonc:?}"
        );
        assert_ne!(
            jsonc, plain,
            "jsonc should be highlighted differently than plaintext"
        );
    }

    #[test]
    fn alias_csharp_case_insensitive() {
        assert_eq!(Highlight::syntax_for("CSHARP").name, "C#");
        assert_eq!(Highlight::syntax_for("csharp").name, "C#");
    }

    #[test]
    fn tabs_expanded_in_html_output() {
        let html = render_html_for("```rust\n\tlet x = 1;\n```\n", "dark");
        assert!(
            !html.contains('\t'),
            "tabs should be expanded to spaces: {html}"
        );
        // 4 spaces (default tab_width) should appear in the output
        assert!(
            html.contains("    "),
            "tab should expand to 4 spaces (default): {html}"
        );
    }

    #[test]
    fn hl_line_spans_are_self_contained() {
        let src = "```rust\nfn main() {\n\tlet x = 1;\n\tif x > 0 {\n\t\tprintln!(\"hi\");\n\t}\n}\n```\n";
        let html = render_html_for(src, "dark");
        // Tabs should be expanded to spaces, so no raw tabs in output
        assert!(
            !html.contains('\t'),
            "tabs should be expanded to spaces in HTML output: {html}"
        );
        // The closing brace should be on the same hl-line as its indentation
        assert!(
            !html.contains("hl-line\">    </span>"),
            "closing brace should not be separated from its indentation: {html}"
        );
    }
}
