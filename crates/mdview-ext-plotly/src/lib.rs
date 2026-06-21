#![forbid(unsafe_code)]

//! Plotly extension for mdview.
//!
//! Matches fenced code blocks whose info string is `plotly`. The body must
//! parse as a JSON object exposing a `data` array (and optionally a `layout`
//! object). For the HTML surface a client-side `<div class="plotly-chart">`
//! is emitted; for the terminal surface we shell out to `mdview-sidecar` to
//! produce an SVG that is forwarded to the sixel renderer, falling back to
//! an ASCII placeholder when the sidecar is missing.

mod _stubs;

pub use _stubs::{Asset, AstNode, Html, MdViewExtension, RenderCtx, TermChunk, TermChunks, Theme};

use comrak::nodes::NodeValue;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const SIDECAR_BIN: &str = "mdview-sidecar";
const SIDECAR_ENV: &str = "MDVIEW_SIDECAR";

/// The literal prefix line shown when the `mdview-sidecar` binary is missing
/// or otherwise unavailable. Surfaced in terminal output with ANSI red SGR.
pub const MISSING_CLI_PREFIX: &str = "missing plotly command";

const ANSI_RED: &str = "\x1b[31m";
const ANSI_RESET: &str = "\x1b[0m";

static CLIENT_ASSETS: &[Asset] = &[
    Asset {
        path: "vendor/mdv-plotly-init.js",
        mime: "application/javascript",
    },
    Asset {
        path: "vendor/plotly.min.js",
        mime: "application/javascript",
    },
];

#[derive(Debug, thiserror::Error)]
pub enum PlotlyError {
    #[error("invalid plotly json: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("sidecar exited with non-zero status")]
    SidecarFailed,
    #[error("sidecar io error: {0}")]
    SidecarIo(#[from] std::io::Error),
    #[error("sidecar not available")]
    SidecarMissing,
}

pub struct Plotly;

impl Plotly {
    fn fenced_plotly_source<'a>(node: &'a AstNode<'a>) -> Option<String> {
        match &node.data.borrow().value {
            NodeValue::CodeBlock(block) => {
                let info = block.info.trim();
                if info.eq_ignore_ascii_case("plotly") {
                    Some(block.literal.clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn validate(source: &str) -> Result<serde_json::Value, PlotlyError> {
        let value: serde_json::Value = serde_json::from_str(source)?;
        Ok(value)
    }

    fn emit_html(source: &str) -> Html {
        let spec = match Self::validate(source) {
            Ok(v) => v.to_string(),
            Err(_) => serde_json::Value::Null.to_string(),
        };
        let escaped = escape_attr(&spec);
        Html(format!(
            "<div class=\"plotly-chart\" data-spec=\"{escaped}\"></div>"
        ))
    }

    fn locate_sidecar() -> Option<PathBuf> {
        if let Some(env_path) = std::env::var_os(SIDECAR_ENV) {
            let p = PathBuf::from(env_path);
            if p.exists() {
                return Some(p);
            }
            return None;
        }
        which::which(SIDECAR_BIN).ok()
    }

    fn emit_terminal(source: &str) -> TermChunks {
        match Self::run_sidecar(source) {
            Ok(svg) => vec![TermChunk::plain(String::from_utf8_lossy(&svg).into_owned())],
            Err(PlotlyError::SidecarMissing) => missing_cli_terminal(source),
            Err(_) => fallback_code_terminal(source),
        }
    }

    fn run_sidecar(source: &str) -> Result<Vec<u8>, PlotlyError> {
        let sidecar = Self::locate_sidecar().ok_or(PlotlyError::SidecarMissing)?;
        let job = serde_json::json!({
            "kind": "plotly",
            "source": source,
        })
        .to_string();
        let mut child = Command::new(sidecar)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(job.as_bytes())?;
        }
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(PlotlyError::SidecarFailed);
        }
        Ok(output.stdout)
    }
}

impl MdViewExtension for Plotly {
    fn name(&self) -> &'static str {
        "plotly"
    }

    fn render_html<'a>(&self, n: &'a AstNode<'a>, _ctx: &RenderCtx<'_>) -> Option<Html> {
        let source = Self::fenced_plotly_source(n)?;
        Some(Self::emit_html(&source))
    }

    fn render_terminal<'a>(&self, n: &'a AstNode<'a>, _ctx: &RenderCtx<'_>) -> Option<TermChunks> {
        let source = Self::fenced_plotly_source(n)?;
        Some(Self::emit_terminal(&source))
    }

    fn client_assets(&self) -> &'static [Asset] {
        CLIENT_ASSETS
    }
}

fn escape_attr(s: &str) -> String {
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

/// Render the block as a normal code block with a single ANSI-red prefix line
/// reading `missing plotly command`. Used when the sidecar binary cannot be
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

/// Convenience entry point used by the demo binaries. Walks the parsed AST
/// and substitutes fenced `plotly` blocks with rendered HTML.
pub fn render_markdown_html(markdown: &str) -> String {
    let arena = comrak::Arena::new();
    let opts = comrak::ComrakOptions::default();
    let root = comrak::parse_document(&arena, markdown, &opts);

    let mut out = String::new();
    let theme = Theme::default();
    let ctx = RenderCtx::new(&theme);
    let ext = Plotly;

    for node in root.children() {
        if let Some(Html(html)) = ext.render_html(node, &ctx) {
            out.push_str(&html);
            out.push('\n');
            continue;
        }

        let mut buf = Vec::new();
        let _ = comrak::format_html(node, &opts, &mut buf);
        out.push_str(&String::from_utf8_lossy(&buf));
    }
    out
}

/// Locate `fixtures/<name>` by walking upward from the crate root.
pub fn locate_fixture(name: &str) -> std::path::PathBuf {
    let crate_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for up in 0..4 {
        let mut p = crate_root.clone();
        for _ in 0..up {
            p.pop();
        }
        let candidate = p.join("fixtures").join(name);
        if candidate.exists() {
            return candidate;
        }
    }
    panic!("fixtures/{name} not found relative to {crate_root:?}");
}

/// HTML skeleton served by `demo_serve` (and reused by tests).
pub fn demo_document(markdown: &str) -> String {
    let body = render_markdown_html(markdown);
    let plotly_js = include_str!("../assets/vendor/plotly.min.js");
    let init_js = include_str!("../assets/vendor/mdv-plotly-init.js");
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>mdview plotly demo</title>\
         <style>body{{font-family:ui-sans-serif,system-ui;padding:2rem;line-height:1.7}}\
         .plotly-chart{{border-radius:14px;box-shadow:0 1px 3px rgba(0,0,0,.08);padding:1rem;margin:1rem 0;min-height:320px}}</style>\
         </head><body>{body}<script>{plotly_js}</script><script>{init_js}</script></body></html>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_first_block<'a>(arena: &'a comrak::Arena<AstNode<'a>>, md: &str) -> &'a AstNode<'a> {
        let opts = comrak::ComrakOptions::default();
        let root = comrak::parse_document(arena, md, &opts);
        root.first_child().expect("at least one node")
    }

    #[test]
    fn matches_plotly_fence() {
        let arena = comrak::Arena::new();
        let node = parse_first_block(
            &arena,
            "```plotly\n{\"data\":[{\"x\":[1,2],\"y\":[3,4],\"type\":\"scatter\"}]}\n```\n",
        );
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        let html = Plotly.render_html(node, &ctx).unwrap();
        assert!(html.0.contains("class=\"plotly-chart\""));
        assert!(html.0.contains("data-spec=\""));
    }

    #[test]
    fn ignores_non_plotly_fence() {
        let arena = comrak::Arena::new();
        let node = parse_first_block(&arena, "```rust\nfn main(){}\n```\n");
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        assert!(Plotly.render_html(node, &ctx).is_none());
    }

    #[test]
    fn html_escapes_embedded_quotes_and_angles() {
        let arena = comrak::Arena::new();
        let node = parse_first_block(
            &arena,
            "```plotly\n{\"data\":[{\"text\":\"<bad>&\\\"stuff\\\"\"}]}\n```\n",
        );
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        let html = Plotly.render_html(node, &ctx).unwrap();
        assert!(!html.0.contains("<bad>"));
        assert!(html.0.contains("&lt;bad&gt;"));
        assert!(html.0.contains("&quot;"));
        assert!(html.0.contains("&amp;"));
    }

    #[test]
    fn invalid_json_yields_null_spec() {
        let arena = comrak::Arena::new();
        let node = parse_first_block(&arena, "```plotly\nnot json at all\n```\n");
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        let html = Plotly.render_html(node, &ctx).unwrap();
        assert!(html.0.contains("data-spec=\"null\""));
    }

    #[test]
    fn terminal_renders_code_with_ansi_red_prefix_when_sidecar_missing() {
        let prev_env = std::env::var_os(SIDECAR_ENV);
        std::env::set_var(SIDECAR_ENV, "/definitely/nonexistent/mdview-sidecar-xyzzy");

        let arena = comrak::Arena::new();
        let src = "{\"data\":[{\"x\":[1],\"y\":[2]}]}";
        let md = format!("```plotly\n{src}\n```\n");
        let node = parse_first_block(&arena, &md);
        let theme = Theme::default();
        let ctx = RenderCtx::new(&theme);
        let chunks = Plotly.render_terminal(node, &ctx).unwrap();
        assert_eq!(chunks.len(), 1);
        let text = &chunks[0].text;
        assert!(
            text.starts_with("\x1b[31mmissing plotly command\x1b[0m\n"),
            "expected ANSI red prefix at start, got: {text:?}"
        );
        assert!(text.contains(src), "source missing: {text:?}");
        assert!(
            !text.contains("placeholder"),
            "old placeholder marker leaked: {text:?}"
        );

        match prev_env {
            Some(v) => std::env::set_var(SIDECAR_ENV, v),
            None => std::env::remove_var(SIDECAR_ENV),
        }
    }

    #[test]
    fn client_assets_lists_plotly_and_init() {
        let assets = Plotly.client_assets();
        assert_eq!(assets.len(), 2);
        let names: Vec<_> = assets.iter().map(|a| a.path).collect();
        assert!(names.iter().any(|p| p.ends_with("plotly.min.js")));
        assert!(names.iter().any(|p| p.ends_with("mdv-plotly-init.js")));
        for a in assets {
            assert_eq!(a.mime, "application/javascript");
        }
    }
}
