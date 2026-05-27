use mdview_theme::ansi::{parse_hex, style_for, style_for_depth, ColorDepth};
use mdview_theme::nvim::{theme_from_nvim_highlights, NvimHl};
use mdview_theme::{builtin_themes, cache, emit_css, find};
use std::collections::BTreeMap;

#[test]
fn all_presets_have_required_keys() {
    let expected_colors = [
        "accent",
        "bg",
        "border.subtle",
        "code.bg",
        "code.hl-bg",
        "fg",
        "link",
        "muted",
        "table.border",
    ];
    let expected_styles = [
        "blockquote",
        "code.inline",
        "emphasis",
        "heading.1",
        "heading.2",
        "heading.3",
        "heading.4",
        "heading.5",
        "heading.6",
        "link",
        "list.marker",
        "strong",
        "table.header",
    ];
    let themes = builtin_themes();
    assert_eq!(themes.len(), 6);
    for t in themes {
        for k in expected_colors {
            assert!(t.colors.contains_key(k), "{} missing color {k}", t.name);
        }
        for k in expected_styles {
            assert!(t.styles.contains_key(k), "{} missing style {k}", t.name);
        }
        assert!(t.radii.sm > 0 && t.radii.md > 0 && t.radii.lg > 0);
    }
}

#[test]
fn code_hl_bg_values_are_correct() {
    let mocha = find("catppuccin-mocha").unwrap();
    assert_eq!(mocha.colors.get("code.hl-bg").unwrap(), "#45475a");
    let latte = find("catppuccin-latte").unwrap();
    assert_eq!(latte.colors.get("code.hl-bg").unwrap(), "#bcc0cc");
}

#[test]
fn presets_are_alphabetical() {
    let names: Vec<&str> = builtin_themes().iter().map(|t| t.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted);
}

#[test]
fn find_returns_known_theme() {
    assert!(find("dracula").is_some());
    assert!(find("nonsuch").is_none());
}

#[test]
fn css_contains_radii_and_custom_properties() {
    let t = find("light").unwrap();
    let css = emit_css(t);
    assert!(!css.is_empty());
    assert!(css.contains(":root"));
    assert!(css.contains("--mdv-fg:"));
    assert!(css.contains("--mdv-bg:"));
    assert!(css.contains(&format!("--mdv-radius-md: {}px", t.radii.md)));
    assert!(css.contains(&format!("--mdv-radius-sm: {}px", t.radii.sm)));
    assert!(css.contains(&format!("--mdv-radius-lg: {}px", t.radii.lg)));
    assert!(css.contains("line-height: 1.7"));
    assert!(css.contains("ui-monospace"));
}

#[test]
fn ansi_emits_truecolor_sgr() {
    let t = find("dark").unwrap();
    let s = style_for(t, "heading.1");
    assert!(s.prefix.starts_with("\x1b["));
    assert!(s.prefix.contains("38;2;"));
    assert!(s.prefix.ends_with('m'));
    assert_eq!(s.suffix, "\x1b[0m");
}

#[test]
fn ansi_256_fallback() {
    let t = find("dark").unwrap();
    let s = style_for_depth(t, "link", ColorDepth::Palette256);
    assert!(s.prefix.contains("38;5;"));
    assert!(!s.prefix.contains("38;2;"));
}

#[test]
fn ansi_unknown_key_is_empty() {
    let t = find("light").unwrap();
    let s = style_for(t, "does.not.exist");
    assert_eq!(s.prefix, "");
    assert_eq!(s.suffix, "");
}

#[test]
fn parse_hex_round_trip() {
    assert_eq!(parse_hex("#112233"), Some((0x11, 0x22, 0x33)));
    assert_eq!(parse_hex("ffffff"), Some((255, 255, 255)));
    assert_eq!(parse_hex("#zz0000"), None);
    assert_eq!(parse_hex("#abc"), None);
}

#[test]
fn nvim_mapper_builds_theme_from_minimal_hl() {
    let mut hl: BTreeMap<String, NvimHl> = BTreeMap::new();
    hl.insert(
        "Normal".into(),
        NvimHl {
            fg: Some(0xe5e7eb),
            bg: Some(0x0b1020),
            ..Default::default()
        },
    );
    hl.insert(
        "Function".into(),
        NvimHl {
            fg: Some(0x60a5fa),
            ..Default::default()
        },
    );
    hl.insert(
        "Comment".into(),
        NvimHl {
            fg: Some(0x9ca3af),
            italic: true,
            ..Default::default()
        },
    );
    hl.insert(
        "String".into(),
        NvimHl {
            fg: Some(0x111827),
            ..Default::default()
        },
    );
    hl.insert(
        "@markup.heading.1".into(),
        NvimHl {
            fg: Some(0xf8fafc),
            bold: true,
            ..Default::default()
        },
    );

    let t = theme_from_nvim_highlights("my-scheme", &hl);
    assert_eq!(t.name, "my-scheme");
    assert_eq!(t.colors.get("fg").unwrap(), "#e5e7eb");
    assert_eq!(t.colors.get("bg").unwrap(), "#0b1020");
    assert_eq!(t.colors.get("accent").unwrap(), "#60a5fa");
    assert_eq!(t.colors.get("muted").unwrap(), "#9ca3af");
    assert_eq!(
        t.styles.get("heading.1").and_then(|s| s.fg.as_deref()),
        Some("#f8fafc")
    );
}

#[test]
fn nvim_mapper_falls_back_when_hl_empty() {
    let hl: BTreeMap<String, NvimHl> = BTreeMap::new();
    let t = theme_from_nvim_highlights("default", &hl);
    assert!(!t.colors.get("fg").unwrap().is_empty());
    assert!(!t.colors.get("bg").unwrap().is_empty());
}

#[test]
fn cache_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    cache::set_cache_dir_for_tests(tmp.path().to_path_buf());

    let original = find("solarized").unwrap().clone();
    let key = cache::cache_key("solarized", "0.10.0");

    assert!(cache::load(&key).is_none());
    cache::store(&key, &original).unwrap();
    let loaded = cache::load(&key).expect("theme should load");
    assert_eq!(loaded, original);

    cache::clear_all().unwrap();
    assert!(cache::load(&key).is_none());
}
