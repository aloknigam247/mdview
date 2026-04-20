// TODO: replace with mdview_core after integration
#![allow(dead_code)]

use std::collections::BTreeMap;

pub use comrak::nodes::AstNode;

#[derive(Debug, Clone, Default)]
pub struct Html(pub String);

#[derive(Debug, Clone, Default)]
pub struct TermChunks(pub Vec<TermChunk>);

#[derive(Debug, Clone)]
pub enum TermChunk {
    Text(String),
    Sixel(Vec<u8>),
    AsciiPlaceholder(String),
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
}

#[derive(Debug, Clone, Copy)]
pub struct Asset {
    pub path: &'static str,
    pub mime: &'static str,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Radii {
    pub sm: u16,
    pub md: u16,
    pub lg: u16,
}

#[derive(Debug, Clone, Default)]
pub struct Typography {
    pub body: String,
    pub mono: String,
    pub headings: String,
}

#[derive(Debug, Clone, Default)]
pub struct StyleSpec {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub bold: bool,
    pub italic: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Theme {
    pub name: &'static str,
    pub colors: BTreeMap<&'static str, &'static str>,
    pub styles: BTreeMap<&'static str, StyleSpec>,
    pub radii: Radii,
    pub typography: Typography,
}

pub trait MdViewExtension: Send + Sync {
    fn name(&self) -> &'static str;
    fn register_parser(&self, _opts: &mut comrak::ComrakOptions) {}
    fn pre_parse(&self, _src: &mut String) {}
    fn transform(&self, _ast: &mut AstNode) {}
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
