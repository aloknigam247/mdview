use std::collections::BTreeMap;

pub use mdview_theme::{NvimHl, Theme};

pub fn theme_from_nvim_highlights(
    colorscheme: &str,
    _version: &str,
    hl: &BTreeMap<String, NvimHl>,
) -> Theme {
    mdview_theme::theme_from_nvim_highlights(colorscheme, hl)
}
