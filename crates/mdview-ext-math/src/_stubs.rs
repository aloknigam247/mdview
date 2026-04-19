// TODO: replace with mdview_core after integration
#![allow(dead_code)]

use std::collections::BTreeMap;

pub use comrak::nodes::AstNode;

#[derive(Debug, Clone, Default)]
pub struct Html(pub String);

#[derive(Debug, Clone, Default)]
pub struct TermChunk {
    pub text: String,
    pub style: Option<StyleSpec>,
}

#[derive(Debug, Clone, Default)]
pub struct TermChunks(pub Vec<TermChunk>);

#[derive(Debug, Clone, Default)]
pub struct Asset {
    pub path: &'static str,
    pub mime: &'static str,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StyleSpec {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub fg: Option<&'static str>,
    pub bg: Option<&'static str>,
}

#[derive(Debug, Clone, Default)]
pub struct Theme {
    pub name: &'static str,
    pub colors: BTreeMap<&'static str, &'static str>,
    pub styles: BTreeMap<&'static str, StyleSpec>,
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

pub trait MdViewExtension: Send + Sync {
    fn name(&self) -> &'static str;
    fn register_parser(&self, _opts: &mut comrak::ComrakOptions) {}
    fn pre_parse(&self, _src: &mut String) {}
    fn transform<'a>(&self, _ast: &'a AstNode<'a>) {}
    fn render_html<'a>(&self, _n: &'a AstNode<'a>, _ctx: &RenderCtx) -> Option<Html> {
        None
    }
    fn render_terminal<'a>(&self, _n: &'a AstNode<'a>, _ctx: &RenderCtx) -> Option<TermChunks> {
        None
    }
    fn client_assets(&self) -> &'static [Asset] {
        &[]
    }
}
