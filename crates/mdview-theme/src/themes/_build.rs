use crate::_stubs::{Radii, StyleSpec, Theme, Typography};
use std::collections::BTreeMap;

pub(crate) struct Palette {
    pub name: &'static str,
    pub accent: &'static str,
    pub accent_blue: &'static str,
    pub accent_green: &'static str,
    pub accent_mauve: &'static str,
    pub accent_peach: &'static str,
    pub accent_teal: &'static str,
    pub accent_yellow: &'static str,
    pub bg: &'static str,
    pub border_subtle: &'static str,
    pub code_bg: &'static str,
    pub code_inline_fg: &'static str,
    pub fg: &'static str,
    pub heading: [&'static str; 6],
    pub link: &'static str,
    pub muted: &'static str,
    pub quote_fg: &'static str,
    pub table_border: &'static str,
}

pub(crate) fn build(p: Palette) -> Theme {
    let mut colors: BTreeMap<String, String> = BTreeMap::new();
    colors.insert("accent".into(), p.accent.into());
    colors.insert("accent.blue".into(), p.accent_blue.into());
    colors.insert("accent.green".into(), p.accent_green.into());
    colors.insert("accent.mauve".into(), p.accent_mauve.into());
    colors.insert("accent.peach".into(), p.accent_peach.into());
    colors.insert("accent.teal".into(), p.accent_teal.into());
    colors.insert("accent.yellow".into(), p.accent_yellow.into());
    colors.insert("bg".into(), p.bg.into());
    colors.insert("border.subtle".into(), p.border_subtle.into());
    colors.insert("code.bg".into(), p.code_bg.into());
    colors.insert("fg".into(), p.fg.into());
    colors.insert("frontmatter.card.bg".into(), p.code_bg.into());
    colors.insert("frontmatter.card.border".into(), p.border_subtle.into());
    colors.insert("frontmatter.heading.fg".into(), p.heading[0].into());
    colors.insert("frontmatter.muted.fg".into(), p.muted.into());
    colors.insert("frontmatter.subtitle.fg".into(), p.quote_fg.into());
    colors.insert("frontmatter.tag.bg".into(), p.accent_mauve.into());
    colors.insert("frontmatter.tag.fg".into(), p.accent_mauve.into());
    colors.insert("link".into(), p.link.into());
    colors.insert("muted".into(), p.muted.into());
    colors.insert("table.border".into(), p.table_border.into());

    let mut styles: BTreeMap<String, StyleSpec> = BTreeMap::new();
    styles.insert(
        "blockquote".into(),
        StyleSpec {
            fg: Some(p.quote_fg.into()),
            italic: true,
            ..Default::default()
        },
    );
    styles.insert(
        "code.inline".into(),
        StyleSpec {
            fg: Some(p.code_inline_fg.into()),
            bg: Some(p.code_bg.into()),
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
    for (i, color) in p.heading.iter().enumerate() {
        styles.insert(
            format!("heading.{}", i + 1),
            StyleSpec {
                fg: Some((*color).into()),
                bold: true,
                ..Default::default()
            },
        );
    }
    styles.insert(
        "link".into(),
        StyleSpec {
            fg: Some(p.link.into()),
            underline: true,
            ..Default::default()
        },
    );
    styles.insert(
        "list.marker".into(),
        StyleSpec {
            fg: Some(p.accent.into()),
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
            fg: Some(p.heading[0].into()),
            bold: true,
            ..Default::default()
        },
    );

    Theme {
        name: p.name.into(),
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
