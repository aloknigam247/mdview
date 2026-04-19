use crate::_stubs::{Radii, StyleSpec, Theme, Typography};
use crate::themes::{dark, light};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct NvimHl {
    pub fg: Option<u32>,
    pub bg: Option<u32>,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
}

pub fn theme_from_nvim_highlights(colorscheme: &str, hl: &BTreeMap<String, NvimHl>) -> Theme {
    let normal = hl.get("Normal").copied().unwrap_or_default();
    let is_dark = normal
        .bg
        .map(is_dark_rgb)
        .unwrap_or_else(|| looks_dark(colorscheme));
    let fallback = if is_dark { dark::get() } else { light::get() };
    let fb = |k: &str| fallback.colors.get(k).cloned().unwrap_or_default();

    let fg_hex = normal.fg.map(hex).unwrap_or_else(|| fb("fg"));
    let bg_hex = normal.bg.map(hex).unwrap_or_else(|| fb("bg"));
    let accent_hex = hl
        .get("@markup.link")
        .or_else(|| hl.get("Function"))
        .and_then(|h| h.fg)
        .map(hex)
        .unwrap_or_else(|| fb("accent"));
    let muted_hex = hl
        .get("Comment")
        .and_then(|h| h.fg)
        .map(hex)
        .unwrap_or_else(|| fb("muted"));
    let code_bg_hex = hl
        .get("String")
        .and_then(|h| h.bg.or(h.fg))
        .map(hex)
        .unwrap_or_else(|| fb("code.bg"));

    let mut colors: BTreeMap<String, String> = BTreeMap::new();
    colors.insert("accent".into(), accent_hex.clone());
    colors.insert("bg".into(), bg_hex);
    colors.insert("border.subtle".into(), fb("border.subtle"));
    colors.insert("code.bg".into(), code_bg_hex.clone());
    colors.insert("fg".into(), fg_hex.clone());
    colors.insert("link".into(), accent_hex.clone());
    colors.insert("muted".into(), muted_hex.clone());
    colors.insert("table.border".into(), fb("table.border"));

    let mut styles: BTreeMap<String, StyleSpec> = BTreeMap::new();
    for n in 1..=6u8 {
        let key = format!("@markup.heading.{n}");
        let color = hl
            .get(&key)
            .and_then(|h| h.fg)
            .map(hex)
            .unwrap_or_else(|| fg_hex.clone());
        styles.insert(
            format!("heading.{n}"),
            StyleSpec {
                fg: Some(color),
                bold: true,
                ..Default::default()
            },
        );
    }
    styles.insert(
        "blockquote".into(),
        StyleSpec {
            fg: Some(muted_hex.clone()),
            italic: true,
            ..Default::default()
        },
    );
    styles.insert(
        "code.inline".into(),
        StyleSpec {
            bg: Some(code_bg_hex),
            ..Default::default()
        },
    );
    styles.insert(
        "emphasis".into(),
        StyleSpec {
            italic: true,
            ..Default::default()
        },
    );
    styles.insert(
        "link".into(),
        StyleSpec {
            fg: Some(accent_hex.clone()),
            underline: true,
            ..Default::default()
        },
    );
    styles.insert(
        "list.marker".into(),
        StyleSpec {
            fg: Some(accent_hex),
            ..Default::default()
        },
    );
    styles.insert(
        "strong".into(),
        StyleSpec {
            bold: true,
            ..Default::default()
        },
    );
    styles.insert(
        "table.header".into(),
        StyleSpec {
            fg: Some(fg_hex),
            bold: true,
            ..Default::default()
        },
    );

    Theme {
        name: colorscheme.to_string(),
        colors,
        styles,
        radii: Radii {
            sm: 4,
            md: 10,
            lg: 16,
        },
        typography: Typography::default(),
    }
}

fn hex(rgb: u32) -> String {
    format!("#{:06x}", rgb & 0x00ff_ffff)
}

fn is_dark_rgb(rgb: u32) -> bool {
    let r = ((rgb >> 16) & 0xff) as f32;
    let g = ((rgb >> 8) & 0xff) as f32;
    let b = (rgb & 0xff) as f32;
    (0.299 * r + 0.587 * g + 0.114 * b) < 128.0
}

fn looks_dark(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("dark")
        || n.contains("night")
        || n.contains("dracula")
        || n.contains("nord")
        || n.contains("tokyonight")
        || n.contains("gruvbox-dark")
}
