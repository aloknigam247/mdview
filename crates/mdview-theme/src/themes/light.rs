use crate::_stubs::Theme;
use crate::themes::_build::{build, Palette};
use std::sync::OnceLock;

static THEME: OnceLock<Theme> = OnceLock::new();

pub fn get() -> &'static Theme {
    THEME.get_or_init(|| {
        build(Palette {
            name: "light",
            accent: "#2563eb",
            bg: "#ffffff",
            border_subtle: "#e5e7eb",
            code_bg: "#f5f7fa",
            code_inline_fg: "#b91c1c",
            fg: "#1f2937",
            heading: [
                "#0f172a", "#111827", "#1f2937", "#374151", "#4b5563", "#6b7280",
            ],
            link: "#2563eb",
            muted: "#6b7280",
            quote_fg: "#4b5563",
            table_border: "#e5e7eb",
        })
    })
}
