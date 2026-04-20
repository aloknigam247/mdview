use mdview_render_terminal::{render_str, Registry, RenderCtx, TerminalCaps, Theme};
use mdview_render_terminal::r#box::{code_frame, table};

fn ctx() -> RenderCtx {
    RenderCtx {
        theme: Theme::default_dark(),
        terminal_caps: TerminalCaps { width: 60, height: 40, truecolor: true, sixel: false },
    }
}

#[test]
fn heading_contains_accent_bar() {
    let out = render_str("# Hello World", &ctx(), &Registry::new());
    assert!(out.contains('▌'), "heading should begin with ▌ bar, got: {:?}", out);
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
    let md = "- [ ] unchecked item\n- [x] checked item\n";
    let out = render_str(md, &ctx(), &Registry::new());
    assert!(out.contains('☐'), "unchecked marker ☐ missing: {:?}", out);
    assert!(out.contains('☑'), "checked marker ☑ missing: {:?}", out);
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
    assert!(out.contains("mdview (https://example.com)"), "link format: {:?}", out);
    assert!(out.contains("\x1b[") && out.contains("4"), "underline SGR missing");
}

#[test]
fn image_alt_placeholder() {
    let out = render_str("![diagram](x.png)", &ctx(), &Registry::new());
    assert!(out.contains("[image"), "image placeholder missing: {:?}", out);
}

#[test]
fn inline_code_background() {
    let out = render_str("use `foo` here", &ctx(), &Registry::new());
    assert!(out.contains("foo"));
    assert!(out.contains("\x1b[") && out.contains("48"), "bg SGR missing");
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
        assert!(visible_width(line) <= 20, "line {:?} exceeds width 20", line);
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
    use mdview_render_terminal::{Registry, TermChunks, TerminalRenderer};
    use comrak::nodes::{AstNode, NodeValue};

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
