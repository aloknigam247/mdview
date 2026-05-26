use crate::_stubs::{TermChunk, TermChunks};

/// A match located within the flattened plaintext of the scrollback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    /// Index into the flattened plaintext lines.
    pub line: usize,
    /// Byte offset within that line.
    pub col: usize,
    /// Length of the match in bytes.
    pub len: usize,
}

/// Search index over the rendered plaintext of a chunk stream.
#[derive(Debug, Clone, Default)]
pub struct SearchIndex {
    /// Plaintext lines with ANSI escapes stripped. Sixel chunks become a
    /// single placeholder line.
    pub lines: Vec<String>,
}

impl SearchIndex {
    pub fn build(chunks: &TermChunks) -> Self {
        let mut lines: Vec<String> = Vec::new();
        let mut current = String::new();
        for chunk in &chunks.chunks {
            match chunk {
                TermChunk::Ansi(s) => {
                    for ch in strip_ansi(s).chars() {
                        if ch == '\n' {
                            lines.push(std::mem::take(&mut current));
                        } else {
                            current.push(ch);
                        }
                    }
                }
                TermChunk::Sixel { rows, .. } => {
                    if !current.is_empty() {
                        lines.push(std::mem::take(&mut current));
                    }
                    for _ in 0..(*rows).max(1) {
                        lines.push(String::new());
                    }
                }
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
        SearchIndex { lines }
    }

    pub fn search(&self, needle: &str) -> Vec<Match> {
        let mut out = Vec::new();
        if needle.is_empty() {
            return out;
        }
        for (i, line) in self.lines.iter().enumerate() {
            let mut start = 0;
            while let Some(pos) = line[start..].find(needle) {
                let col = start + pos;
                out.push(Match {
                    line: i,
                    col,
                    len: needle.len(),
                });
                start = col + needle.len().max(1);
                if start >= line.len() {
                    break;
                }
            }
        }
        out
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}

/// Strip ANSI CSI / OSC escape sequences. Kept public so the pager widget can
/// reuse it for visible-width calculations.
pub fn strip_ansi(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1B && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            if next == b'[' {
                i += 2;
                while i < bytes.len() {
                    let b = bytes[i];
                    i += 1;
                    if (0x40..=0x7E).contains(&b) {
                        break;
                    }
                }
                continue;
            } else if next == b']' {
                i += 2;
                while i < bytes.len() {
                    let b = bytes[i];
                    if b == 0x07 {
                        i += 1;
                        break;
                    }
                    if b == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
                continue;
            } else {
                i += 2;
                continue;
            }
        }
        let ch_len = utf8_char_len(bytes[i]);
        let end = (i + ch_len).min(bytes.len());
        out.push_str(std::str::from_utf8(&bytes[i..end]).unwrap_or(""));
        i = end;
    }
    out
}

fn utf8_char_len(b: u8) -> usize {
    if b < 0xC0 {
        1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
}
