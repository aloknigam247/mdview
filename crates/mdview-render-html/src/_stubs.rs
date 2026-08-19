pub use comrak::nodes::AstNode;
pub use mdview_core::{Asset, Html, Radii, StyleSpec, Theme, Typography};

pub trait HtmlRenderer: Send + Sync {
    fn name(&self) -> &'static str;
    fn node_types(&self) -> &'static [&'static str];
    fn test<'a>(&self, node: &'a AstNode<'a>) -> bool;
    fn render<'a>(&self, node: &'a AstNode<'a>, ctx: &RenderCtx) -> Html;
}

#[derive(Default)]
pub struct Registry {
    html: Vec<Box<dyn HtmlRenderer>>,
}

impl Registry {
    pub fn new() -> Self {
        Self { html: Vec::new() }
    }

    pub fn with_html_renderer<R: HtmlRenderer + 'static>(mut self, r: R) -> Self {
        self.html.push(Box::new(r));
        self
    }

    pub fn html_renderers(&self) -> &[Box<dyn HtmlRenderer>] {
        &self.html
    }
}

#[derive(Debug, Clone)]
pub struct RenderCtx {
    pub theme: Theme,
    pub live_reload: bool,
    pub source_dir: Option<std::path::PathBuf>,
    pub title: String,
}

impl Default for RenderCtx {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            live_reload: false,
            source_dir: None,
            title: "mdview".to_string(),
        }
    }
}
