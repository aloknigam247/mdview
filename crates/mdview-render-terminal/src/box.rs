use crate::_stubs::{StyleSpec, TermChunks, TermChunksExt, Theme};
use crate::wrap::visible_width;
use unicode_width::UnicodeWidthStr;

pub const TL: char = '╭';
pub const TR: char = '╮';
pub const BL: char = '╰';
pub const BR: char = '╯';
pub const H: char = '─';
pub const V: char = '│';
pub const T_DOWN: char = '┬';
pub const T_UP: char = '┴';
pub const T_LEFT: char = '┤';
pub const T_RIGHT: char = '├';
pub const CROSS: char = '┼';

pub fn code_frame(lang: Option<&str>, body: &str, theme: &Theme) -> TermChunks {
    let muted = theme.style("muted");
    let mut chunks = TermChunks::new();

    let lines: Vec<&str> = body.split('\n').collect();
    let max_content = lines
        .iter()
        .map(|l| UnicodeWidthStr::width(*l))
        .max()
        .unwrap_or(0);
    let label = lang.map(|l| format!("[ {} ]", l)).unwrap_or_default();
    let label_w = UnicodeWidthStr::width(label.as_str());
    let inner_w = max_content.max(label_w + 4).max(10);

    let top = {
        let mut s = String::new();
        s.push(TL);
        s.push(H);
        s.push(H);
        s.push(H);
        if !label.is_empty() {
            s.push_str(&label);
        }
        let used = 4 + label_w;
        for _ in used..(inner_w + 2) {
            s.push(H);
        }
        s.push(TR);
        s
    };
    chunks.push_styled(top, muted.clone());
    chunks.push_plain("\n");

    for line in &lines {
        let mut row = String::new();
        row.push(V);
        row.push(' ');
        row.push_str(line);
        let pad = inner_w.saturating_sub(UnicodeWidthStr::width(*line));
        for _ in 0..pad {
            row.push(' ');
        }
        row.push(' ');
        row.push(V);
        chunks.push_styled(row, muted.clone());
        chunks.push_plain("\n");
    }

    let bottom = {
        let mut s = String::new();
        s.push(BL);
        for _ in 0..(inner_w + 2) {
            s.push(H);
        }
        s.push(BR);
        s
    };
    chunks.push_styled(bottom, muted);
    chunks.push_plain("\n");

    chunks
}

pub fn table(headers: &[String], rows: &[Vec<String>], theme: &Theme) -> TermChunks {
    let muted = theme.style("muted");
    let header_style = theme.style("table.header");
    let mut chunks = TermChunks::new();

    let col_count = headers.len();
    let mut widths: Vec<usize> = headers
        .iter()
        .map(|h| UnicodeWidthStr::width(h.as_str()))
        .collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate().take(col_count) {
            let w = visible_width(cell);
            if w > widths[i] {
                widths[i] = w;
            }
        }
    }

    let horizontal_run = |w: usize| -> String {
        let mut s = String::new();
        for _ in 0..(w + 2) {
            s.push(H);
        }
        s
    };

    let top = {
        let mut s = String::new();
        s.push(TL);
        for (i, w) in widths.iter().enumerate() {
            s.push_str(&horizontal_run(*w));
            if i + 1 < widths.len() {
                s.push(T_DOWN);
            }
        }
        s.push(TR);
        s
    };
    chunks.push_styled(top, muted.clone());
    chunks.push_plain("\n");

    let mut header_row = String::from(V);
    for (i, h) in headers.iter().enumerate() {
        header_row.push(' ');
        header_row.push_str(h);
        let pad = widths[i].saturating_sub(UnicodeWidthStr::width(h.as_str()));
        for _ in 0..pad {
            header_row.push(' ');
        }
        header_row.push(' ');
        header_row.push(V);
    }
    chunks.push_styled(header_row, header_style);
    chunks.push_plain("\n");

    let sep = {
        let mut s = String::new();
        s.push(T_RIGHT);
        for (i, w) in widths.iter().enumerate() {
            s.push_str(&horizontal_run(*w));
            if i + 1 < widths.len() {
                s.push(CROSS);
            }
        }
        s.push(T_LEFT);
        s
    };
    chunks.push_styled(sep, muted.clone());
    chunks.push_plain("\n");

    for row in rows {
        let mut line = String::from(V);
        for (i, w) in widths.iter().enumerate().take(col_count) {
            let empty = String::new();
            let cell = row.get(i).unwrap_or(&empty);
            let cell_w = visible_width(cell);
            line.push(' ');
            line.push_str(cell);
            let pad = w.saturating_sub(cell_w);
            for _ in 0..pad {
                line.push(' ');
            }
            line.push(' ');
            line.push(V);
        }
        chunks.push_styled(line, StyleSpec::default());
        chunks.push_plain("\n");
    }

    let bottom = {
        let mut s = String::new();
        s.push(BL);
        for (i, w) in widths.iter().enumerate() {
            s.push_str(&horizontal_run(*w));
            if i + 1 < widths.len() {
                s.push(T_UP);
            }
        }
        s.push(BR);
        s
    };
    chunks.push_styled(bottom, muted);
    chunks.push_plain("\n");

    chunks
}
