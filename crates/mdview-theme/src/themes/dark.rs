use crate::_stubs::Theme;
use crate::themes::_build::{build, Palette};
use std::sync::OnceLock;

static THEME: OnceLock<Theme> = OnceLock::new();

pub fn get() -> &'static Theme {
    THEME.get_or_init(|| {
        build(Palette {
            name: "dark",
            accent: "#60a5fa",
            accent_blue: "#60a5fa",
            accent_green: "#34d399",
            accent_mauve: "#a78bfa",
            accent_peach: "#fb923c",
            accent_teal: "#22d3ee",
            accent_yellow: "#fbbf24",
            bg: "#0b1020",
            border_subtle: "#1f2a44",
            code_bg: "#111827",
            code_hl_bg: "#1f2937",
            code_inline_fg: "#fca5a5",
            fg: "#e5e7eb",
            heading: [
                "#f8fafc", "#f1f5f9", "#e2e8f0", "#cbd5e1", "#94a3b8", "#64748b",
            ],
            link: "#60a5fa",
            muted: "#9ca3af",
            quote_fg: "#cbd5e1",
            table_border: "#1f2a44",
        })
    })
}
