//! D2 diagram extension for mdview.
//!
//! Renders fenced ```d2 code blocks by shelling out to the local `d2` CLI.
//! No remote API. No wasm. If `d2` is missing on PATH at render time, the block
//! is rendered as a normal fenced code block (same container as the highlight
//! extension) with a single red prefix line `missing d2 command`. The original
//! d2 source follows verbatim. If `d2` exists but the render fails, the same
//! code-block shape is used with a red `d2 render failed: <message>` prefix
//! line so users see the actual diagnostic rather than a silent fallback.
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

/// The literal prefix used when `d2` exists but failed to render the diagram.
/// The actual failure message is appended after this constant.
pub const RENDER_ERROR_PREFIX: &str = "d2 render failed: ";

/// Max length for the stderr/error snippet we surface in the rendered prefix
/// line. Keeps the UI tidy when d2 dumps a long stack trace.
const MAX_ERROR_SNIPPET: usize = 200;

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

// ---------------------------------------------------------------------------
// HTML rendering
// ---------------------------------------------------------------------------

fn render_html_impl(source: &str, override_path: Option<&std::path::Path>) -> String {
    let result = cli::render_svg_with(source, override_path, cli::DEFAULT_TIMEOUT);
    format_html_from_result(source, result)
}

/// Format the HTML output from a precomputed CLI result. Separated so unit
/// tests can feed in a synthetic [`cli::D2Error`] (covering ExitStatus / Spawn
/// / Io paths) without invoking a real binary.
fn format_html_from_result(source: &str, result: Result<Vec<u8>, cli::D2Error>) -> String {
    match result {
        Ok(svg) => {
            let svg_str = String::from_utf8_lossy(&svg);
            format!("<div class=\"d2\">{}</div>", inline_svg(&svg_str))
        }
        Err(cli::D2Error::NotFound) => missing_cli_html(source),
        Err(e) => render_error_html(source, &error_message(&e)),
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

/// Render the d2 block as a code block with a red error-prefix line. Used
/// when the `d2` CLI exists but fails (spawn / io / non-zero exit). Keeps
/// the user's source visible AND surfaces the real diagnostic.
fn render_error_html(source: &str, message: &str) -> String {
    let mut s = String::from("<pre class=\"mdv-code\" data-lang=\"d2\"><code>");
    s.push_str("<span class=\"mdv-d2-error\" style=\"color:var(--mdv-error,#f38ba8)\">");
    s.push_str(&escape_html(RENDER_ERROR_PREFIX));
    s.push_str(&escape_html(message));
    s.push_str("</span>\n");
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

// ---------------------------------------------------------------------------
// Terminal rendering
// ---------------------------------------------------------------------------

fn render_terminal_impl<R: SixelRenderer>(
    source: &str,
    renderer: &R,
    override_path: Option<&std::path::Path>,
) -> TermChunks {
    let result = cli::render_svg_with(source, override_path, cli::DEFAULT_TIMEOUT);
    format_terminal_from_result(source, result, Some(renderer))
}

fn render_terminal_default(source: &str, override_path: Option<&std::path::Path>) -> TermChunks {
    // The default path is the one invoked when no sixel-capable renderer is
    // wired up. If `d2` is missing, surface the same red prefix block; if
    // `d2` exists but errors, surface the red error-prefix block; on success
    // fall back to plain code (still safer than silently dropping the
    // diagram — there is no sixel renderer to draw the SVG).
    if cli::locate_d2(override_path).is_none() {
        return missing_cli_terminal(source);
    }
    let result = cli::render_svg_with(source, override_path, cli::DEFAULT_TIMEOUT);
    match result {
        Ok(_) => fallback_code_terminal(source),
        Err(cli::D2Error::NotFound) => missing_cli_terminal(source),
        Err(e) => render_error_terminal(source, &error_message(&e)),
    }
}

/// Format terminal chunks from a precomputed CLI result. When a renderer is
/// supplied AND the call succeeded, the SVG is handed off for sixel
/// rasterization. Separated so unit tests can feed synthetic errors.
fn format_terminal_from_result<R: SixelRenderer>(
    source: &str,
    result: Result<Vec<u8>, cli::D2Error>,
    renderer: Option<&R>,
) -> TermChunks {
    match result {
        Ok(svg) => match renderer {
            Some(r) => match r.render_image(&svg) {
                Ok(bytes) => vec![TermChunk::plain(
                    String::from_utf8_lossy(&bytes).into_owned(),
                )],
                Err(_) => fallback_code_terminal(source),
            },
            None => fallback_code_terminal(source),
        },
        Err(cli::D2Error::NotFound) => missing_cli_terminal(source),
        Err(e) => render_error_terminal(source, &error_message(&e)),
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

fn render_error_terminal(source: &str, message: &str) -> TermChunks {
    let mut text = String::new();
    text.push_str(ANSI_RED);
    text.push_str(RENDER_ERROR_PREFIX);
    text.push_str(message);
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

// ---------------------------------------------------------------------------
// Error formatting
// ---------------------------------------------------------------------------

/// Extract a user-facing message from a [`cli::D2Error`].
///
/// For [`cli::D2Error::ExitStatus`] we surface the first non-empty stderr
/// line, truncated to [`MAX_ERROR_SNIPPET`] chars so a stack trace doesn't
/// blow up the rendered code block. Spawn / Io errors get a short label
/// followed by the underlying message. `NotFound` is not handled here —
/// callers route it to the missing-CLI path separately.
fn error_message(e: &cli::D2Error) -> String {
    match e {
        cli::D2Error::NotFound => INSTALL_HINT_FALLBACK.to_string(),
        cli::D2Error::Spawn(s) => format!("spawn failed: {}", truncate(s, MAX_ERROR_SNIPPET)),
        cli::D2Error::Io(s) => format!("io error: {}", truncate(s, MAX_ERROR_SNIPPET)),
        cli::D2Error::ExitStatus { code, stderr } => {
            let snippet = first_nonempty_line(stderr);
            let snippet = truncate(snippet, MAX_ERROR_SNIPPET);
            if snippet.is_empty() {
                format!("exit status {code}")
            } else {
                format!("exit status {code}: {snippet}")
            }
        }
    }
}

/// Used only by [`error_message`] as a defensive fallback if [`cli::D2Error::NotFound`]
/// is ever routed here by mistake. The normal NotFound path is the
/// `missing_cli_*` renderers, which never call [`error_message`].
const INSTALL_HINT_FALLBACK: &str = "d2 not found";

fn first_nonempty_line(s: &str) -> &str {
    s.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
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

    /// Stand-in for a `SixelRenderer` that's expected never to be called.
    struct NeverRenderer;
    impl SixelRenderer for NeverRenderer {
        fn render_image(&self, _svg: &[u8]) -> Result<Vec<u8>, String> {
            panic!("must not be invoked");
        }
    }

    #[test]
    fn parser_detects_d2_fenced_block() {
        let arena = Arena::new();
        let root = parse_document(&arena, "```d2\na -> b\n```\n", &ComrakOptions::default());
        let node = first_code_block(root).expect("code block");
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

    /// HTML: missing CLI renders as a code block with the red
    /// `missing d2 command` prefix as the first child of `<code>`. (Bug 1
    /// does NOT change this path; the existing contract is preserved.)
    #[test]
    fn html_without_cli_renders_code_block_with_red_prefix() {
        let source = "a -> b\n";
        let missing = missing_path();
        let html = render_html_impl(source, Some(&missing));

        assert!(
            html.contains("<pre class=\"mdv-code\""),
            "expected mdv-code container, got: {html}"
        );
        assert!(html.contains("<code>"));
        assert!(html.contains("a -&gt; b"), "source missing: {html}");

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

        assert!(html.contains("mdv-d2-missing"), "expected red prefix class");
        assert!(
            html.contains("#f38ba8") || html.contains("--mdv-error"),
            "expected red color hint (catppuccin red or --mdv-error): {html}"
        );

        // Regression: render-error prefix must NOT appear (this is the
        // missing-CLI path, not the render-error path).
        assert!(
            !html.contains("d2 render failed:"),
            "missing-CLI path leaked render-error prefix: {html}"
        );
        assert!(
            !html.contains("d2lang.com/tour/install"),
            "install URL leaked into rendered output: {html}"
        );
    }

    /// Terminal (default path): missing CLI → ANSI red `missing d2 command`.
    #[test]
    fn terminal_without_cli_renders_code_with_ansi_red_prefix() {
        let source = "a -> b\n";
        let missing = missing_path();
        let chunks = render_terminal_default(source, Some(&missing));
        let text = joined_text(&chunks);

        assert!(
            text.starts_with("\x1b[31mmissing d2 command\x1b[0m\n"),
            "expected ANSI red prefix at start, got: {text:?}"
        );
        assert!(text.contains("a -> b"), "source missing: {text:?}");
        assert!(
            !text.contains("d2lang.com/tour/install"),
            "install URL leaked: {text:?}"
        );
        assert!(
            !text.contains("d2 render failed:"),
            "missing-CLI path leaked render-error prefix: {text:?}"
        );
    }

    /// Terminal (sixel-aware path): missing CLI → same expectations.
    #[test]
    fn render_terminal_with_without_cli_renders_code_with_ansi_red_prefix() {
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

    // -------------------------------------------------------------------
    // Bug 1 regressions: ExitStatus / Spawn / Io must surface a real
    // diagnostic via the `d2 render failed: …` red prefix, not be
    // silently swallowed.
    // -------------------------------------------------------------------

    #[test]
    fn html_render_error_exit_status_surfaces_stderr_snippet() {
        let source = "a -> b\nc -> d\n";
        let synthetic = Err(cli::D2Error::ExitStatus {
            code: 2,
            stderr: "boom: the universe imploded\nstack trace line 1\n".into(),
        });
        let html = format_html_from_result(source, synthetic);

        // Container shape: still a code block.
        assert!(
            html.contains("<pre class=\"mdv-code\""),
            "expected mdv-code container, got: {html}"
        );
        assert!(html.contains("<code>"));

        // Red prefix span is the first child of <code> and carries the
        // RENDER_ERROR_PREFIX plus the actual stderr line.
        let code_open = html.find("<code>").expect("code open");
        let after_code = &html[code_open + "<code>".len()..];
        assert!(
            after_code.starts_with("<span class=\"mdv-d2-error\""),
            "red error prefix not first child of <code>: {after_code}"
        );
        assert!(
            after_code.contains("d2 render failed:"),
            "RENDER_ERROR_PREFIX missing: {after_code}"
        );
        assert!(
            after_code.contains("exit status 2"),
            "exit code missing: {after_code}"
        );
        assert!(
            after_code.contains("boom: the universe imploded"),
            "stderr snippet missing: {after_code}"
        );

        // Source preserved.
        assert!(html.contains("a -&gt; b"), "source line 1 missing: {html}");
        assert!(html.contains("c -&gt; d"), "source line 2 missing: {html}");

        // Red color hint present, and install URL absent.
        assert!(
            html.contains("#f38ba8") || html.contains("--mdv-error"),
            "expected red color hint: {html}"
        );
        assert!(
            !html.contains("d2lang.com/tour/install"),
            "install URL leaked into render-error output: {html}"
        );

        // The render-error path must NOT use the missing-CLI class.
        assert!(
            !html.contains("mdv-d2-missing"),
            "render-error path should not use missing-CLI class: {html}"
        );
    }

    #[test]
    fn terminal_render_error_exit_status_surfaces_ansi_red_prefix() {
        let source = "a -> b\n";
        let synthetic = Err(cli::D2Error::ExitStatus {
            code: 2,
            stderr: "boom\n".into(),
        });
        let chunks = format_terminal_from_result::<NeverRenderer>(source, synthetic, None);
        let text = joined_text(&chunks);

        assert!(
            text.starts_with("\x1b[31md2 render failed: exit status 2: boom\x1b[0m\n"),
            "expected ANSI red render-failed prefix, got: {text:?}"
        );
        assert!(text.contains("a -> b"), "source missing: {text:?}");
        assert!(
            !text.contains("d2lang.com/tour/install"),
            "install URL leaked: {text:?}"
        );
    }

    #[test]
    fn html_render_error_spawn_surfaces_short_message() {
        let synthetic = Err(cli::D2Error::Spawn("permission denied".into()));
        let html = format_html_from_result("a -> b\n", synthetic);
        assert!(
            html.contains("d2 render failed: spawn failed: permission denied"),
            "spawn message missing: {html}"
        );
        assert!(
            html.contains("mdv-d2-error"),
            "expected render-error class: {html}"
        );
        assert!(html.contains("a -&gt; b"), "source missing: {html}");
    }

    #[test]
    fn html_render_error_io_surfaces_short_message() {
        let synthetic = Err(cli::D2Error::Io("disk on fire".into()));
        let html = format_html_from_result("a -> b\n", synthetic);
        assert!(
            html.contains("d2 render failed: io error: disk on fire"),
            "io message missing: {html}"
        );
    }

    #[test]
    fn error_message_truncates_long_stderr() {
        let long = "x".repeat(500);
        let e = cli::D2Error::ExitStatus {
            code: 1,
            stderr: long.clone(),
        };
        let msg = error_message(&e);
        // The snippet should be capped at MAX_ERROR_SNIPPET chars plus the
        // ellipsis. Allow for the "exit status N: " preamble.
        let snippet_len = msg
            .rsplit_once(": ")
            .map(|(_, s)| s.chars().count())
            .unwrap_or(0);
        assert!(
            snippet_len <= MAX_ERROR_SNIPPET + 1,
            "snippet not truncated: len={snippet_len}"
        );
        assert!(msg.ends_with('…'), "expected ellipsis suffix: {msg}");
    }

    #[test]
    fn error_message_picks_first_nonempty_stderr_line() {
        let e = cli::D2Error::ExitStatus {
            code: 3,
            stderr: "\n   \nfirst real line\nsecond line\n".into(),
        };
        let msg = error_message(&e);
        assert!(msg.contains("first real line"), "msg: {msg}");
        assert!(!msg.contains("second line"), "msg: {msg}");
    }

    #[test]
    fn html_render_success_path_emits_inline_svg_div() {
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><g/></svg>".to_vec();
        let html = format_html_from_result("a -> b\n", Ok(svg));
        assert!(
            html.starts_with("<div class=\"d2\">"),
            "expected d2 wrapper div: {html}"
        );
        assert!(html.contains("<svg"), "svg missing: {html}");
        assert!(
            !html.contains("d2 render failed:"),
            "success path leaked error prefix: {html}"
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

    // -------------------------------------------------------------------
    // Gated real-render tests. Only run when `d2-cli-tests` is enabled
    // AND the host actually has `d2` on PATH (we soft-skip otherwise so
    // CI stays green on hosts without d2).
    // -------------------------------------------------------------------

    #[cfg(feature = "d2-cli-tests")]
    fn d2_cli_available() -> bool {
        std::process::Command::new("d2")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[cfg(feature = "d2-cli-tests")]
    #[test]
    fn real_render_produces_svg() {
        if !d2_cli_available() {
            eprintln!("skipping: d2 CLI not available");
            return;
        }
        let svg = cli::render_svg("a -> b\n").expect("d2 svg");
        let text = String::from_utf8_lossy(&svg);
        assert!(text.contains("<svg"));
    }

    /// Regression for Bug 2 (Windows tempfile handle conflict): a multi-line
    /// d2 source must round-trip through the CLI and produce a non-empty
    /// SVG containing `<svg`. Pre-fix this failed on Windows because the
    /// `NamedTempFile` handle was holding the output path exclusively.
    #[cfg(feature = "d2-cli-tests")]
    #[test]
    fn real_render_multi_line_produces_nonempty_svg() {
        if !d2_cli_available() {
            eprintln!("skipping: d2 CLI not available");
            return;
        }
        let source = "a -> b\nb -> c\nc -> a\n";
        let svg = cli::render_svg(source).expect("d2 svg");
        assert!(!svg.is_empty(), "SVG bytes empty");
        let text = String::from_utf8_lossy(&svg);
        assert!(
            text.contains("<svg"),
            "expected <svg in output, got: {text:?}"
        );
    }
}
