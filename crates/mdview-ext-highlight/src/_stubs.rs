use std::collections::BTreeMap;

pub use comrak::nodes::AstNode;

// TODO: replace with mdview_core::MdViewExtension after integration
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

// TODO: replace with mdview_theme::Theme after integration
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub colors: BTreeMap<&'static str, &'static str>,
    pub styles: BTreeMap<&'static str, StyleSpec>,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "light",
            colors: BTreeMap::new(),
            styles: BTreeMap::new(),
        }
    }
}

// TODO: replace with mdview_theme::StyleSpec after integration
#[derive(Debug, Clone, Default)]
pub struct StyleSpec {
    pub fg: Option<&'static str>,
    pub bg: Option<&'static str>,
    pub bold: bool,
    pub italic: bool,
}

// TODO: replace with mdview_core::RenderCtx after integration
#[derive(Debug, Clone, Default)]
pub struct RenderCtx {
    pub theme: Theme,
    pub truecolor: bool,
}

// TODO: replace with mdview_core::Html after integration
#[derive(Debug, Clone)]
pub struct Html(pub String);

// TODO: replace with mdview_core::TermChunks after integration
#[derive(Debug, Clone, Default)]
pub struct TermChunks {
    pub chunks: Vec<TermChunk>,
}

// TODO: replace with mdview_core::TermChunk after integration
#[derive(Debug, Clone)]
pub struct TermChunk {
    pub text: String,
    pub style: Option<StyleSpec>,
    pub ansi: Option<String>,
}

// TODO: replace with mdview_core::Asset after integration
#[derive(Debug, Clone)]
pub struct Asset {
    pub path: &'static str,
    pub bytes: &'static [u8],
}
