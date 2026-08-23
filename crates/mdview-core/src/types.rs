use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Html(pub String);

impl Html {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.0.into_bytes()
    }
}

impl From<String> for Html {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Html {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl std::fmt::Display for Html {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::ops::Deref for Html {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[u8]> for Html {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

pub type TermChunks = Vec<TermChunk>;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TermChunk {
    pub style: StyleSpec,
    pub text: String,
}

impl TermChunk {
    pub fn new(text: impl Into<String>, style: StyleSpec) -> Self {
        Self {
            style,
            text: text.into(),
        }
    }

    pub fn plain(text: impl Into<String>) -> Self {
        Self::new(text, StyleSpec::default())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub mime: &'static str,
    pub path: &'static str,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct StyleSpec {
    pub bg: Option<String>,
    pub bold: bool,
    pub fg: Option<String>,
    pub italic: bool,
    pub underline: bool,
}

impl StyleSpec {
    pub fn bold() -> Self {
        Self {
            bold: true,
            ..Default::default()
        }
    }

    pub fn fg(color: &str) -> Self {
        Self {
            fg: Some(color.to_string()),
            ..Default::default()
        }
    }

    pub fn italic() -> Self {
        Self {
            italic: true,
            ..Default::default()
        }
    }

    pub fn with_bg(mut self, color: &str) -> Self {
        self.bg = Some(color.to_string());
        self
    }

    pub fn with_bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn with_fg(mut self, color: &str) -> Self {
        self.fg = Some(color.to_string());
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
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Radii {
    pub lg: u32,
    pub md: u32,
    pub sm: u32,
}

impl Default for Radii {
    fn default() -> Self {
        Self {
            lg: 16,
            md: 10,
            sm: 6,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Typography {
    pub body: String,
    pub headings: String,
    pub mono: String,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            body: "ui-sans-serif, system-ui, sans-serif".into(),
            headings: "ui-sans-serif, system-ui, sans-serif".into(),
            mono: "ui-monospace, SFMono-Regular, Menlo, monospace".into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    pub colors: BTreeMap<String, String>,
    pub name: String,
    pub radii: Radii,
    pub styles: BTreeMap<String, StyleSpec>,
    pub typography: Typography,
}

impl Theme {
    pub fn default_dark() -> Self {
        let mut colors = BTreeMap::new();
        colors.insert("accent".into(), "#89b4fa".into());
        colors.insert("bg".into(), "#1e1e2e".into());
        colors.insert("code.bg".into(), "#313244".into());
        colors.insert("fg".into(), "#e6e6e6".into());
        colors.insert("link".into(), "#74c7ec".into());
        colors.insert("muted".into(), "#6c7086".into());

        let mut styles = BTreeMap::new();
        styles.insert("blockquote".into(), StyleSpec::fg("#6c7086").with_italic());
        styles.insert(
            "code.inline".into(),
            StyleSpec::fg("#f9e2af").with_bg("#313244"),
        );
        styles.insert("emph".into(), StyleSpec::italic());
        styles.insert("heading".into(), StyleSpec::fg("#89b4fa").with_bold());
        styles.insert("link".into(), StyleSpec::fg("#74c7ec").with_underline());
        styles.insert("muted".into(), StyleSpec::fg("#6c7086"));
        styles.insert("strong".into(), StyleSpec::bold());
        styles.insert("table.header".into(), StyleSpec::fg("#89b4fa").with_bold());

        Self {
            colors,
            name: "default-dark".into(),
            radii: Radii::default(),
            styles,
            typography: Typography::default(),
        }
    }

    pub fn color(&self, key: &str) -> Option<&str> {
        self.colors.get(key).map(String::as_str)
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TerminalCaps {
    pub height: u16,
    pub sixel: bool,
    pub truecolor: bool,
    pub width: u16,
}

impl Default for TerminalCaps {
    fn default() -> Self {
        Self {
            height: 24,
            sixel: false,
            truecolor: true,
            width: 80,
        }
    }
}

pub struct RenderCtx<'a> {
    pub asset_resolver: fn(&str) -> String,
    pub frontmatter: Option<serde_json::Value>,
    pub source_dir: Option<std::path::PathBuf>,
    pub tab_width: u8,
    pub terminal_caps: Option<TerminalCaps>,
    pub theme: &'a Theme,
}

impl<'a> RenderCtx<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self {
            asset_resolver: default_asset_resolver,
            frontmatter: None,
            source_dir: None,
            tab_width: 4,
            terminal_caps: None,
            theme,
        }
    }

    pub fn resolve_asset(&self, path: &str) -> String {
        (self.asset_resolver)(path)
    }
}

impl Default for RenderCtx<'static> {
    fn default() -> Self {
        use std::sync::OnceLock;

        static THEME: OnceLock<Theme> = OnceLock::new();
        Self::new(THEME.get_or_init(Theme::default))
    }
}

fn default_asset_resolver(path: &str) -> String {
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_newtype_roundtrip() {
        let h: Html = "<p>hi</p>".into();
        assert_eq!(h.as_str(), "<p>hi</p>");
        assert_eq!(h, Html::new("<p>hi</p>".to_string()));
    }

    #[test]
    fn term_chunk_plain_has_default_style() {
        let c = TermChunk::plain("x");
        assert_eq!(c.text, "x");
        assert_eq!(c.style, StyleSpec::default());
    }

    #[test]
    fn render_ctx_defaults() {
        let t = Theme::default();
        let ctx = RenderCtx::new(&t);
        assert!(ctx.terminal_caps.is_none());
        assert!(ctx.source_dir.is_none());
        assert_eq!(ctx.resolve_asset("foo"), "foo");
    }

    #[test]
    fn radii_default_is_curvy() {
        let r = Radii::default();
        assert!(r.sm < r.md && r.md < r.lg);
    }
}
