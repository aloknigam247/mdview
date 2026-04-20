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
    pub background: Option<String>,
    pub bold: bool,
    pub color: Option<String>,
    pub italic: bool,
    pub underline: bool,
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

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    pub colors: BTreeMap<String, String>,
    pub name: String,
    pub radii: Radii,
    pub styles: BTreeMap<String, StyleSpec>,
    pub typography: Typography,
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
    pub terminal_caps: Option<TerminalCaps>,
    pub theme: &'a Theme,
}

impl<'a> RenderCtx<'a> {
    pub fn new(theme: &'a Theme) -> Self {
        Self {
            asset_resolver: default_asset_resolver,
            terminal_caps: None,
            theme,
        }
    }

    pub fn resolve_asset(&self, path: &str) -> String {
        (self.asset_resolver)(path)
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
        assert_eq!(ctx.resolve_asset("foo"), "foo");
    }

    #[test]
    fn radii_default_is_curvy() {
        let r = Radii::default();
        assert!(r.sm < r.md && r.md < r.lg);
    }
}
