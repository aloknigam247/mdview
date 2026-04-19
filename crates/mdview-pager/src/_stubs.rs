// TODO: replace with mdview_render_terminal / mdview_theme after integration

use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct StyleSpec {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Radii {
    pub sm: u8,
    pub md: u8,
    pub lg: u8,
}

#[derive(Debug, Clone, Default)]
pub struct Typography {
    pub body: String,
    pub mono: String,
    pub headings: String,
}

#[derive(Debug, Clone, Default)]
pub struct Theme {
    pub name: &'static str,
    pub colors: BTreeMap<&'static str, &'static str>,
    pub styles: BTreeMap<&'static str, StyleSpec>,
    pub radii: Radii,
    pub typography: Typography,
}

#[derive(Debug, Clone)]
pub enum TermChunk {
    /// Already-rendered text containing ANSI escape sequences.
    Ansi(String),
    /// Raw sixel payload to be written verbatim to stdout.
    Sixel {
        payload: String,
        rows: u16,
    },
}

#[derive(Debug, Clone, Default)]
pub struct TermChunks {
    pub chunks: Vec<TermChunk>,
    pub source_name: Option<String>,
}

impl TermChunks {
    pub fn push(&mut self, c: TermChunk) {
        self.chunks.push(c);
    }
}
