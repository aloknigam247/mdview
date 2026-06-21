use mdview_render_terminal::r#box::{code_frame, table};
use mdview_render_terminal::{render_str, Registry, RenderCtx, TerminalCaps, Theme};

fn ctx() -> RenderCtx {
    RenderCtx {
        theme: Theme::default_dark(),
        source_dir: None,
        terminal_caps: TerminalCaps {
            width: 60,
            height: 40,
            truecolor: true,
            sixel: false,
        },
    }
}

#[test]
fn heading_contains_accent_bar() {
    let out = render_str("# Hello World", &ctx(), &Registry::new());
    assert!(
        out.contains('▌'),
        "heading should begin with ▌ bar, got: {:?}",
        out
    );
    assert!(out.contains("Hello World"), "content missing");
    assert!(out.contains("\x1b[1"), "bold SGR missing");
}

#[test]
fn blockquote_has_bar_prefix() {
    let out = render_str("> quoted line", &ctx(), &Registry::new());
    assert!(out.contains('▎'), "blockquote bar ▎ missing: {:?}", out);
    assert!(out.contains("quoted line"));
}

#[test]
fn tasklist_markers_present() {
    let md = "- [x] done\n- [ ] todo\n";
    let out = render_str(md, &ctx(), &Registry::new());
    assert!(
        out.contains('\u{F05E1}'),
        "checked nerd-font icon U+F05E1 missing: {:?}",
        out
    );
    assert!(
        out.contains('\u{F0130}'),
        "unchecked nerd-font icon U+F0130 missing: {:?}",
        out
    );
    assert!(
        !out.contains('☑') && !out.contains('☐'),
        "legacy ballot box glyphs leaked: {:?}",
        out
    );
}

#[test]
fn tasklist_does_not_affect_plain_bullets() {
    let out = render_str("- alpha\n- beta\n", &ctx(), &Registry::new());
    assert!(out.contains('•'), "bullet missing: {:?}", out);
    assert!(
        !out.contains('\u{F05E1}') && !out.contains('\u{F0130}'),
        "task icons should not appear for plain bullets: {:?}",
        out
    );
}

#[test]
fn bullet_list_markers() {
    let out = render_str("- one\n- two\n", &ctx(), &Registry::new());
    assert!(out.contains('•'), "bullet missing: {:?}", out);
}

#[test]
fn ordered_list_numbers() {
    let out = render_str("1. first\n2. second\n", &ctx(), &Registry::new());
    assert!(out.contains("1."));
    assert!(out.contains("2."));
}

#[test]
fn thematic_break_has_dashes() {
    let out = render_str("---\n", &ctx(), &Registry::new());
    assert!(out.contains('─'), "thematic break ─ missing: {:?}", out);
}

#[test]
fn link_renders_url_in_parens() {
    let out = render_str("[mdview](https://example.com)", &ctx(), &Registry::new());
    assert!(
        out.contains("mdview (https://example.com)"),
        "link format: {:?}",
        out
    );
    assert!(
        out.contains("\x1b[") && out.contains("4"),
        "underline SGR missing"
    );
}

#[test]
fn image_alt_placeholder() {
    let out = render_str("![diagram](x.png)", &ctx(), &Registry::new());
    assert!(
        out.contains("[image"),
        "image placeholder missing: {:?}",
        out
    );
}

#[test]
fn image_without_sixel_support_emits_placeholder() {
    let tmp = std::env::temp_dir().join("mdview-term-img-tests");
    std::fs::create_dir_all(&tmp).unwrap();
    let img = tmp.join("y.png");
    std::fs::write(&img, b"fakepng").unwrap();
    let c = RenderCtx {
        theme: Theme::default_dark(),
        source_dir: Some(tmp),
        terminal_caps: TerminalCaps {
            width: 60,
            height: 40,
            truecolor: true,
            sixel: false,
        },
    };
    let out = render_str("![diagram](y.png)", &c, &Registry::new());
    assert!(
        out.contains("[image: diagram]"),
        "placeholder missing: {:?}",
        out
    );
    assert!(
        !out.contains("\x1bPq"),
        "sixel emitted unexpectedly: {:?}",
        out
    );
}

#[test]
fn image_with_sixel_support_emits_sixel_bytes() {
    use mdview_render_terminal::image::encode_local_to_sixel;
    let tmp = std::env::temp_dir().join("mdview-term-img-tests");
    std::fs::create_dir_all(&tmp).unwrap();
    let img = tmp.join("z.png");
    let mut buf = image::RgbaImage::new(8, 8);
    for px in buf.pixels_mut() {
        *px = image::Rgba([10, 200, 30, 255]);
    }
    let dyn_img = image::DynamicImage::ImageRgba8(buf);
    let mut out_bytes = std::io::Cursor::new(Vec::new());
    dyn_img
        .write_to(&mut out_bytes, image::ImageFormat::Png)
        .unwrap();
    std::fs::write(&img, out_bytes.into_inner()).unwrap();

    let sixel = encode_local_to_sixel(&img, true).expect("sixel encode");
    assert!(sixel.starts_with("\x1bP"), "missing DCS introducer");
    assert!(sixel.contains('q'), "missing q in DCS prelude");
    assert!(sixel.ends_with("\x1b\\"), "missing ST terminator");
}

#[test]
fn image_oversize_is_skipped() {
    use mdview_render_terminal::image::encode_local_to_sixel;
    let tmp = std::env::temp_dir().join("mdview-term-img-tests");
    std::fs::create_dir_all(&tmp).unwrap();
    let huge = tmp.join("huge.png");
    let big = vec![0u8; 11 * 1024 * 1024];
    std::fs::write(&huge, &big).unwrap();
    assert!(encode_local_to_sixel(&huge, true).is_none());
}

#[test]
fn inline_code_background() {
    let out = render_str("use `foo` here", &ctx(), &Registry::new());
    assert!(out.contains("foo"));
    assert!(
        out.contains("\x1b[") && out.contains("48"),
        "bg SGR missing"
    );
}

#[test]
fn table_rounded_output() {
    let theme = Theme::default_dark();
    let headers = vec!["a".to_string(), "b".to_string()];
    let rows = vec![
        vec!["1".to_string(), "2".to_string()],
        vec!["3".to_string(), "4".to_string()],
    ];
    let chunks = table(&headers, &rows, &theme);
    let out = chunks.to_ansi();
    for ch in ['╭', '┬', '╮', '├', '┼', '┤', '╰', '┴', '╯', '│', '─'] {
        assert!(out.contains(ch), "table missing {}: {:?}", ch, out);
    }
}

#[test]
fn code_frame_has_rounded_corners() {
    let theme = Theme::default_dark();
    let chunks = code_frame(Some("rust"), "fn main() {}", &theme);
    let out = chunks.to_ansi();
    for ch in ['╭', '╮', '╰', '╯', '│', '─'] {
        assert!(out.contains(ch), "code frame missing {}: {:?}", ch, out);
    }
    assert!(out.contains("rust"), "lang label missing");
}

#[test]
fn wrap_respects_width() {
    use mdview_render_terminal::wrap::{visible_width, wrap_ansi};
    let text = "the quick brown fox jumps over the lazy dog multiple times repeatedly here";
    let lines = wrap_ansi(text, 20);
    assert!(lines.len() > 1, "should wrap into multiple lines");
    for line in &lines {
        assert!(
            visible_width(line) <= 20,
            "line {:?} exceeds width 20",
            line
        );
    }
}

#[test]
fn wrap_preserves_ansi_escapes() {
    use mdview_render_terminal::wrap::wrap_ansi;
    let text = "\x1b[1mhello world this is a long styled line\x1b[0m";
    let lines = wrap_ansi(text, 15);
    for line in &lines {
        assert!(line.contains("\x1b["), "lost ANSI escape in {:?}", line);
    }
}

#[test]
fn registry_override_replaces_default() {
    use comrak::nodes::{AstNode, NodeValue};
    use mdview_render_terminal::{Registry, TermChunks, TerminalRenderer};

    struct AllCaps;
    impl TerminalRenderer for AllCaps {
        fn render_terminal<'a>(
            &self,
            node: &'a AstNode<'a>,
            _ctx: &RenderCtx,
        ) -> Option<TermChunks> {
            if matches!(node.data.borrow().value, NodeValue::Heading(_)) {
                let mut c = TermChunks::new();
                c.push_plain("OVERRIDE");
                Some(c)
            } else {
                None
            }
        }
    }

    let mut registry = Registry::new();
    registry.register_terminal(Box::new(AllCaps));
    let out = render_str("# original", &ctx(), &registry);
    assert!(out.contains("OVERRIDE"));
    assert!(!out.contains("original"));
}

#[test]
fn emphasis_and_strong_styles() {
    let out = render_str("*emph* and **strong**", &ctx(), &Registry::new());
    assert!(out.contains("emph"));
    assert!(out.contains("strong"));
    assert!(out.contains("\x1b[3"), "italic SGR missing: {:?}", out);
    assert!(out.contains("\x1b[1"), "bold SGR missing: {:?}", out);
}
