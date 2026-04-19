// TODO: replace with mdview_theme after integration
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Theme {
    pub name: String,
    #[serde(default)]
    pub colors: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct NvimHl {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
}

pub fn theme_from_nvim_highlights(
    colorscheme: &str,
    _version: &str,
    hl: &BTreeMap<String, NvimHl>,
) -> Theme {
    let mut colors = BTreeMap::new();
    if let Some(normal) = hl.get("Normal") {
        if let Some(fg) = &normal.fg {
            colors.insert("fg".to_string(), fg.clone());
        }
        if let Some(bg) = &normal.bg {
            colors.insert("bg".to_string(), bg.clone());
        }
    }
    if let Some(func) = hl.get("Function").and_then(|h| h.fg.as_ref()) {
        colors.insert("accent".to_string(), func.clone());
    }
    if let Some(comment) = hl.get("Comment").and_then(|h| h.fg.as_ref()) {
        colors.insert("muted".to_string(), comment.clone());
    }
    Theme {
        name: colorscheme.to_string(),
        colors,
    }
}
