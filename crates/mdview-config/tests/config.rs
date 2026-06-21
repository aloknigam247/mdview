use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use mdview_config::{
    Action, CodemapConfig, Config, ConfigErrorSource, KeyBinding, Keymap, ThemeConfig, ThemeMode,
    TocConfig, TocPosition,
};

fn ev(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: mods,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

#[test]
fn key_string_parses_known_good() {
    let cases = ["Q", "Ctrl+Q", "Ctrl+Shift+F1", "Down", "Alt+PageUp"];
    for c in cases {
        let b: KeyBinding = c.parse().unwrap_or_else(|e| panic!("{c}: {e}"));
        // Round-trip via Display must reparse to the same binding.
        let s = b.to_string();
        let b2: KeyBinding = s.parse().unwrap();
        assert_eq!(b, b2, "round-trip for {c}");
    }
}

#[test]
fn key_string_rejects_bad() {
    for c in ["", "Foo", "Ctrl+", "+Q", "Ctrl+Ctrl+Q"] {
        assert!(c.parse::<KeyBinding>().is_err(), "should reject {c:?}");
    }
}

#[test]
fn modifier_order_canonicalizes() {
    let a: KeyBinding = "Shift+Ctrl+Q".parse().unwrap();
    let b: KeyBinding = "Ctrl+Shift+Q".parse().unwrap();
    assert_eq!(a, b);
}

#[test]
fn lookup_matches_configured_binding() {
    let cfg = Config::from_toml_str("[keymap]\nquit = \"Ctrl+Q\"\n");
    let km = &cfg.keymap;
    assert_eq!(
        km.lookup(&ev(KeyCode::Char('q'), KeyModifiers::CONTROL)),
        Some(Action::Quit)
    );
    assert_eq!(km.lookup(&ev(KeyCode::Char('q'), KeyModifiers::NONE)), None);
}

#[test]
fn override_default_via_toml() {
    let cfg = Config::from_toml_str("[keymap]\nquit = \"Ctrl+T\"\n");
    assert_eq!(
        cfg.keymap
            .lookup(&ev(KeyCode::Char('t'), KeyModifiers::CONTROL)),
        Some(Action::Quit)
    );
    assert_eq!(
        cfg.keymap
            .lookup(&ev(KeyCode::Char('q'), KeyModifiers::CONTROL)),
        None
    );
}

#[test]
fn malformed_toml_falls_back_to_defaults() {
    let cfg = Config::from_toml_str("this is = not [valid toml");
    assert_eq!(cfg, Config::defaults());
    assert!(cfg.keymap.bindings.is_empty());
}

#[test]
fn unknown_action_is_ignored() {
    let cfg = Config::from_toml_str("[keymap]\nfly = \"Ctrl+F\"\n");
    assert!(cfg.keymap.bindings.is_empty());
}

#[test]
fn unparseable_binding_is_ignored() {
    let cfg = Config::from_toml_str("[keymap]\nquit = \"Foo+Bar\"\n");
    assert!(cfg.keymap.bindings.is_empty());
}

#[test]
fn defaults_are_empty() {
    let km = Keymap::defaults();
    assert!(km.bindings.is_empty());
    assert_eq!(
        km.lookup(&ev(KeyCode::Char('q'), KeyModifiers::CONTROL)),
        None
    );
}

#[test]
fn commented_quit_yields_no_binding() {
    let cfg = Config::from_toml_str(mdview_config::DEFAULT_CONFIG_TOML);
    assert!(cfg.keymap.bindings.is_empty());
    for code in [KeyCode::Char('q'), KeyCode::Esc, KeyCode::Enter] {
        for mods in [
            KeyModifiers::NONE,
            KeyModifiers::CONTROL,
            KeyModifiers::ALT,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ] {
            assert_eq!(cfg.keymap.lookup(&ev(code, mods)), None);
        }
    }
}

#[test]
fn defaults_include_toc_and_codemap() {
    let cfg = Config::defaults();
    assert_eq!(cfg.toc, TocConfig::default());
    assert_eq!(cfg.toc.position, TocPosition::FloatingRight);
    assert_eq!(cfg.toc.depth, 3);
    assert_eq!(cfg.codemap, CodemapConfig::default());
    assert!(cfg.codemap.enabled);
}

#[test]
fn toc_position_fixed_left_parses() {
    let cfg = Config::from_toml_str("[toc]\nposition = \"fixed-left\"\n");
    assert_eq!(cfg.toc.position, TocPosition::FixedLeft);
    assert_eq!(cfg.toc.depth, 3);
}

#[test]
fn toc_position_floating_center_parses() {
    let cfg = Config::from_toml_str("[toc]\nposition = \"floating-center\"\n");
    assert_eq!(cfg.toc.position, TocPosition::FloatingCenter);
}

#[test]
fn toc_position_inline_parses() {
    let cfg = Config::from_toml_str("[toc]\nposition = \"inline\"\n");
    assert_eq!(cfg.toc.position, TocPosition::Inline);
}

#[test]
fn toc_depth_in_range_parses() {
    let cfg = Config::from_toml_str("[toc]\ndepth = 5\n");
    assert_eq!(cfg.toc.depth, 5);
}

#[test]
fn toc_depth_zero_clamps_up() {
    let cfg = Config::from_toml_str("[toc]\ndepth = 0\n");
    assert_eq!(cfg.toc.depth, 1);
}

#[test]
fn toc_depth_over_six_clamps_down() {
    let cfg = Config::from_toml_str("[toc]\ndepth = 9\n");
    assert_eq!(cfg.toc.depth, 6);
}

#[test]
fn codemap_disabled_parses() {
    let cfg = Config::from_toml_str("[codemap]\nenabled = false\n");
    assert!(!cfg.codemap.enabled);
}

#[test]
fn toggle_bionic_keybinding_parses() {
    let cfg = Config::from_toml_str("[keymap]\ntoggle-bionic = \"Ctrl+Shift+B\"\n");
    assert_eq!(
        cfg.keymap.lookup(&ev(
            KeyCode::Char('b'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT
        )),
        Some(Action::ToggleBionic)
    );
}

#[test]
fn toggle_toc_keybinding_parses() {
    let cfg = Config::from_toml_str("[keymap]\ntoggle-toc = \"Ctrl+B\"\n");
    assert_eq!(
        cfg.keymap
            .lookup(&ev(KeyCode::Char('b'), KeyModifiers::CONTROL)),
        Some(Action::ToggleToc)
    );
}

#[test]
fn toggle_codemap_keybinding_parses() {
    let cfg = Config::from_toml_str("[keymap]\ntoggle-codemap = \"Ctrl+M\"\n");
    assert_eq!(
        cfg.keymap
            .lookup(&ev(KeyCode::Char('m'), KeyModifiers::CONTROL)),
        Some(Action::ToggleCodemap)
    );
}

#[test]
fn collects_all_errors_from_multi_problem_toml() {
    let toml = "[keymap]\n\
                quit = \"Ctrr+Q\"\n\
                togle-theme = \"Ctrl+T\"\n\
                [toc]\n\
                depth = 9\n";
    let res = Config::from_toml_str_full(toml);
    assert_eq!(res.errors.len(), 3, "errors: {:?}", res.errors);
    let lines: Vec<String> = res.errors.iter().map(|e| e.to_string()).collect();
    let joined = lines.join("\n");
    assert!(
        joined.contains("keymap[quit]") && joined.contains("Ctrr"),
        "expected keymap[quit]/Ctrr line, got:\n{joined}"
    );
    assert!(
        joined.contains("keymap[togle-theme]") && joined.contains("unknown action"),
        "expected unknown-action line, got:\n{joined}"
    );
    assert!(
        joined.contains("[toc] depth") && joined.contains("1..=6"),
        "expected toc depth range line, got:\n{joined}"
    );
}

#[test]
fn keymap_unknown_modifier_message_lists_valid_modifiers() {
    let res = Config::from_toml_str_full("[keymap]\nquit = \"Ctrr+Q\"\n");
    let e = &res.errors[0];
    assert_eq!(e.source, ConfigErrorSource::Keymap);
    let s = e.to_string();
    assert!(s.contains("\"Ctrr\""));
    assert!(s.contains("Ctrl"));
    assert!(s.contains("Shift"));
    assert!(s.contains("Alt"));
    assert!(s.contains("Super"));
}

#[test]
fn keymap_unknown_action_message_lists_valid_actions() {
    let res = Config::from_toml_str_full("[keymap]\ntogle-theme = \"Ctrl+T\"\n");
    let e = &res.errors[0];
    let s = e.to_string();
    assert!(s.contains("keymap[togle-theme]"));
    assert!(s.contains("unknown action"));
    assert!(s.contains("quit"));
    assert!(s.contains("toggle-codemap"));
    assert!(s.contains("toggle-toc"));
}

#[test]
fn keymap_empty_binding_is_an_error() {
    let res = Config::from_toml_str_full("[keymap]\nquit = \"\"\n");
    let s = res.errors[0].to_string();
    assert!(s.contains("keymap[quit]"));
    assert!(s.contains("empty binding"));
}

#[test]
fn toc_depth_out_of_range_emits_error() {
    let res = Config::from_toml_str_full("[toc]\ndepth = 9\n");
    let s = res.errors[0].to_string();
    assert!(s.contains("[toc] depth"));
    assert!(s.contains("1..=6"));
    assert_eq!(res.config.toc.depth, 6);
}

#[test]
fn malformed_toml_includes_line_col() {
    let toml = "[toc]\n#good comment\nthis is = not [valid toml";
    let res = Config::from_toml_str_full(toml);
    assert_eq!(res.errors.len(), 1);
    let s = res.errors[0].to_string();
    assert!(s.contains("config.toml line"), "got: {s}");
    assert!(s.contains("invalid TOML"));
}

#[test]
fn loadresult_for_valid_toml_has_no_errors() {
    let res = Config::from_toml_str_full("[keymap]\nquit = \"Ctrl+Q\"\n");
    assert!(res.errors.is_empty());
}

#[test]
fn action_round_trip_via_str() {
    for a in Action::ALL {
        let s = a.as_str();
        let parsed: Action = s.parse().unwrap();
        assert_eq!(parsed, *a);
    }
}

#[test]
fn defaults_include_theme() {
    let cfg = Config::defaults();
    assert_eq!(cfg.theme, ThemeConfig::default());
    assert_eq!(cfg.theme.mode, ThemeMode::Auto);
    assert_eq!(cfg.theme.light, "catppuccin-latte");
    assert_eq!(cfg.theme.dark, "catppuccin-mocha");
}

#[test]
fn theme_mode_light_parses() {
    let cfg = Config::from_toml_str("[theme]\nmode = \"light\"\n");
    assert_eq!(cfg.theme.mode, ThemeMode::Light);
}

#[test]
fn theme_mode_dark_parses() {
    let cfg = Config::from_toml_str("[theme]\nmode = \"dark\"\n");
    assert_eq!(cfg.theme.mode, ThemeMode::Dark);
}

#[test]
fn theme_light_override_parses() {
    let cfg = Config::from_toml_str("[theme]\nlight = \"custom\"\n");
    assert_eq!(cfg.theme.light, "custom");
    assert_eq!(cfg.theme.dark, "catppuccin-mocha");
}

#[test]
fn theme_dark_override_parses() {
    let cfg = Config::from_toml_str("[theme]\ndark = \"custom\"\n");
    assert_eq!(cfg.theme.dark, "custom");
    assert_eq!(cfg.theme.light, "catppuccin-latte");
}

#[test]
fn toggle_theme_keybinding_parses() {
    let cfg = Config::from_toml_str("[keymap]\ntoggle-theme = \"Ctrl+T\"\n");
    assert_eq!(
        cfg.keymap
            .lookup(&ev(KeyCode::Char('t'), KeyModifiers::CONTROL)),
        Some(Action::ToggleTheme)
    );
}

#[test]
fn theme_errors_collected_alongside_other_sections() {
    let toml = "[theme]\nmode = \"sepia\"\n[toc]\ndepth = 9\n";
    let res = Config::from_toml_str_full(toml);
    assert_eq!(res.errors.len(), 2, "errors: {:?}", res.errors);
    let joined: String = res
        .errors
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("[theme] mode"), "got:\n{joined}");
    assert!(joined.contains("[toc] depth"), "got:\n{joined}");
    assert_eq!(res.config.theme.mode, ThemeMode::Auto);
    assert_eq!(res.config.toc.depth, 6);
    assert!(res
        .errors
        .iter()
        .any(|e| e.source == ConfigErrorSource::Theme));
}

#[test]
fn theme_mode_light_resolve_is_light() {
    assert!(ThemeMode::Light.resolve_is_light());
}

#[test]
fn theme_mode_dark_resolve_is_light() {
    assert!(!ThemeMode::Dark.resolve_is_light());
}

#[test]
fn code_tab_width_default_is_4() {
    let cfg = Config::defaults();
    assert_eq!(cfg.code.tab_width, 4);
}

#[test]
fn code_tab_width_parses() {
    let cfg = Config::from_toml_str("[code]\ntab_width = 2\n");
    assert_eq!(cfg.code.tab_width, 2);
}

#[test]
fn code_tab_width_out_of_range_clamps() {
    let res = Config::from_toml_str_full("[code]\ntab_width = 12\n");
    assert_eq!(res.config.code.tab_width, 8);
    assert_eq!(res.errors.len(), 1);
    assert!(res.errors[0].to_string().contains("1..=8"));
}

#[test]
fn validation_error_carries_source_line() {
    let toml = "[toc]\n#comment\ndepth = 9\n";
    let res = Config::from_toml_str_full(toml);
    assert_eq!(res.errors.len(), 1);
    assert_eq!(
        res.errors[0].line,
        Some(3),
        "expected line 3 for depth on line 3; got {:?}",
        res.errors[0].line
    );
}

#[test]
fn display_line_formats_path_line_em_dash_message() {
    let toml = "[toc]\ndepth = 9\n";
    let res = Config::from_toml_str_full(toml);
    let path = std::path::PathBuf::from("/tmp/config.toml");
    let s = res.errors[0].display_line(&path);
    assert!(s.starts_with("/tmp/config.toml:2 \u{2014}"), "got: {s}");
    assert!(s.contains("[toc] depth"), "got: {s}");
    assert!(s.contains("1..=6"), "got: {s}");
}

#[test]
fn malformed_toml_display_line_uses_parser_line() {
    let toml = "[toc]\n#good\nthis is = not [valid toml";
    let res = Config::from_toml_str_full(toml);
    let path = std::path::PathBuf::from("/tmp/config.toml");
    let s = res.errors[0].display_line(&path);
    assert!(s.starts_with("/tmp/config.toml:"), "got: {s}");
    assert!(s.contains(" \u{2014} "), "got: {s}");
    assert!(s.contains("invalid TOML"), "got: {s}");
}

#[test]
fn code_tab_width_not_integer_is_error() {
    let res = Config::from_toml_str_full("[code]\ntab_width = \"four\"\n");
    assert_eq!(res.config.code.tab_width, 4);
    assert_eq!(res.errors.len(), 1);
    assert_eq!(res.errors[0].source, ConfigErrorSource::Code);
}
