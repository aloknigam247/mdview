//! D2 diagram extension for mdview.
//!
//! Renders fenced ```d2 code blocks by shelling out to the local `d2` CLI.
//! No remote API. No wasm. If `d2` is missing on PATH at render time, the block
//! is rendered as a normal fenced code block (same container as the highlight
//! extension) with a single red prefix line `missing d2 command`. The original
//! d2 source follows verbatim.
#![forbid(unsafe_code)]

mod _stubs;
pub mod cli;
pub mod scan;

pub use _stubs::{
    Asset, AstNode, Html, MdViewExtension, RenderCtx, SixelRenderer, StyleSpec, TermChunk,
    TermChunks, Theme,
};

use comrak::nodes::NodeValue;

/// The literal prefix line shown when the `d2` CLI is missing. Both HTML and
/// terminal outputs surface this verbatim (HTML styled red via class,
/// terminal styled red via ANSI SGR).
pub const MISSING_CLI_PREFIX: &str = "missing d2 command";

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
        Some(render_terminal_impl(&source, renderer, None))
    }
}

impl MdViewExtension for D2 {
    fn name(&self) -> &'static str {
        "d2"
    }

    fn render_html<'a>(&self, node: &'a AstNode<'a>, _ctx: &RenderCtx<'_>) -> Option<Html> {
        let source = Self::extract_source(node)?;
        Some(Html(render_html_impl(&source, None)))
    }

    fn render_terminal<'a>(
        &self,
        node: &'a AstNode<'a>,
        _ctx: &RenderCtx<'_>,
    ) -> Option<TermChunks> {
        let source = Self::extract_source(node)?;
        Some(render_terminal_default(&source, None))
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

fn render_html_impl(source: &str, override_path: Option<&std::path::Path>) -> String {
    match cli::render_svg_with(source, override_path, cli::DEFAULT_TIMEOUT) {
        Ok(svg) => {
            let svg_str = String::from_utf8_lossy(&svg);
            format!("<div class=\"d2\">{}</div>", inline_svg(&svg_str))
        }
        Err(cli::D2Error::NotFound) => missing_cli_html(source),
        // For non-missing CLI errors we still surface as a code block, but
        // without the red prefix — keep the d2 source visible so users see
        // exactly what failed. (Mirrors the spirit of "treat as code".)
        Err(_) => fallback_code_html(source),
    }
}

fn inline_svg(svg: &str) -> String {
    // `d2 --omit-xml-tag` already strips the XML declaration; pass through as-is.
    svg.trim().to_string()
}

/// Render the d2 block as a normal code block with a red `missing d2 command`
/// prefix as the first child. The container matches the highlight extension's
/// `<pre class="mdv-code">…<code>` shape so theming is consistent.
fn missing_cli_html(source: &str) -> String {
    let mut s = String::from("<pre class=\"mdv-code\" data-lang=\"d2\"><code>");
    s.push_str("<span class=\"mdv-d2-missing\" style=\"color:var(--mdv-error,#f38ba8)\">");
    s.push_str(&escape_html(MISSING_CLI_PREFIX));
    s.push_str("</span>\n");
    s.push_str(&escape_html(source));
    s.push_str("</code></pre>");
    s
}

/// Generic code-block fallback used when `d2` exists but errors out at render
/// time. Keeps the user's source visible without claiming the CLI is missing.
fn fallback_code_html(source: &str) -> String {
    let mut s = String::from("<pre class=\"mdv-code\" data-lang=\"d2\"><code>");
    s.push_str(&escape_html(source));
    s.push_str("</code></pre>");
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

fn render_terminal_impl<R: SixelRenderer>(
    source: &str,
    renderer: &R,
    override_path: Option<&std::path::Path>,
) -> TermChunks {
    match cli::render_svg_with(source, override_path, cli::DEFAULT_TIMEOUT) {
        Ok(svg) => match renderer.render_image(&svg) {
            Ok(bytes) => vec![TermChunk::plain(
                String::from_utf8_lossy(&bytes).into_owned(),
            )],
            Err(_) => fallback_code_terminal(source),
        },
        Err(cli::D2Error::NotFound) => missing_cli_terminal(source),
        Err(_) => fallback_code_terminal(source),
    }
}

fn render_terminal_default(source: &str, override_path: Option<&std::path::Path>) -> TermChunks {
    // The default path is the one invoked when no sixel-capable renderer is
    // wired up. If `d2` is missing, surface the same red prefix block;
    // otherwise fall back to plain code (still safer than silently dropping
    // the diagram).
    if cli::locate_d2(override_path).is_none() {
        missing_cli_terminal(source)
    } else {
        fallback_code_terminal(source)
    }
}

const ANSI_RED: &str = "\x1b[31m";
const ANSI_RESET: &str = "\x1b[0m";

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

    /// A path that is guaranteed not to exist on the host. Passing this to
    /// the override parameter forces `locate_d2` to return `None` — which is
    /// indistinguishable from "no `d2` on PATH" in the production code path,
    /// without touching the process environment.
    fn missing_path() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push("mdview-d2-tests");
        p.push("definitely-not-a-real-d2-binary");
        p
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

    /// HTML: missing CLI renders as a code block, the source is visible, and
    /// the first line of code content is the red `missing d2 command` span.
    /// Detection is forced to "missing" by passing a non-existent override
    /// path to the internal renderer (the same path the trait method takes
    /// after extracting the source).
    #[test]
    fn html_without_cli_renders_code_block_with_red_prefix() {
        let source = "a -> b\n";
        let missing = missing_path();
        let html = render_html_impl(source, Some(&missing));

        // 1. Output looks like a normal mdview code block container.
        assert!(
            html.contains("<pre class=\"mdv-code\""),
            "expected mdv-code container, got: {html}"
        );
        assert!(html.contains("<code>"));

        // 2. Original d2 source is preserved (HTML-escaped).
        assert!(html.contains("a -&gt; b"), "source missing: {html}");

        // 3. Red prefix line is the first thing inside <code>.
        let code_open = html.find("<code>").expect("code open");
        let after_code = &html[code_open + "<code>".len()..];
        assert!(
            after_code.starts_with("<span class=\"mdv-d2-missing\""),
            "red prefix not first child of <code>: {after_code}"
        );
        assert!(
            after_code.contains("missing d2 command"),
            "prefix text missing: {after_code}"
        );

        // 4. Red styling is present (either class or inline style).
        assert!(html.contains("mdv-d2-missing"), "expected red prefix class");
        assert!(
            html.contains("#f38ba8") || html.contains("--mdv-error"),
            "expected red color hint (catppuccin red or --mdv-error): {html}"
        );

        // 5. Regression: install URL must NOT appear in rendered output.
        assert!(
            !html.contains("d2lang.com/tour/install"),
            "install URL leaked into rendered output: {html}"
        );
    }

    /// Terminal (default path): missing CLI renders the source as plain text
    /// with an ANSI-red prefix line `missing d2 command` and no install URL.
    #[test]
    fn terminal_without_cli_renders_code_with_ansi_red_prefix() {
        let source = "a -> b\n";
        let missing = missing_path();
        let chunks = render_terminal_default(source, Some(&missing));
        let text = joined_text(&chunks);

        // 4. ANSI red prefix at start.
        assert!(
            text.starts_with("\x1b[31mmissing d2 command\x1b[0m\n"),
            "expected ANSI red prefix at start, got: {text:?}"
        );

        // 1. Source is visible as code below the prefix.
        assert!(text.contains("a -> b"), "source missing: {text:?}");

        // 5. Regression: no install URL in terminal output.
        assert!(
            !text.contains("d2lang.com/tour/install"),
            "install URL leaked: {text:?}"
        );
    }

    /// Terminal (sixel-aware path): same expectations when CLI is missing.
    #[test]
    fn render_terminal_with_without_cli_renders_code_with_ansi_red_prefix() {
        struct NeverRenderer;
        impl SixelRenderer for NeverRenderer {
            fn render_image(&self, _svg: &[u8]) -> Result<Vec<u8>, String> {
                panic!("must not be invoked when CLI missing");
            }
        }
        let source = "a -> b\n";
        let missing = missing_path();
        let chunks = render_terminal_impl(source, &NeverRenderer, Some(&missing));
        let text = joined_text(&chunks);

        assert!(
            text.starts_with("\x1b[31mmissing d2 command\x1b[0m\n"),
            "expected ANSI red prefix at start, got: {text:?}"
        );
        assert!(text.contains("a -> b"));
        assert!(
            !text.contains("d2lang.com/tour/install"),
            "install URL leaked: {text:?}"
        );
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
