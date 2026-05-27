use crate::_stubs::Theme;
use crate::themes::_build::{build, Palette};
use std::sync::OnceLock;

static THEME: OnceLock<Theme> = OnceLock::new();

pub fn get() -> &'static Theme {
    THEME.get_or_init(|| {
        build(Palette {
            name: "catppuccin-mocha",
            accent: "#89b4fa",
            accent_blue: "#89b4fa",
            accent_green: "#a6e3a1",
            accent_mauve: "#cba6f7",
            accent_peach: "#fab387",
            accent_teal: "#94e2d5",
            accent_yellow: "#f9e2af",
            bg: "#1e1e2e",
            border_subtle: "#11111b",
            code_bg: "#313244",
            code_hl_bg: "#45475a",
            code_inline_fg: "#f38ba8",
            fg: "#cdd6f4",
            heading: [
                "#cdd6f4", "#cba6f7", "#89b4fa", "#94e2d5", "#a6e3a1", "#fab387",
            ],
            link: "#89b4fa",
            muted: "#a6adc8",
            quote_fg: "#bac2de",
            table_border: "#11111b",
        })
    })
}
