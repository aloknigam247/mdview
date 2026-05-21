use anyhow::Result;
use comrak::nodes::{AstNode, ListType, NodeValue, TableAlignment};
use comrak::Arena;
use mdview_core::{parse, Registry, RenderCtx, TermChunks, Theme};
use std::path::Path;

use crate::builtins::builtin_extensions;

#[allow(dead_code)]
pub fn render_ansi(src: &str) -> Result<String> {
    render_ansi_with_source(src, None)
}

pub fn render_ansi_with_source(src: &str, source_dir: Option<&Path>) -> Result<String> {
    let mut registry = Registry::new();
    for ext in builtin_extensions() {
        registry.register(ext);
    }

    let mut src_owned = src.to_string();
    registry.apply_pre_parse(&mut src_owned);

    let arena = Arena::new();
    let ast = parse(&arena, &src_owned, &registry);
    registry.apply_transforms(ast);

    let theme = Theme::default();
    let mut ctx = RenderCtx::new(&theme);
    ctx.source_dir = source_dir.map(|p| p.to_path_buf());

    let mut out = String::new();
    for ext in registry.terminal_renderers() {
        if let Some(chunks) = ext.pre_render_terminal(&ctx) {
            write_chunks(&chunks, &mut out);
        }
    }
    for child in ast.children() {
        render_block(child, &ctx, &registry, &mut out, 0);
        out.push('\n');
    }
    Ok(out)
}

fn render_block<'a>(
    node: &'a AstNode<'a>,
    ctx: &RenderCtx<'_>,
    registry: &Registry,
    out: &mut String,
    depth: usize,
) {
    // Code blocks always get the rounded frame; the extension (if any) only
    // colors the interior. Every other node type is handled by the
    // extension dispatch first, then the built-in block formatter.
    if matches!(node.data.borrow().value, NodeValue::CodeBlock(_)) {
        render_code_block(node, ctx, registry, out);
        return;
    }
    for ext in registry.terminal_renderers() {
        if let Some(chunks) = ext.render_terminal(node, ctx) {
            write_chunks(&chunks, out);
            return;
        }
    }

    let data = node.data.borrow();
    match &data.value {
        NodeValue::Document => {
            for c in node.children() {
                render_block(c, ctx, registry, out, depth);
            }
        }
        NodeValue::Heading(h) => {
            let prefix = "#".repeat(h.level as usize);
            out.push_str(BOLD_ACCENT);
            out.push_str(&prefix);
            out.push(' ');
            render_inlines(node, out);
            out.push_str(RESET);
            out.push('\n');
        }
        NodeValue::Paragraph => {
            render_inlines(node, out);
            out.push('\n');
        }
        NodeValue::BlockQuote => {
            let mut inner = String::new();
            for c in node.children() {
                render_block(c, ctx, registry, &mut inner, depth);
            }
            for line in inner.trim_end().split('\n') {
                out.push_str(ACCENT);
                out.push_str("▎ ");
                out.push_str(RESET);
                out.push_str(MUTED);
                out.push_str(line);
                out.push_str(RESET);
                out.push('\n');
            }
        }
        NodeValue::List(_) => {
            let children: Vec<_> = node.children().collect();
            let total = children.len();
            for (i, c) in children.into_iter().enumerate() {
                render_list_item(c, ctx, registry, out, depth, i + 1, total);
            }
        }
        NodeValue::ThematicBreak => {
            out.push_str(MUTED);
            out.push_str(&"─".repeat(60));
            out.push_str(RESET);
            out.push('\n');
        }
        NodeValue::Table(_) => render_table(node, out),
        NodeValue::HtmlBlock(h) => {
            out.push_str(MUTED);
            out.push_str(h.literal.trim_end());
            out.push_str(RESET);
            out.push('\n');
        }
        NodeValue::FootnoteDefinition(def) => {
            out.push_str(MUTED);
            out.push_str(&format!("[{}]: ", def.name));
            let mut inner = String::new();
            for c in node.children() {
                render_block(c, ctx, registry, &mut inner, depth);
            }
            out.push_str(inner.trim_end());
            out.push_str(RESET);
            out.push('\n');
        }
        _ => {
            // Fallback: try inlines, then children
            render_inlines(node, out);
            if node.children().next().is_some() {
                for c in node.children() {
                    render_block(c, ctx, registry, out, depth);
                }
            }
        }
    }
}

fn render_code_block<'a>(
    node: &'a AstNode<'a>,
    ctx: &RenderCtx<'_>,
    registry: &Registry,
    out: &mut String,
) {
    let data = node.data.borrow();
    let NodeValue::CodeBlock(cb) = &data.value else {
        return;
    };
    let lang = cb.info.split_whitespace().next().unwrap_or("").to_string();
    let literal = cb.literal.clone();
    drop(data);

    // Try to get colored content from an extension (e.g. highlight for known
    // languages, math for ```math fences). Fall back to the raw literal.
    let body = registry
        .terminal_renderers()
        .find_map(|ext| ext.render_terminal(node, ctx))
        .map(|chunks| chunks.into_iter().map(|c| c.text).collect::<String>())
        .unwrap_or(literal);

    let width = 60usize;
    let label = if lang.is_empty() {
        "".to_string()
    } else {
        format!("─ {} ", lang)
    };
    let top = format!(
        "╭{label}{}╮",
        "─".repeat(width.saturating_sub(label.chars().count() + 2))
    );
    let bot = format!("╰{}╯", "─".repeat(width.saturating_sub(2)));

    out.push_str(MUTED);
    out.push_str(&top);
    out.push_str(RESET);
    out.push('\n');
    for line in body.split('\n') {
        if line.is_empty() && body.ends_with('\n') {
            continue;
        }
        out.push_str(MUTED);
        out.push_str("│ ");
        out.push_str(RESET);
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(MUTED);
    out.push_str(&bot);
    out.push_str(RESET);
    out.push('\n');
}

fn render_list_item<'a>(
    node: &'a AstNode<'a>,
    ctx: &RenderCtx<'_>,
    registry: &Registry,
    out: &mut String,
    depth: usize,
    index: usize,
    _total: usize,
) {
    let indent = "  ".repeat(depth);
    let data = node.data.borrow();
    let marker = match &data.value {
        NodeValue::Item(item) => match item.list_type {
            ListType::Bullet => match depth % 3 {
                0 => "•".to_string(),
                1 => "◦".to_string(),
                _ => "▪".to_string(),
            },
            ListType::Ordered => format!("{}.", index),
        },
        NodeValue::TaskItem(Some(ch)) if *ch != ' ' => "☒".to_string(),
        NodeValue::TaskItem(_) => "☐".to_string(),
        _ => "•".to_string(),
    };
    drop(data);

    let mut first_line = true;
    let mut inner = String::new();
    for c in node.children() {
        render_block(c, ctx, registry, &mut inner, depth + 1);
    }
    for line in inner.trim_end().split('\n') {
        if first_line {
            out.push_str(&indent);
            out.push_str(ACCENT);
            out.push_str(&marker);
            out.push(' ');
            out.push_str(RESET);
            out.push_str(line);
            first_line = false;
        } else {
            out.push_str(&indent);
            out.push_str("  ");
            out.push_str(line);
        }
        out.push('\n');
    }
}

fn render_table<'a>(node: &'a AstNode<'a>, out: &mut String) {
    let alignments: Vec<TableAlignment> = match &node.data.borrow().value {
        NodeValue::Table(t) => t.alignments.clone(),
        _ => Vec::new(),
    };
    let rows: Vec<Vec<String>> = node
        .children()
        .map(|row| {
            row.children()
                .map(|cell| {
                    let mut s = String::new();
                    render_inlines(cell, &mut s);
                    s
                })
                .collect()
        })
        .collect();
    if rows.is_empty() {
        return;
    }
    let cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut widths = vec![0usize; cols];
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(visible_width(cell));
        }
    }
    let top = corners("╭", "┬", "╮", &widths);
    let mid = corners("├", "┼", "┤", &widths);
    let bot = corners("╰", "┴", "╯", &widths);
    out.push_str(MUTED);
    out.push_str(&top);
    out.push_str(RESET);
    out.push('\n');
    for (r, row) in rows.iter().enumerate() {
        out.push_str(MUTED);
        out.push_str("│");
        out.push_str(RESET);
        for (i, cell) in row.iter().enumerate() {
            let target = widths[i];
            let gap = target.saturating_sub(visible_width(cell));
            let align = alignments.get(i).copied().unwrap_or(TableAlignment::None);
            let (left_pad, right_pad) = match align {
                TableAlignment::Center => (gap / 2, gap - gap / 2),
                TableAlignment::Right => (gap, 0),
                _ => (0, gap),
            };
            out.push(' ');
            for _ in 0..left_pad {
                out.push(' ');
            }
            if r == 0 {
                out.push_str(BOLD);
                out.push_str(cell);
                out.push_str(RESET);
            } else {
                out.push_str(cell);
            }
            for _ in 0..right_pad {
                out.push(' ');
            }
            out.push(' ');
            out.push_str(MUTED);
            out.push_str("│");
            out.push_str(RESET);
        }
        out.push('\n');
        if r == 0 {
            out.push_str(MUTED);
            out.push_str(&mid);
            out.push_str(RESET);
            out.push('\n');
        }
    }
    out.push_str(MUTED);
    out.push_str(&bot);
    out.push_str(RESET);
    out.push('\n');
}

fn corners(left: &str, sep: &str, right: &str, widths: &[usize]) -> String {
    let mut s = String::from(left);
    for (i, w) in widths.iter().enumerate() {
        s.push_str(&"─".repeat(w + 2));
        if i + 1 < widths.len() {
            s.push_str(sep);
        }
    }
    s.push_str(right);
    s
}

fn render_inlines<'a>(node: &'a AstNode<'a>, out: &mut String) {
    for c in node.children() {
        render_inline(c, out);
    }
}

fn render_inline<'a>(node: &'a AstNode<'a>, out: &mut String) {
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Text(t) => out.push_str(t),
        NodeValue::SoftBreak | NodeValue::LineBreak => out.push('\n'),
        NodeValue::Code(c) => {
            out.push_str(CODE_BG);
            out.push(' ');
            out.push_str(&c.literal);
            out.push(' ');
            out.push_str(RESET);
        }
        NodeValue::Emph => {
            out.push_str(ITALIC);
            drop(data);
            render_inlines(node, out);
            out.push_str(RESET);
        }
        NodeValue::Strong => {
            out.push_str(BOLD);
            drop(data);
            render_inlines(node, out);
            out.push_str(RESET);
        }
        NodeValue::Strikethrough => {
            out.push_str(STRIKE);
            drop(data);
            render_inlines(node, out);
            out.push_str(RESET);
        }
        NodeValue::Link(l) => {
            let url = l.url.clone();
            drop(data);
            out.push_str(UNDERLINE);
            out.push_str(LINK);
            render_inlines(node, out);
            out.push_str(RESET);
            out.push_str(MUTED);
            out.push_str(" (");
            out.push_str(&url);
            out.push(')');
            out.push_str(RESET);
        }
        NodeValue::Image(i) => {
            let url = i.url.clone();
            drop(data);
            let alt = collect_text_from_children(node);
            let display = if alt.is_empty() { url.as_str() } else { alt.as_str() };
            out.push_str(MUTED);
            out.push_str("[image: ");
            out.push_str(display);
            out.push(']');
            out.push_str(RESET);
        }
        NodeValue::Math(m) => {
            out.push_str(ACCENT);
            out.push_str(&m.literal);
            out.push_str(RESET);
        }
        NodeValue::HtmlInline(h) => out.push_str(h),
        NodeValue::FootnoteReference(r) => {
            out.push_str(MUTED);
            out.push_str("[^");
            out.push_str(&r.name);
            out.push(']');
            out.push_str(RESET);
        }
        _ => {
            drop(data);
            render_inlines(node, out);
        }
    }
}

fn collect_text_from_children<'a>(node: &'a AstNode<'a>) -> String {
    let mut out = String::new();
    for c in node.children() {
        if let NodeValue::Text(t) = &c.data.borrow().value {
            out.push_str(t);
        }
    }
    out
}

fn write_chunks(chunks: &TermChunks, out: &mut String) {
    for c in chunks {
        out.push_str(&c.text);
    }
    out.push('\n');
}

fn visible_width(s: &str) -> usize {
    let mut in_esc = false;
    let mut w = 0usize;
    for ch in s.chars() {
        if in_esc {
            if ch == 'm' {
                in_esc = false;
            }
            continue;
        }
        if ch == '\x1b' {
            in_esc = true;
            continue;
        }
        w += 1;
    }
    w
}

// ANSI SGR sequences (truecolor + attr combos).
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const ITALIC: &str = "\x1b[3m";
const UNDERLINE: &str = "\x1b[4m";
const STRIKE: &str = "\x1b[9m";
const ACCENT: &str = "\x1b[38;2;108;124;255m"; // #6c7cff
const MUTED: &str = "\x1b[38;2;107;114;128m"; // #6b7280
const LINK: &str = "\x1b[38;2;37;99;235m"; // #2563eb
const CODE_BG: &str = "\x1b[48;2;246;248;250m\x1b[38;2;17;24;39m"; // #f6f8fa bg + dark fg
const BOLD_ACCENT: &str = "\x1b[1m\x1b[38;2;108;124;255m";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_is_bold_accented() {
        let out = render_ansi("# Hello\n").unwrap();
        assert!(out.contains("Hello"));
        assert!(out.contains("\x1b[1m"));
    }

    #[test]
    fn blockquote_has_bar_prefix() {
        let out = render_ansi("> quoted\n").unwrap();
        assert!(out.contains('▎'));
    }

    #[test]
    fn rounded_table_borders() {
        let out = render_ansi("| a | b |\n|---|---|\n| 1 | 2 |\n").unwrap();
        assert!(out.contains('╭'));
        assert!(out.contains('╮'));
        assert!(out.contains('╰'));
        assert!(out.contains('╯'));
    }

    #[test]
    fn task_list_markers() {
        let out = render_ansi("- [x] done\n- [ ] todo\n").unwrap();
        assert!(out.contains("☒"));
        assert!(out.contains("☐"));
    }

    #[test]
    fn image_renders_alt_text_placeholder() {
        let out = render_ansi("![cover](./x.png)\n").unwrap();
        assert!(out.contains("[image: cover]"), "got: {out}");
    }

    #[test]
    fn fenced_code_has_rounded_frame() {
        let out = render_ansi("```rust\nfn main() {}\n```\n").unwrap();
        assert!(out.contains("╭"));
        assert!(out.contains("rust"));
        assert!(out.contains("╯"));
    }
}
