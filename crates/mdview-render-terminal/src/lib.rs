#![forbid(unsafe_code)]

pub mod _stubs;
pub mod r#box;
pub mod color;
pub mod image;
pub mod inline_html;
pub mod wrap;

use comrak::nodes::{AstNode, ListType, NodeValue};

pub use _stubs::{
    Registry, RenderCtx, StyleSpec, TermChunk, TermChunks, TerminalCaps, TerminalRenderer, Theme,
};

pub fn render<'a>(root: &'a AstNode<'a>, ctx: &RenderCtx, registry: &Registry) -> TermChunks {
    let mut out = TermChunks::new();
    render_node(root, ctx, registry, &mut out, 0);
    out
}

fn render_node<'a>(
    node: &'a AstNode<'a>,
    ctx: &RenderCtx,
    registry: &Registry,
    out: &mut TermChunks,
    depth: usize,
) {
    for ext in registry.terminal_renderers() {
        if let Some(chunks) = ext.render_terminal(node, ctx) {
            out.extend(chunks);
            return;
        }
    }

    let value = node.data.borrow().value.clone();
    match &value {
        NodeValue::Document => {
            for child in node.children() {
                render_node(child, ctx, registry, out, depth);
            }
        }
        NodeValue::Heading(h) => {
            let key = format!("heading.{}", h.level);
            let color = ctx
                .theme
                .color(&key)
                .or_else(|| ctx.theme.color("accent"))
                .unwrap_or("#89b4fa");
            let style = StyleSpec::fg(color).with_bold();
            let bar_style = StyleSpec::fg(color);
            out.push_styled("▌ ", bar_style);
            let inline = render_inline(node, ctx, registry);
            let text = inline.plain_text();
            out.push_styled(text, style);
            out.push_plain("\n\n");
        }
        NodeValue::Paragraph => {
            let inline = render_inline(node, ctx, registry);
            let ansi = inline.to_ansi();
            let width = effective_width(ctx);
            let lines = wrap::wrap_ansi(&ansi, width);
            for line in lines {
                out.push_plain(line);
                out.push_plain("\n");
            }
            out.push_plain("\n");
        }
        NodeValue::List(list) => {
            let mut idx: u32 = list.start as u32;
            let marker_style = ctx.theme.style("muted");
            for child in node.children() {
                let marker = match list.list_type {
                    ListType::Bullet => "• ".to_string(),
                    ListType::Ordered => {
                        let m = format!("{}. ", idx);
                        idx += 1;
                        m
                    }
                };
                render_list_item(child, ctx, registry, out, &marker, &marker_style, depth);
            }
            if depth == 0 {
                out.push_plain("\n");
            }
        }
        NodeValue::Item(_) => {
            for child in node.children() {
                render_node(child, ctx, registry, out, depth + 1);
            }
        }
        NodeValue::TaskItem(state) => {
            let marker = if state.is_some() {
                "\u{F05E1} "
            } else {
                "\u{F0130} "
            };
            out.push_styled(marker, ctx.theme.style("accent"));
            for child in node.children() {
                render_node(child, ctx, registry, out, depth + 1);
            }
        }
        NodeValue::BlockQuote => {
            let muted = ctx.theme.style("blockquote");
            let mut inner = TermChunks::new();
            for child in node.children() {
                render_node(child, ctx, registry, &mut inner, depth + 1);
            }
            let ansi = inner.to_ansi();
            for line in ansi.split('\n') {
                if line.is_empty() {
                    continue;
                }
                out.push_styled("▎ ", muted.clone());
                out.push_plain(line.to_string());
                out.push_plain("\n");
            }
            out.push_plain("\n");
        }
        NodeValue::ThematicBreak => {
            let width = effective_width(ctx);
            let line: String = "─".repeat(width);
            out.push_styled(line, ctx.theme.style("muted"));
            out.push_plain("\n\n");
        }
        NodeValue::CodeBlock(cb) => {
            let lang = if cb.info.is_empty() {
                None
            } else {
                Some(cb.info.as_str())
            };
            let body = cb.literal.trim_end_matches('\n').to_string();
            let frame = r#box::code_frame(lang, &body, &ctx.theme);
            out.extend(frame);
            out.push_plain("\n");
        }
        NodeValue::HtmlBlock(hb) => {
            out.push_styled(hb.literal.clone(), ctx.theme.style("muted"));
            out.push_plain("\n");
        }
        NodeValue::Table(_) => {
            render_table(node, ctx, registry, out);
        }
        NodeValue::Text(t) => {
            out.push_plain(t.clone());
        }
        NodeValue::SoftBreak => {
            out.push_plain(" ");
        }
        NodeValue::LineBreak => {
            out.push_plain("\n");
        }
        NodeValue::Code(c) => {
            out.push_styled(format!(" {} ", c.literal), ctx.theme.style("code.inline"));
        }
        NodeValue::HtmlInline(h) => {
            out.push_styled(h.clone(), ctx.theme.style("muted"));
        }
        NodeValue::Emph => {
            let style = ctx.theme.style("emph").with_italic();
            let inline = render_inline(node, ctx, registry);
            out.push_styled(inline.plain_text(), style);
        }
        NodeValue::Strong => {
            let style = ctx.theme.style("strong").with_bold();
            let inline = render_inline(node, ctx, registry);
            out.push_styled(inline.plain_text(), style);
        }
        NodeValue::Strikethrough => {
            let inline = render_inline(node, ctx, registry);
            out.push_styled(inline.plain_text(), ctx.theme.style("muted"));
        }
        NodeValue::Link(link) => {
            let style = ctx.theme.style("link").with_underline();
            let inline = render_inline(node, ctx, registry);
            let text = inline.plain_text();
            out.push_styled(format!("{} ({})", text, link.url), style);
        }
        NodeValue::Image(link) => {
            render_image(node, link.url.as_str(), ctx, out);
        }
        NodeValue::FootnoteReference(fr) => {
            out.push_styled(format!("[^{}]", fr.name), ctx.theme.style("muted"));
        }
        NodeValue::FootnoteDefinition(fd) => {
            let muted = ctx.theme.style("muted");
            out.push_styled(format!("[^{}]: ", fd.name), muted);
            for child in node.children() {
                render_node(child, ctx, registry, out, depth + 1);
            }
        }
        _ => {
            for child in node.children() {
                render_node(child, ctx, registry, out, depth);
            }
        }
    }
}

fn render_inline<'a>(parent: &'a AstNode<'a>, ctx: &RenderCtx, registry: &Registry) -> TermChunks {
    let mut out = TermChunks::new();
    let children: Vec<&'a AstNode<'a>> = parent.children().collect();
    let mut i = 0;
    while i < children.len() {
        let child = children[i];
        if let Some((tag, end)) = match_sub_sup_run(&children, i) {
            let inner = collect_text(&children[i + 1..end]);
            let mapped = match tag {
                inline_html::HtmlTag::SubOpen => inline_html::unicode_sub(&inner),
                inline_html::HtmlTag::SupOpen => inline_html::unicode_sup(&inner),
                _ => inner,
            };
            out.push_plain(mapped);
            i = end + 1;
            continue;
        }
        render_node(child, ctx, registry, &mut out, 0);
        i += 1;
    }
    out
}

fn match_sub_sup_run<'a>(
    children: &[&'a AstNode<'a>],
    start: usize,
) -> Option<(inline_html::HtmlTag, usize)> {
    let open = children.get(start)?;
    let open_tag = match &open.data.borrow().value {
        NodeValue::HtmlInline(h) => inline_html::classify(h),
        _ => return None,
    };
    let (open_kind, close_kind) = match open_tag {
        inline_html::HtmlTag::SubOpen => (
            inline_html::HtmlTag::SubOpen,
            inline_html::HtmlTag::SubClose,
        ),
        inline_html::HtmlTag::SupOpen => (
            inline_html::HtmlTag::SupOpen,
            inline_html::HtmlTag::SupClose,
        ),
        _ => return None,
    };
    for (j, n) in children.iter().enumerate().skip(start + 1) {
        match &n.data.borrow().value {
            NodeValue::HtmlInline(h) => {
                if inline_html::classify(h) == close_kind {
                    return Some((open_kind, j));
                }
            }
            NodeValue::Text(_) | NodeValue::Code(_) => continue,
            _ => return None,
        }
    }
    None
}

fn collect_text<'a>(nodes: &[&'a AstNode<'a>]) -> String {
    let mut s = String::new();
    for n in nodes {
        match &n.data.borrow().value {
            NodeValue::Text(t) => s.push_str(t),
            NodeValue::Code(c) => s.push_str(&c.literal),
            _ => {}
        }
    }
    s
}

fn render_list_item<'a>(
    item: &'a AstNode<'a>,
    ctx: &RenderCtx,
    registry: &Registry,
    out: &mut TermChunks,
    marker: &str,
    marker_style: &StyleSpec,
    depth: usize,
) {
    if depth > 0 {
        out.push_plain("  ".repeat(depth));
    }

    if let NodeValue::TaskItem(state) = &item.data.borrow().value {
        let m = if state.is_some() {
            "\u{F05E1} "
        } else {
            "\u{F0130} "
        };
        out.push_styled(m.to_string(), ctx.theme.style("accent"));
    } else {
        out.push_styled(marker.to_string(), marker_style.clone());
    }

    let mut child_out = TermChunks::new();
    for child in item.children() {
        render_node(child, ctx, registry, &mut child_out, depth + 1);
    }
    let rendered = child_out.to_ansi();
    let trimmed = rendered.trim_end_matches('\n');
    out.push_plain(trimmed.to_string());
    out.push_plain("\n");
}

fn render_table<'a>(
    node: &'a AstNode<'a>,
    ctx: &RenderCtx,
    registry: &Registry,
    out: &mut TermChunks,
) {
    let mut headers: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut is_header = true;

    for row_node in node.children() {
        if !matches!(row_node.data.borrow().value, NodeValue::TableRow(_)) {
            continue;
        }
        let mut cells: Vec<String> = Vec::new();
        for cell in row_node.children() {
            if !matches!(cell.data.borrow().value, NodeValue::TableCell) {
                continue;
            }
            let inline = render_inline(cell, ctx, registry);
            cells.push(inline.plain_text());
        }
        if is_header {
            headers = cells;
            is_header = false;
        } else {
            rows.push(cells);
        }
    }

    out.extend(r#box::table(&headers, &rows, &ctx.theme));
    out.push_plain("\n");
}

fn render_image<'a>(node: &'a AstNode<'a>, url: &str, ctx: &RenderCtx, out: &mut TermChunks) {
    let alt = image::collect_alt_text(node);
    let muted = ctx.theme.style("muted");
    match image::resolve(url, ctx.source_dir.as_deref()) {
        image::ImgResolution::Remote => {
            let label = image::placeholder_label(&alt, url, None);
            out.push_styled(label, muted);
        }
        image::ImgResolution::Unresolvable => {
            tracing::warn!(url, "mdview: image path unresolvable (no source_dir)");
            let label = image::placeholder_label(&alt, url, None);
            out.push_styled(label, muted);
        }
        image::ImgResolution::Local(path) => {
            if !path.exists() {
                tracing::warn!(path = %path.display(), "mdview: image not found");
                let label = image::placeholder_label(&alt, url, Some(&path));
                out.push_styled(label, muted);
                return;
            }
            let sixel_supported = ctx.terminal_caps.sixel;
            if let Some(sixel) = image::encode_local_to_sixel(&path, sixel_supported) {
                out.push_plain(sixel);
                out.push_plain("\n");
            } else {
                let label = image::placeholder_label(&alt, url, Some(&path));
                out.push_styled(label, muted);
            }
        }
    }
}

fn effective_width(ctx: &RenderCtx) -> usize {
    let w = ctx.terminal_caps.width;
    if w == 0 {
        100
    } else {
        w
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("render error: {0}")]
    Other(String),
}

pub fn render_str(src: &str, ctx: &RenderCtx, registry: &Registry) -> String {
    let arena = comrak::Arena::new();
    let mut opts = comrak::ComrakOptions::default();
    opts.extension.table = true;
    opts.extension.tasklist = true;
    opts.extension.strikethrough = true;
    opts.extension.autolink = true;
    opts.extension.footnotes = true;
    let root = comrak::parse_document(&arena, src, &opts);
    let chunks = render(root, ctx, registry);
    chunks.to_ansi_with(ctx.terminal_caps.truecolor)
}
