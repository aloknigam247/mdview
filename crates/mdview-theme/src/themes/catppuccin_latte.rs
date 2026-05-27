use crate::_stubs::Theme;
use crate::themes::_build::{build, Palette};
use std::sync::OnceLock;

static THEME: OnceLock<Theme> = OnceLock::new();

pub fn get() -> &'static Theme {
    THEME.get_or_init(|| {
        build(Palette {
            name: "catppuccin-latte",
            accent: "#1e66f5",
            accent_blue: "#1e66f5",
            accent_green: "#40a02b",
            accent_mauve: "#8839ef",
            accent_peach: "#fe640b",
            accent_teal: "#179299",
            accent_yellow: "#df8e1d",
            bg: "#eff1f5",
            border_subtle: "#dce0e8",
            code_bg: "#ccd0da",
            code_inline_fg: "#d20f39",
            fg: "#4c4f69",
            heading: [
                "#4c4f69", "#8839ef", "#1e66f5", "#179299", "#40a02b", "#fe640b",
            ],
            link: "#1e66f5",
            muted: "#6c6f85",
            quote_fg: "#5c5f77",
            table_border: "#dce0e8",
        })
    })
}
