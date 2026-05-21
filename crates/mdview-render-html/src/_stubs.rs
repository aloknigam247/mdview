// TODO: replace with mdview_core / mdview_theme types after integration.

use std::collections::BTreeMap;

pub use comrak::nodes::AstNode;

pub type Html = String;

#[derive(Debug, Clone, Default)]
pub struct Asset {
    pub path: &'static str,
    pub bytes: &'static [u8],
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Radii {
    pub sm: u16,
    pub md: u16,
    pub lg: u16,
}

#[derive(Debug, Clone, Default)]
pub struct Typography {
    pub body: &'static str,
    pub mono: &'static str,
    pub headings: &'static str,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StyleSpec {
    pub bold: bool,
    pub italic: bool,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub colors: BTreeMap<&'static str, &'static str>,
    pub styles: BTreeMap<&'static str, StyleSpec>,
    pub radii: Radii,
    pub typography: Typography,
}

impl Default for Theme {
    fn default() -> Self {
        let mut colors: BTreeMap<&'static str, &'static str> = BTreeMap::new();
        colors.insert("accent", "#6c7cff");
        colors.insert("bg", "#ffffff");
        colors.insert("border", "#e5e7eb");
        colors.insert("code.bg", "#f6f8fa");
        colors.insert("fg", "#111827");
        colors.insert("link", "#2563eb");
        colors.insert("muted", "#6b7280");
        Self {
            name: "default",
            colors,
            styles: BTreeMap::new(),
            radii: Radii {
                sm: 4,
                md: 10,
                lg: 16,
            },
            typography: Typography {
                body: "ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, sans-serif",
                mono: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
                headings: "ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, sans-serif",
            },
        }
    }
}

// TODO: replace with mdview_theme::emit_css after integration.
pub mod mdview_theme {
    use super::Theme;

    pub fn emit_css(theme: &Theme) -> String {
        let fg = theme.colors.get("fg").copied().unwrap_or("#111827");
        let bg = theme.colors.get("bg").copied().unwrap_or("#ffffff");
        let accent = theme.colors.get("accent").copied().unwrap_or("#6c7cff");
        let muted = theme.colors.get("muted").copied().unwrap_or("#6b7280");
        let link = theme.colors.get("link").copied().unwrap_or("#2563eb");
        let code_bg = theme.colors.get("code.bg").copied().unwrap_or("#f6f8fa");
        let border = theme.colors.get("border").copied().unwrap_or("#e5e7eb");
        let r_sm = theme.radii.sm;
        let r_md = theme.radii.md;
        let r_lg = theme.radii.lg;
        let body_font = theme.typography.body;
        let mono_font = theme.typography.mono;
        let heading_font = theme.typography.headings;
        format!(
            ":root {{\n  --mdv-fg: {fg};\n  --mdv-bg: {bg};\n  --mdv-accent: {accent};\n  --mdv-muted: {muted};\n  --mdv-link: {link};\n  --mdv-code-bg: {code_bg};\n  --mdv-border: {border};\n  --mdv-radius-sm: {r_sm}px;\n  --mdv-radius-md: {r_md}px;\n  --mdv-radius-lg: {r_lg}px;\n  --mdv-font-body: {body_font};\n  --mdv-font-mono: {mono_font};\n  --mdv-font-heading: {heading_font};\n}}\n"
        )
    }
}

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
