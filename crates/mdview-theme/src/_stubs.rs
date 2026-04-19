// TODO: replace with mdview_core imports after integration
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Theme {
    pub name: String,
    pub colors: BTreeMap<String, String>,
    pub styles: BTreeMap<String, StyleSpec>,
    pub radii: Radii,
    pub typography: Typography,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct StyleSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bg: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub underline: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Radii {
    pub sm: u16,
    pub md: u16,
    pub lg: u16,
}

impl Default for Radii {
    fn default() -> Self {
        Self {
            sm: 4,
            md: 10,
            lg: 16,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Typography {
    pub body: String,
    pub mono: String,
    pub headings: String,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            body: "ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, sans-serif".into(),
            mono: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace".into(),
            headings: "ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, sans-serif"
                .into(),
        }
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}
