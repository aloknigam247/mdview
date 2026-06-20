#![forbid(unsafe_code)]

pub mod _stubs;
pub mod htmlesc;
pub mod image;

pub use _stubs::{
    mdview_theme as theme_mod, Asset, AstNode, Html, HtmlRenderer, Radii, Registry, RenderCtx,
    StyleSpec, Theme, Typography,
};

use comrak::nodes::{NodeLink, NodeValue};
use comrak::{format_html, parse_document, Arena, ComrakOptions};

use crate::image::{
    collect_alt_text, local_to_mdview_url, render_image_html, render_placeholder_html,
    resolve_image_url, ImgResolution,
};

const MDV_MISSING_SENTINEL: &str = "mdv-missing://";

/// Fluent System Icons (Filled, 24px) used for GFM task list items.
const FLUENT_CHECKMARK_CIRCLE_SVG: &str = r#"<svg class="mdv-task-icon mdv-task-checked" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="1em" height="1em" aria-hidden="true" focusable="false"><path d="M12 2C17.5228 2 22 6.47715 22 12C22 17.5228 17.5228 22 12 22C6.47715 22 2 17.5228 2 12C2 6.47715 6.47715 2 12 2ZM16.2929 8.29289L10.5 14.0858L7.70711 11.2929C7.31658 10.9024 6.68342 10.9024 6.29289 11.2929C5.90237 11.6834 5.90237 12.3166 6.29289 12.7071L9.79289 16.2071C10.1834 16.5976 10.8166 16.5976 11.2071 16.2071L17.7071 9.70711C18.0976 9.31658 18.0976 8.68342 17.7071 8.29289C17.3166 7.90237 16.6834 7.90237 16.2929 8.29289Z" fill="currentColor"/></svg>"#;
const FLUENT_CIRCLE_SVG: &str = r#"<svg class="mdv-task-icon mdv-task-unchecked" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="1em" height="1em" aria-hidden="true" focusable="false"><path d="M12 2C17.5228 2 22 6.47715 22 12C22 17.5228 17.5228 22 12 22C6.47715 22 2 17.5228 2 12C2 6.47715 6.47715 2 12 2Z" fill="currentColor"/></svg>"#;

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

fn rewrite_image_urls<'a>(node: &'a AstNode<'a>, ctx: &RenderCtx) {
    let new_link = if let NodeValue::Image(link) = &node.data.borrow().value {
        Some(resolve_image_link(link, ctx, node))
    } else {
        None
    };
    if let Some(updated) = new_link {
        if let NodeValue::Image(link) = &mut node.data.borrow_mut().value {
            *link = updated;
        }
    }
    for child in node.children() {
        rewrite_image_urls(child, ctx);
    }
}

fn resolve_image_link<'a>(link: &NodeLink, ctx: &RenderCtx, node: &'a AstNode<'a>) -> NodeLink {
    let title = link.title.clone();
    match resolve_image_url(&link.url, ctx.source_dir.as_deref()) {
        ImgResolution::Remote(u) | ImgResolution::DataUri(u) => NodeLink { url: u, title },
        ImgResolution::Local(abs) => {
            if !abs.exists() {
                tracing::warn!(path = %abs.display(), "mdview: image not found");
                let alt = collect_alt_text(node);
                let fallback = abs
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                missing_link(&alt, &fallback)
            } else {
                NodeLink {
                    url: local_to_mdview_url(&abs),
                    title,
                }
            }
        }
        ImgResolution::UnresolvableRelative => {
            let alt = collect_alt_text(node);
            missing_link(&alt, &link.url)
        }
    }
}

fn missing_link(alt: &str, fallback: &str) -> NodeLink {
    let label = if !alt.is_empty() { alt } else { fallback };
    NodeLink {
        url: format!("{MDV_MISSING_SENTINEL}{label}"),
        title: String::new(),
    }
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
    rewrite_image_urls(root, ctx);
    let mut out = String::from("<article class=\"mdv-doc\">");
    for child in root.children() {
        if let Some(override_html) = override_for(child, ctx, registry) {
            out.push_str(&override_html);
        } else if let Some(fig) = standalone_image_figure(child) {
            out.push_str(&fig);
        } else {
            let raw = comrak_serialize(child, opts);
            let raw = swap_task_checkboxes(&raw);
            out.push_str(&swap_missing_sentinels(&raw));
        }
    }
    out.push_str("</article>");
    out
}

fn standalone_image_figure<'a>(node: &'a AstNode<'a>) -> Option<String> {
    if !matches!(node.data.borrow().value, NodeValue::Paragraph) {
        return None;
    }
    let mut img_link: Option<NodeLink> = None;
    let mut alt = String::new();
    let mut other_inline = false;
    for child in node.children() {
        match &child.data.borrow().value {
            NodeValue::Image(link) => {
                if img_link.is_some() {
                    other_inline = true;
                    break;
                }
                img_link = Some(link.clone());
                alt = collect_alt_text(child);
            }
            NodeValue::Text(t) if t.trim().is_empty() => {}
            NodeValue::SoftBreak | NodeValue::LineBreak => {}
            _ => {
                other_inline = true;
                break;
            }
        }
    }
    if other_inline {
        return None;
    }
    let link = img_link?;
    if link.url.starts_with(MDV_MISSING_SENTINEL) {
        let label = link.url.trim_start_matches(MDV_MISSING_SENTINEL);
        return Some(render_placeholder_html(label, None));
    }
    Some(render_image_html(&link.url, &alt, &link.title))
}

/// Replace comrak's default GFM task-list `<input type="checkbox" ...>` markers
/// with inline Fluent System Icons (CheckmarkCircle for checked, Circle for
/// unchecked). Scope: only `<input type="checkbox" ...>` markers — bullets and
/// other inputs are untouched.
fn swap_task_checkboxes(html: &str) -> String {
    if !html.contains("type=\"checkbox\"") {
        return html.to_string();
    }
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(idx) = rest.find("<input ") {
        out.push_str(&rest[..idx]);
        let after = &rest[idx..];
        let end = match after.find('>') {
            Some(e) => e + 1,
            None => {
                out.push_str(after);
                rest = "";
                break;
            }
        };
        let tag = &after[..end];
        if tag.contains("type=\"checkbox\"") {
            if tag.contains("checked") {
                out.push_str(FLUENT_CHECKMARK_CIRCLE_SVG);
            } else {
                out.push_str(FLUENT_CIRCLE_SVG);
            }
        } else {
            out.push_str(tag);
        }
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

fn swap_missing_sentinels(html: &str) -> String {
    if !html.contains(MDV_MISSING_SENTINEL) {
        return html.to_string();
    }
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(idx) = rest.find("<img ") {
        out.push_str(&rest[..idx]);
        let after = &rest[idx..];
        let end = match after.find('>') {
            Some(e) => e + 1,
            None => {
                out.push_str(after);
                rest = "";
                break;
            }
        };
        let tag = &after[..end];
        if tag.contains(&format!("src=\"{MDV_MISSING_SENTINEL}")) {
            let label = extract_sentinel_label(tag);
            out.push_str(&render_placeholder_html(&label, None));
        } else {
            out.push_str(tag);
        }
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

fn extract_sentinel_label(tag: &str) -> String {
    let needle = format!("src=\"{MDV_MISSING_SENTINEL}");
    if let Some(start) = tag.find(&needle) {
        let after = &tag[start + needle.len()..];
        if let Some(quote) = after.find('"') {
            return after[..quote].to_string();
        }
    }
    String::from("image")
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

pub fn base_stylesheet() -> &'static str {
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
.mdv-doc figcaption{color:var(--mdv-muted);font-size:0.9em;text-align:center;margin-top:0.25rem;}
.mdv-doc figure{margin:1rem 0;}
.mdv-doc hr{border:none;border-top:1px solid var(--mdv-border);margin:2rem 0;}
.mdv-doc img{max-width:100%;height:auto;border-radius:var(--mdv-radius-md);}
.mdv-img-missing{color:var(--mdv-muted);font-style:italic;}
.mdv-doc .mdv-card{background:var(--mdv-bg);border:1px solid var(--mdv-border);border-radius:var(--mdv-radius-md);padding:1rem 1.25rem;box-shadow:0 1px 2px rgba(16,24,40,0.04);}
.mdv-doc .mdv-task-icon{display:inline-block;vertical-align:-0.15em;margin-right:0.35em;width:1em;height:1em;}
.mdv-doc .mdv-task-checked{color:var(--mdv-accent);}
.mdv-doc .mdv-task-unchecked{color:var(--mdv-muted);}
"#
}
