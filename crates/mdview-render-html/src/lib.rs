#![forbid(unsafe_code)]

pub mod _stubs;
pub mod htmlesc;

pub use _stubs::{
    mdview_theme as theme_mod, Asset, AstNode, Html, HtmlRenderer, Radii, Registry, RenderCtx,
    StyleSpec, Theme, Typography,
};

use comrak::nodes::NodeValue;
use comrak::{format_html, parse_document, Arena, ComrakOptions};

const LIVE_RELOAD_SCRIPT: &str = r#"<script>
(function () {
  try {
    var ws = new WebSocket("ws://" + location.host + "/__mdview_live");
    ws.onmessage = function (ev) {
      if (ev.data === "reload") { location.reload(); }
    };
    ws.onclose = function () {
      setTimeout(function () { location.reload(); }, 1500);
    };
  } catch (e) { /* live-reload disabled */ }
})();
</script>"#;

/// Render a parsed comrak AST into a full HTML5 document.
///
/// Walks the AST; for each node, if any registered `HtmlRenderer` claims it,
/// that renderer's raw HTML is used. Otherwise comrak's built-in HTML
/// formatter produces the output.
pub fn render<'a>(root: &'a AstNode<'a>, ctx: &RenderCtx, registry: &Registry) -> Html {
    let opts = markdown_options();
    let body = render_body(root, ctx, registry, &opts);
    let css = theme_mod::emit_css(&ctx.theme);
    let base = base_stylesheet();
    let title = htmlesc::escape_html(&ctx.title);
    let live = if ctx.live_reload {
        LIVE_RELOAD_SCRIPT
    } else {
        ""
    };
    format!(
        "<!doctype html>\n<html><head><meta charset=utf-8><title>{title}</title><style>{css}{base}</style></head><body class=\"mdv\">{body}{live}</body></html>"
    )
}

/// Parse markdown and render HTML in one step. Convenience for demos/tests.
pub fn render_markdown(src: &str, ctx: &RenderCtx, registry: &Registry) -> Html {
    let arena = Arena::new();
    let opts = markdown_options();
    let root = parse_document(&arena, src, &opts);
    render(root, ctx, registry)
}

pub fn markdown_options() -> ComrakOptions<'static> {
    let mut o = ComrakOptions::default();
    o.extension.table = true;
    o.extension.strikethrough = true;
    o.extension.tasklist = true;
    o.extension.autolink = true;
    o.extension.footnotes = true;
    o.render.unsafe_ = false;
    o
}

fn render_body<'a>(
    root: &'a AstNode<'a>,
    ctx: &RenderCtx,
    registry: &Registry,
    opts: &ComrakOptions<'_>,
) -> String {
    let mut out = String::from("<article class=\"mdv-doc\">");
    for child in root.children() {
        if let Some(override_html) = override_for(child, ctx, registry) {
            out.push_str(&override_html);
        } else {
            out.push_str(&comrak_serialize(child, opts));
        }
    }
    out.push_str("</article>");
    out
}

fn override_for<'a>(node: &'a AstNode<'a>, ctx: &RenderCtx, registry: &Registry) -> Option<String> {
    let kind = node_kind(node);
    for r in registry.html_renderers() {
        let types = r.node_types();
        let matches_kind = types.is_empty() || types.iter().any(|t| *t == kind || *t == "*");
        if matches_kind && r.test(node) {
            return Some(r.render(node, ctx));
        }
    }
    None
}

fn comrak_serialize<'a>(node: &'a AstNode<'a>, opts: &ComrakOptions<'_>) -> String {
    let mut buf: Vec<u8> = Vec::new();
    if format_html(node, opts, &mut buf).is_err() {
        return String::new();
    }
    String::from_utf8(buf).unwrap_or_default()
}

/// Best-effort stable string label for each comrak node kind, used by
/// renderer overrides to declare which node types they handle.
pub fn node_kind<'a>(node: &'a AstNode<'a>) -> &'static str {
    match node.data.borrow().value {
        NodeValue::Document => "document",
        NodeValue::FrontMatter(_) => "front_matter",
        NodeValue::BlockQuote => "block_quote",
        NodeValue::List(_) => "list",
        NodeValue::Item(_) => "item",
        NodeValue::DescriptionList => "description_list",
        NodeValue::DescriptionItem(_) => "description_item",
        NodeValue::DescriptionTerm => "description_term",
        NodeValue::DescriptionDetails => "description_details",
        NodeValue::CodeBlock(_) => "code_block",
        NodeValue::HtmlBlock(_) => "html_block",
        NodeValue::Paragraph => "paragraph",
        NodeValue::Heading(_) => "heading",
        NodeValue::ThematicBreak => "thematic_break",
        NodeValue::FootnoteDefinition(_) => "footnote_definition",
        NodeValue::Table(_) => "table",
        NodeValue::TableRow(_) => "table_row",
        NodeValue::TableCell => "table_cell",
        NodeValue::Text(_) => "text",
        NodeValue::TaskItem(_) => "task_item",
        NodeValue::SoftBreak => "soft_break",
        NodeValue::LineBreak => "line_break",
        NodeValue::Code(_) => "code",
        NodeValue::HtmlInline(_) => "html_inline",
        NodeValue::Emph => "emph",
        NodeValue::Strong => "strong",
        NodeValue::Strikethrough => "strikethrough",
        NodeValue::Superscript => "superscript",
        NodeValue::Link(_) => "link",
        NodeValue::Image(_) => "image",
        NodeValue::FootnoteReference(_) => "footnote_reference",
        _ => "unknown",
    }
}

fn base_stylesheet() -> &'static str {
    r#"
html,body{margin:0;padding:0;background:var(--mdv-bg);color:var(--mdv-fg);}
body.mdv{font-family:var(--mdv-font-body);line-height:1.7;font-size:16px;padding:2.5rem 1.25rem;}
.mdv-doc{max-width:46rem;margin:0 auto;}
.mdv-doc h1,.mdv-doc h2,.mdv-doc h3,.mdv-doc h4,.mdv-doc h5,.mdv-doc h6{font-family:var(--mdv-font-heading);line-height:1.25;margin:2.25rem 0 1rem;letter-spacing:-0.01em;}
.mdv-doc h1{font-size:2rem;}
.mdv-doc h2{font-size:1.5rem;}
.mdv-doc h3{font-size:1.25rem;}
.mdv-doc p{margin:0 0 1rem;}
.mdv-doc a{color:var(--mdv-link);text-decoration:none;border-bottom:1px solid color-mix(in srgb,var(--mdv-link) 35%,transparent);}
.mdv-doc a:hover{border-bottom-color:var(--mdv-link);}
.mdv-doc code{font-family:var(--mdv-font-mono);background:var(--mdv-code-bg);padding:0.15em 0.4em;border-radius:var(--mdv-radius-sm);font-size:0.925em;}
.mdv-doc pre{font-family:var(--mdv-font-mono);background:var(--mdv-code-bg);padding:1rem 1.125rem;border-radius:var(--mdv-radius-md);overflow-x:auto;box-shadow:0 1px 2px rgba(16,24,40,0.04),0 1px 3px rgba(16,24,40,0.06);border:1px solid var(--mdv-border);}
.mdv-doc pre code{background:transparent;padding:0;border-radius:0;}
.mdv-doc blockquote{margin:1rem 0;padding:0.5rem 1rem;border-left:3px solid var(--mdv-accent);background:color-mix(in srgb,var(--mdv-accent) 6%,transparent);color:var(--mdv-muted);border-radius:var(--mdv-radius-sm);}
.mdv-doc ul,.mdv-doc ol{padding-left:1.5rem;margin:0 0 1rem;}
.mdv-doc li{margin:0.25rem 0;}
.mdv-doc table{border-collapse:separate;border-spacing:0;width:100%;margin:1rem 0;border:1px solid var(--mdv-border);border-radius:var(--mdv-radius-md);overflow:hidden;box-shadow:0 1px 2px rgba(16,24,40,0.04);}
.mdv-doc th,.mdv-doc td{padding:0.6rem 0.8rem;text-align:left;border-bottom:1px solid var(--mdv-border);}
.mdv-doc tr:last-child td{border-bottom:none;}
.mdv-doc th{background:color-mix(in srgb,var(--mdv-accent) 8%,transparent);font-weight:600;}
.mdv-doc img{max-width:100%;border-radius:var(--mdv-radius-md);}
.mdv-doc hr{border:none;border-top:1px solid var(--mdv-border);margin:2rem 0;}
.mdv-doc .mdv-card{background:var(--mdv-bg);border:1px solid var(--mdv-border);border-radius:var(--mdv-radius-md);padding:1rem 1.25rem;box-shadow:0 1px 2px rgba(16,24,40,0.04);}
"#
}
