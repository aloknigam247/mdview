use crate::_stubs::Theme;
use crate::themes::_build::{build, Palette};
use std::sync::OnceLock;

static THEME: OnceLock<Theme> = OnceLock::new();

pub fn get() -> &'static Theme {
    THEME.get_or_init(|| {
        build(Palette {
            name: "solarized",
            accent: "#268bd2",
            accent_blue: "#268bd2",
            accent_green: "#859900",
            accent_mauve: "#6c71c4",
            accent_peach: "#cb4b16",
            accent_teal: "#2aa198",
            accent_yellow: "#b58900",
            bg: "#fdf6e3",
            border_subtle: "#eee8d5",
            code_bg: "#eee8d5",
            code_hl_bg: "#dcd5b8",
            code_inline_fg: "#dc322f",
            fg: "#586e75",
            heading: [
                "#002b36", "#073642", "#586e75", "#657b83", "#839496", "#93a1a1",
            ],
            link: "#268bd2",
            muted: "#93a1a1",
            quote_fg: "#657b83",
            table_border: "#eee8d5",
        })
    })
}
