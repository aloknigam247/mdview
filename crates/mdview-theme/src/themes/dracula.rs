use crate::_stubs::Theme;
use crate::themes::_build::{build, Palette};
use std::sync::OnceLock;

static THEME: OnceLock<Theme> = OnceLock::new();

pub fn get() -> &'static Theme {
    THEME.get_or_init(|| {
        build(Palette {
            name: "dracula",
            accent: "#bd93f9",
            bg: "#282a36",
            border_subtle: "#44475a",
            code_bg: "#21222c",
            code_inline_fg: "#ff79c6",
            fg: "#f8f8f2",
            heading: [
                "#ffffff", "#f8f8f2", "#f1fa8c", "#8be9fd", "#50fa7b", "#bd93f9",
            ],
            link: "#8be9fd",
            muted: "#6272a4",
            quote_fg: "#bd93f9",
            table_border: "#44475a",
        })
    })
}
