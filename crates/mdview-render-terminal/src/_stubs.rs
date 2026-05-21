// TODO: replace with mdview_core / mdview_theme after integration
#![allow(dead_code)]

use std::collections::BTreeMap;

pub use comrak::nodes::AstNode;

#[derive(Debug, Clone, Default)]
pub struct TermChunk {
    pub text: String,
    pub style: Option<StyleSpec>,
}

impl TermChunk {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: None,
        }
    }

    pub fn styled(text: impl Into<String>, style: StyleSpec) -> Self {
        Self {
            text: text.into(),
            style: Some(style),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TermChunks {
    pub chunks: Vec<TermChunk>,
}

impl TermChunks {
    pub fn new() -> Self {
        Self { chunks: Vec::new() }
    }

    pub fn push(&mut self, chunk: TermChunk) {
        self.chunks.push(chunk);
    }

    pub fn push_plain(&mut self, text: impl Into<String>) {
        self.chunks.push(TermChunk::plain(text));
    }

    pub fn push_styled(&mut self, text: impl Into<String>, style: StyleSpec) {
        self.chunks.push(TermChunk::styled(text, style));
    }

    pub fn extend(&mut self, other: TermChunks) {
        self.chunks.extend(other.chunks);
    }

    pub fn to_ansi(&self) -> String {
        self.to_ansi_with(true)
    }

    pub fn to_ansi_with(&self, truecolor: bool) -> String {
        let mut out = String::new();
        for c in &self.chunks {
            if let Some(style) = &c.style {
                let open = crate::color::sgr_open(style, truecolor);
                if !open.is_empty() {
                    out.push_str(&open);
                    out.push_str(&c.text);
                    out.push_str(crate::color::SGR_RESET);
                } else {
                    out.push_str(&c.text);
                }
            } else {
                out.push_str(&c.text);
            }
        }
        out
    }

    pub fn plain_text(&self) -> String {
        self.chunks.iter().map(|c| c.text.clone()).collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct StyleSpec {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl StyleSpec {
    pub fn fg(color: &str) -> Self {
        Self {
            fg: Some(color.to_string()),
            ..Default::default()
        }
    }
    pub fn bold() -> Self {
        Self {
            bold: true,
            ..Default::default()
        }
    }
    pub fn italic() -> Self {
        Self {
            italic: true,
            ..Default::default()
        }
    }
    pub fn with_bold(mut self) -> Self {
        self.bold = true;
        self
    }
    pub fn with_italic(mut self) -> Self {
        self.italic = true;
        self
    }
    pub fn with_underline(mut self) -> Self {
        self.underline = true;
        self
    }
    pub fn with_fg(mut self, c: &str) -> Self {
        self.fg = Some(c.to_string());
        self
    }
    pub fn with_bg(mut self, c: &str) -> Self {
        self.bg = Some(c.to_string());
        self
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub colors: BTreeMap<&'static str, &'static str>,
    pub styles: BTreeMap<&'static str, StyleSpec>,
}

impl Theme {
    pub fn default_dark() -> Self {
        let mut colors = BTreeMap::new();
        colors.insert("fg", "#e6e6e6");
        colors.insert("bg", "#1e1e2e");
        colors.insert("accent", "#89b4fa");
        colors.insert("muted", "#6c7086");
        colors.insert("code.bg", "#313244");
        colors.insert("link", "#74c7ec");
        colors.insert("heading.1", "#f38ba8");
        colors.insert("heading.2", "#fab387");
        colors.insert("heading.3", "#f9e2af");
        colors.insert("heading.4", "#a6e3a1");
        colors.insert("heading.5", "#89b4fa");
        colors.insert("heading.6", "#cba6f7");

        let mut styles = BTreeMap::new();
        styles.insert("heading", StyleSpec::fg("#89b4fa").with_bold());
        styles.insert("blockquote", StyleSpec::fg("#6c7086").with_italic());
        styles.insert("link", StyleSpec::fg("#74c7ec").with_underline());
        styles.insert("code.inline", StyleSpec::fg("#f9e2af").with_bg("#313244"));
        styles.insert("emph", StyleSpec::italic());
        styles.insert("strong", StyleSpec::bold());
        styles.insert("muted", StyleSpec::fg("#6c7086"));
        styles.insert("table.header", StyleSpec::fg("#89b4fa").with_bold());

        Self {
            name: "default-dark",
            colors,
            styles,
        }
    }

    pub fn color(&self, key: &str) -> Option<&str> {
        self.colors.get(key).copied()
    }

    pub fn style(&self, key: &str) -> StyleSpec {
        self.styles.get(key).cloned().unwrap_or_default()
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::default_dark()
    }
}

#[derive(Debug, Clone)]
pub struct TerminalCaps {
    pub width: usize,
    pub height: usize,
    pub truecolor: bool,
    pub sixel: bool,
}

impl Default for TerminalCaps {
    fn default() -> Self {
        Self {
            width: 100,
            height: 40,
            truecolor: true,
            sixel: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RenderCtx {
    pub theme: Theme,
    pub source_dir: Option<std::path::PathBuf>,
    pub terminal_caps: TerminalCaps,
}

pub trait TerminalRenderer: Send + Sync {
    fn render_terminal<'a>(&self, node: &'a AstNode<'a>, ctx: &RenderCtx) -> Option<TermChunks>;
}

#[derive(Default)]
pub struct Registry {
    terminal_renderers: Vec<Box<dyn TerminalRenderer>>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_terminal(&mut self, r: Box<dyn TerminalRenderer>) {
        self.terminal_renderers.push(r);
    }

    pub fn terminal_renderers(&self) -> &[Box<dyn TerminalRenderer>] {
        &self.terminal_renderers
    }
}
