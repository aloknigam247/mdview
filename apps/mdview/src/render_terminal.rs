use anyhow::Result;
use comrak::nodes::{AstNode, ListType, NodeValue, TableAlignment};
use comrak::Arena;
use mdview_core::{parse, Registry, RenderCtx, TermChunks, TerminalCaps, Theme};
use mdview_ext_highlight::Highlight;
use std::path::Path;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;

use crate::builtins::builtin_extensions;

#[allow(dead_code)]
pub fn render_ansi(src: &str) -> Result<String> {
    render_ansi_with_source(src, None, 4)
}

pub fn render_ansi_with_source(
    src: &str,
    source_dir: Option<&Path>,
    tab_width: u8,
) -> Result<String> {
    render_ansi_with_source_and_width(src, source_dir, tab_width, terminal_width())
}

fn render_ansi_with_source_and_width(
    src: &str,
    source_dir: Option<&Path>,
    tab_width: u8,
    width: usize,
) -> Result<String> {
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
    ctx.tab_width = tab_width;
    ctx.terminal_caps = Some(TerminalCaps {
        width: width.min(u16::MAX as usize) as u16,
        ..TerminalCaps::default()
    });

    let mut out = String::new();
    for ext in registry.terminal_renderers() {
        if let Some(chunks) = ext.pre_render_terminal(&ctx) {
            write_chunks(&chunks, &mut out);
        }
    }
    for child in ast.children() {
        render_block(child, &ctx, &registry, &mut out, 0, tab_width);
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
    tab_width: u8,
) {
    // Code blocks always get the rounded frame; the extension (if any) only
    // colors the interior. Every other node type is handled by the
    // extension dispatch first, then the built-in block formatter.
    if matches!(node.data.borrow().value, NodeValue::CodeBlock(_)) {
        render_code_block(node, ctx, registry, out, tab_width);
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
                render_block(c, ctx, registry, out, depth, tab_width);
            }
        }
        NodeValue::Heading(h) => {
            let prefix = "#".repeat(h.level as usize);
            out.push_str(BOLD_ACCENT);
            out.push_str(&prefix);
            out.push(' ');
            render_inlines(node, None, out);
            out.push_str(RESET);
            out.push('\n');
        }
        NodeValue::Paragraph => {
            render_inlines(node, Some(ctx), out);
            out.push('\n');
        }
        NodeValue::BlockQuote => {
            let mut inner = String::new();
            for c in node.children() {
                render_block(c, ctx, registry, &mut inner, depth, tab_width);
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
                render_list_item(c, ctx, registry, out, depth, i + 1, total, tab_width);
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
                render_block(c, ctx, registry, &mut inner, depth, tab_width);
            }
            out.push_str(inner.trim_end());
            out.push_str(RESET);
            out.push('\n');
        }
        _ => {
            // Fallback: try inlines, then children
            render_inlines(node, None, out);
            if node.children().next().is_some() {
                for c in node.children() {
                    render_block(c, ctx, registry, out, depth, tab_width);
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
    tab_width: u8,
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

    let tab_spaces: String = " ".repeat(tab_width as usize);
    let body = body.replace('\t', &tab_spaces);

    let outer_width = ctx
        .terminal_caps
        .map(|caps| caps.width as usize)
        .unwrap_or(DEFAULT_TERMINAL_WIDTH)
        .max(MIN_CODE_FRAME_WIDTH);
    let inner_width = outer_width.saturating_sub(4).max(1);
    let label = code_frame_label(&lang, outer_width);
    let label_width = code_visible_width(&label);
    let top = format!(
        "╭{label}{}╮",
        "─".repeat(outer_width.saturating_sub(label_width + 2))
    );
    let bot = format!("╰{}╯", "─".repeat(outer_width.saturating_sub(2)));

    out.push_str(MUTED);
    out.push_str(&top);
    out.push_str(RESET);
    out.push('\n');
    for line in code_body_lines(&body) {
        for wrapped in wrap_ansi_code_line(line, inner_width) {
            out.push_str(MUTED);
            out.push_str("│ ");
            out.push_str(RESET);
            out.push_str(&wrapped);
            out.push_str(RESET);
            out.push_str(&" ".repeat(inner_width.saturating_sub(code_visible_width(&wrapped))));
            out.push_str(MUTED);
            out.push_str(" │");
            out.push_str(RESET);
            out.push('\n');
        }
    }
    out.push_str(MUTED);
    out.push_str(&bot);
    out.push_str(RESET);
    out.push('\n');
}

fn code_body_lines(body: &str) -> Vec<&str> {
    let mut lines: Vec<_> = body.split('\n').collect();
    while matches!(lines.last(), Some(line) if line.is_empty() || code_visible_width(line) == 0) {
        lines.pop();
    }
    if lines.is_empty() {
        lines.push("");
    }
    lines
}

fn code_frame_label(lang: &str, outer_width: usize) -> String {
    if lang.is_empty() {
        return String::new();
    }
    let label = format!("─ {lang} ");
    if code_visible_width(&label) <= outer_width.saturating_sub(2) {
        label
    } else {
        String::new()
    }
}

fn terminal_width() -> usize {
    crossterm::terminal::size()
        .map(|(width, _)| width as usize)
        .unwrap_or(DEFAULT_TERMINAL_WIDTH)
}

fn code_visible_width(s: &str) -> usize {
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

fn update_active_sgr(active: &mut String, sequence: &str) {
    if !sequence.ends_with('m') {
        return;
    }
    if sequence == RESET || sequence.contains("[0") {
        active.clear();
    } else {
        active.push_str(sequence);
    }
}

fn wrap_ansi_code_line(line: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut active_sgr = String::new();
    let mut chars = line.chars().peekable();
    let mut current = String::new();
    let mut current_width = 0usize;
    let mut rows = Vec::new();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            let sequence = read_ansi_sequence(&mut chars);
            current.push('\x1b');
            current.push_str(&sequence);
            update_active_sgr(&mut active_sgr, &format!("\x1b{sequence}"));
            continue;
        }

        let char_width = ch.width().unwrap_or(0);
        if char_width > 0 && current_width > 0 && current_width + char_width > width {
            rows.push(current);
            current = active_sgr.clone();
            current_width = 0;
        }
        current.push(ch);
        current_width += char_width;
    }

    rows.push(current);
    rows
}

fn read_ansi_sequence<I>(chars: &mut std::iter::Peekable<I>) -> String
where
    I: Iterator<Item = char>,
{
    let mut sequence = String::new();
    if chars.peek() != Some(&'[') {
        return sequence;
    }
    for ch in chars.by_ref() {
        sequence.push(ch);
        if ch.is_ascii_alphabetic() {
            break;
        }
    }
    sequence
}

#[allow(clippy::too_many_arguments)]
fn render_list_item<'a>(
    node: &'a AstNode<'a>,
    ctx: &RenderCtx<'_>,
    registry: &Registry,
    out: &mut String,
    depth: usize,
    index: usize,
    _total: usize,
    tab_width: u8,
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
        NodeValue::TaskItem(Some(ch)) if *ch != ' ' => "\u{F05E1}".to_string(),
        NodeValue::TaskItem(_) => "\u{F0130}".to_string(),
        _ => "•".to_string(),
    };
    drop(data);

    let mut first_line = true;
    let mut inner = String::new();
    for c in node.children() {
        render_block(c, ctx, registry, &mut inner, depth + 1, tab_width);
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
                    render_inlines(cell, None, &mut s);
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
        out.push('│');
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
            out.push('│');
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

fn render_inlines<'a>(node: &'a AstNode<'a>, ctx: Option<&RenderCtx<'_>>, out: &mut String) {
    for c in node.children() {
        render_inline(c, ctx, out);
    }
}

fn render_inline<'a>(node: &'a AstNode<'a>, ctx: Option<&RenderCtx<'_>>, out: &mut String) {
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Text(t) => out.push_str(t),
        NodeValue::SoftBreak | NodeValue::LineBreak => out.push('\n'),
        NodeValue::Code(c) => {
            let highlighted =
                ctx.and_then(|ctx| Highlight::render_inline_terminal_literal(&c.literal, ctx));
            out.push_str(CODE_BG);
            out.push(' ');
            match highlighted {
                Some(ansi) => out.push_str(&ansi),
                None => out.push_str(&c.literal),
            }
            out.push(' ');
            out.push_str(RESET);
        }
        NodeValue::Emph => {
            out.push_str(ITALIC);
            drop(data);
            render_inlines(node, ctx, out);
            out.push_str(RESET);
        }
        NodeValue::Strong => {
            out.push_str(BOLD);
            drop(data);
            render_inlines(node, ctx, out);
            out.push_str(RESET);
        }
        NodeValue::Strikethrough => {
            out.push_str(STRIKE);
            drop(data);
            render_inlines(node, ctx, out);
            out.push_str(RESET);
        }
        NodeValue::Link(l) => {
            let url = l.url.clone();
            drop(data);
            out.push_str(UNDERLINE);
            out.push_str(LINK);
            render_inlines(node, None, out);
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
            let display = if alt.is_empty() {
                url.as_str()
            } else {
                alt.as_str()
            };
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
            render_inlines(node, ctx, out);
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
    let mut visible = String::with_capacity(s.len());
    for ch in s.chars() {
        if in_esc {
            if ch.is_ascii_alphabetic() {
                in_esc = false;
            }
            continue;
        }
        if ch == '\x1b' {
            in_esc = true;
            continue;
        }
        visible.push(ch);
    }
    visible
        .graphemes(true)
        .map(|g| {
            if g.chars()
                .any(|c| UnicodeWidthChar::width(c).unwrap_or(0) >= 2)
            {
                2
            } else {
                g.chars()
                    .map(|c| UnicodeWidthChar::width(c).unwrap_or(0))
                    .sum()
            }
        })
        .sum()
}

const DEFAULT_TERMINAL_WIDTH: usize = 80;
const MIN_CODE_FRAME_WIDTH: usize = 12;

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

    fn strip_ansi(s: &str) -> String {
        let mut out = String::new();
        let mut in_esc = false;
        for ch in s.chars() {
            if in_esc {
                if ch.is_ascii_alphabetic() {
                    in_esc = false;
                }
                continue;
            }
            if ch == '\x1b' {
                in_esc = true;
                continue;
            }
            out.push(ch);
        }
        out
    }

    #[test]
    fn inline_code_hashbang_highlights_in_terminal() {
        let out = render_ansi("Use `#!rust let x = 1;` now.\n").unwrap();
        assert!(
            out.contains("\x1b[38;2;"),
            "expected syntect truecolor ANSI: {out:?}"
        );
        assert!(strip_ansi(&out).contains("let x = 1;"), "{out:?}");
        assert!(!out.contains("#!rust"), "{out:?}");
    }

    #[test]
    fn inline_code_without_hashbang_keeps_existing_terminal_style() {
        let out = render_ansi("Use `let x = 1;` now.\n").unwrap();
        assert!(out.contains(CODE_BG), "{out:?}");
        assert!(out.contains("let x = 1;"), "{out:?}");
    }

    #[test]
    fn inline_code_in_link_text_not_highlighted_terminal() {
        let out = render_ansi("See [`#!rust let x = 1;`](https://e.x) now.\n").unwrap();
        assert!(out.contains("#!rust let x = 1;"), "{out:?}");
    }

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
    fn table_aligns_borders_for_wide_and_zwj_cells() {
        let src = "| a | b |\n|---|---|\n| 🦀 | 中 |\n| x | 👨‍👩‍👧‍👦 |\n";
        let out = render_ansi(src).unwrap();
        let lines: Vec<String> = out
            .lines()
            .map(strip_ansi_for_test)
            .filter(|l| l.contains('╭') || l.contains('│') || l.contains('├') || l.contains('╰'))
            .collect();
        assert_eq!(lines.len(), 6, "expected 6 table lines: {lines:#?}");

        let expected_w = display_width_for_test(&lines[0]);
        assert!(
            lines
                .iter()
                .all(|l| display_width_for_test(l) == expected_w),
            "all table lines must share one display width: {lines:#?}"
        );

        assert_eq!(border_columns_for_test(&lines[0], '╭'), vec![0]);
        assert_eq!(border_columns_for_test(&lines[0], '┬'), vec![5]);
        assert_eq!(border_columns_for_test(&lines[0], '╮'), vec![10]);
        assert_eq!(border_columns_for_test(&lines[1], '│'), vec![0, 5, 10]);
        assert_eq!(border_columns_for_test(&lines[2], '├'), vec![0]);
        assert_eq!(border_columns_for_test(&lines[2], '┼'), vec![5]);
        assert_eq!(border_columns_for_test(&lines[2], '┤'), vec![10]);
        assert_eq!(border_columns_for_test(&lines[3], '│'), vec![0, 5, 10]);
        assert_eq!(border_columns_for_test(&lines[4], '│'), vec![0, 5, 10]);
        assert_eq!(border_columns_for_test(&lines[5], '╰'), vec![0]);
        assert_eq!(border_columns_for_test(&lines[5], '┴'), vec![5]);
        assert_eq!(border_columns_for_test(&lines[5], '╯'), vec![10]);
    }

    #[test]
    fn task_list_markers() {
        let out = render_ansi("- [x] done\n- [ ] todo\n").unwrap();
        assert!(
            out.contains('\u{F05E1}'),
            "checked nerd-font icon U+F05E1 missing: {out:?}"
        );
        assert!(
            out.contains('\u{F0130}'),
            "unchecked nerd-font icon U+F0130 missing: {out:?}"
        );
        assert!(
            !out.contains('☒') && !out.contains('☐'),
            "legacy ballot box glyphs leaked: {out:?}"
        );
    }

    #[test]
    fn plain_bullet_list_not_decorated_with_task_icons() {
        let out = render_ansi("- alpha\n- beta\n").unwrap();
        assert!(out.contains('•'), "bullet missing: {out:?}");
        assert!(
            !out.contains('\u{F05E1}') && !out.contains('\u{F0130}'),
            "task icons should not appear for plain bullets: {out:?}"
        );
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

    #[test]
    fn fenced_code_rows_have_right_border_at_fixed_width() {
        let width = 24usize;
        let out = render_ansi_with_source_and_width(
            "```rust\nlet message = \"abcdefghijklmnopqrstuvwxyz\";\n```\n",
            None,
            4,
            width,
        )
        .unwrap();
        let rows = code_frame_rows(&out);

        assert!(rows.len() > 3, "long code line should wrap: {out:?}");
        for (idx, row) in rows.iter().enumerate() {
            assert_eq!(code_visible_width(row), width, "row {idx} width: {row:?}");
            let visible = strip_ansi_for_test(row);
            let first = visible.chars().next().unwrap();
            let last = visible.chars().last().unwrap();
            match idx {
                0 => assert_eq!((first, last), ('╭', '╮'), "top row: {visible:?}"),
                n if n + 1 == rows.len() => {
                    assert_eq!((first, last), ('╰', '╯'), "bottom row: {visible:?}");
                }
                _ => assert_eq!((first, last), ('│', '│'), "body row: {visible:?}"),
            }
        }
    }

    #[test]
    fn fenced_code_soft_wraps_unbroken_tokens_inside_border() {
        let width = 20usize;
        let out = render_ansi_with_source_and_width(
            "```\nabcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ\n```\n",
            None,
            4,
            width,
        )
        .unwrap();
        let body_rows: Vec<_> = code_frame_rows(&out)
            .into_iter()
            .filter(|row| strip_ansi_for_test(row).starts_with('│'))
            .collect();

        assert!(
            body_rows.len() >= 3,
            "unbroken token should hard-wrap: {out:?}"
        );
        for row in body_rows {
            let visible = strip_ansi_for_test(row);
            assert_eq!(code_visible_width(row), width, "wrapped row width: {row:?}");
            assert!(visible.starts_with('│'), "missing left border: {visible:?}");
            assert!(visible.ends_with('│'), "missing right border: {visible:?}");
        }
    }

    #[test]
    fn highlighted_fenced_code_reopens_body_sgr_on_wrapped_rows() {
        let width = 24usize;
        let out = render_ansi_with_source_and_width(
            "```rust\nlet message = \"abcdefghijklmnopqrstuvwxyz\";\n```\n",
            None,
            4,
            width,
        )
        .unwrap();
        let body_rows: Vec<_> = code_frame_rows(&out)
            .into_iter()
            .filter(|row| strip_ansi_for_test(row).starts_with('│'))
            .collect();

        assert!(
            body_rows.len() >= 2,
            "highlighted line should wrap: {out:?}"
        );
        let body_sgr = first_non_frame_sgr(body_rows[0]).expect("syntect body SGR");
        assert_ne!(
            body_sgr, MUTED,
            "test must pin body styling, not frame styling"
        );

        for row in body_rows.iter().skip(1).filter(|row| {
            strip_ansi_for_test(row)
                .chars()
                .any(|c| c.is_alphanumeric())
        }) {
            assert!(
                row.contains(&body_sgr),
                "continuation row did not reopen body SGR {body_sgr:?}: {row:?}"
            );
        }
    }

    #[test]
    fn code_block_expands_tabs() {
        let src = "```\n\tindented\n```\n";
        let out = render_ansi_with_source(src, None, 4).unwrap();
        assert!(out.contains("    indented"), "got: {out}");
        let out2 = render_ansi_with_source(src, None, 2).unwrap();
        assert!(out2.contains("  indented"), "got: {out2}");
    }

    fn border_columns_for_test(line: &str, border: char) -> Vec<usize> {
        let mut cols = Vec::new();
        let mut col = 0usize;
        for g in line.graphemes(true) {
            if g.chars().next() == Some(border) && g.chars().count() == 1 {
                cols.push(col);
            }
            col += display_width_for_test(g);
        }
        cols
    }

    fn display_width_for_test(s: &str) -> usize {
        s.graphemes(true)
            .map(|g| {
                if g.chars()
                    .any(|c| UnicodeWidthChar::width(c).unwrap_or(0) >= 2)
                {
                    2
                } else {
                    g.chars()
                        .map(|c| UnicodeWidthChar::width(c).unwrap_or(0))
                        .sum()
                }
            })
            .sum()
    }

    fn ansi_sgrs_for_test(input: &str) -> Vec<String> {
        let mut sgrs = Vec::new();
        let mut chars = input.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch != '\x1b' || chars.next() != Some('[') {
                continue;
            }
            let mut sgr = String::from("\x1b[");
            for next in chars.by_ref() {
                sgr.push(next);
                if next == 'm' {
                    sgrs.push(sgr);
                    break;
                }
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        }
        sgrs
    }

    fn code_frame_rows(out: &str) -> Vec<&str> {
        out.lines()
            .filter(|line| {
                let visible = strip_ansi_for_test(line);
                visible.starts_with('╭') || visible.starts_with('│') || visible.starts_with('╰')
            })
            .collect()
    }

    fn first_non_frame_sgr(input: &str) -> Option<String> {
        ansi_sgrs_for_test(input)
            .into_iter()
            .find(|sgr| sgr != MUTED && sgr != RESET)
    }

    fn strip_ansi_for_test(input: &str) -> String {
        let mut out = String::new();
        let mut chars = input.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\x1b' && chars.next() == Some('[') {
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                out.push(ch);
            }
        }
        out
    }
}
