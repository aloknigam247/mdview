// TODO: replace with mdview_core / mdview_theme / mdview_sixel after integration.
//
// Minimal local definitions that mirror the shared contracts so this crate
// compiles and tests in isolation. Replace with real imports once the sibling
// crates land in this worktree.

use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Html(pub String);

#[derive(Debug, Clone)]
pub struct TermChunk {
    pub text: String,
    pub style: Option<String>,
    pub image: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Default)]
pub struct TermChunks(pub Vec<TermChunk>);

impl TermChunks {
    pub fn from_text(text: impl Into<String>) -> Self {
        Self(vec![TermChunk {
            text: text.into(),
            style: None,
            image: None,
        }])
    }

    pub fn from_image(bytes: Vec<u8>) -> Self {
        Self(vec![TermChunk {
            text: String::new(),
            style: None,
            image: Some(bytes),
        }])
    }

    pub fn joined_text(&self) -> String {
        self.0
            .iter()
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>()
            .join("")
    }
}

#[derive(Debug, Clone)]
pub struct Asset {
    pub path: &'static str,
    pub mime: &'static str,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Radii {
    pub sm: u8,
    pub md: u8,
    pub lg: u8,
}

#[derive(Debug, Clone, Default)]
pub struct StyleSpec {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Typography {
    pub body: String,
    pub mono: String,
    pub headings: String,
}

#[derive(Debug, Clone, Default)]
pub struct Theme {
    pub name: String,
    pub colors: BTreeMap<String, String>,
    pub styles: BTreeMap<String, StyleSpec>,
    pub radii: Radii,
    pub typography: Typography,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TerminalCaps {
    pub truecolor: bool,
    pub sixel: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RenderCtx {
    pub theme: Theme,
    pub terminal_caps: TerminalCaps,
    pub width: u16,
}

/// A fenced code block surrogate for the AST node.
///
/// In the integrated world this is `comrak::nodes::AstNode`. Here we accept a
/// lightweight enum so the extension can be unit-tested without the full
/// parser.
#[derive(Debug, Clone)]
pub enum AstNode {
    FencedCode { info: String, literal: String },
    Other,
}

pub trait SixelRenderer: Send + Sync {
    fn render_image(&self, svg: &[u8]) -> Result<Vec<u8>, String>;
}

pub trait MdViewExtension: Send + Sync {
    fn name(&self) -> &'static str;
    fn render_html(&self, _n: &AstNode, _ctx: &RenderCtx) -> Option<Html> {
        None
    }
    fn render_terminal(&self, _n: &AstNode, _ctx: &RenderCtx) -> Option<TermChunks> {
        None
    }
    fn client_assets(&self) -> &'static [Asset] {
        &[]
    }
}
