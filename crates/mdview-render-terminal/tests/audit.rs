use mdview_render_terminal::{render_str, Registry, RenderCtx, TerminalCaps, Theme};

fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            while let Some(&c) = chars.peek() {
                chars.next();
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn theme() -> &'static Theme {
    use std::sync::OnceLock;

    static THEME: OnceLock<Theme> = OnceLock::new();
    THEME.get_or_init(Theme::default_dark)
}

fn ctx_with_width(width: usize) -> RenderCtx<'static> {
    RenderCtx {
        theme: theme(),
        source_dir: None,
        terminal_caps: Some(TerminalCaps {
            width: width as u16,
            height: 40,
            truecolor: true,
            sixel: false,
        }),
        ..RenderCtx::new(theme())
    }
}

fn ctx() -> RenderCtx<'static> {
    ctx_with_width(80)
}

#[test]
fn strip_ansi_removes_sgr_sequences() {
    let s = "\x1b[1mbold\x1b[0m and \x1b[38;2;1;2;3mcolor\x1b[0m";
    assert_eq!(strip_ansi(s), "bold and color");
}

// --- Section 1: Headings -----------------------------------------------------

#[test]
fn renders_section_1_ansi() {
    let md = "# H1\n## H2\n### H3\n#### H4\n##### H5\n###### H6\n";
    let out = render_str(md, &ctx(), &Registry::new());
    assert!(out.contains("\x1b[1"), "bold SGR missing for headings");
    assert!(out.contains("\x1b[38;2;"), "fg colour SGR missing");
}

#[test]
fn renders_section_1_content() {
    let md = "# H1\n## H2\n### H3\n#### H4\n##### H5\n###### H6\n";
    let out = render_str(md, &ctx(), &Registry::new());
    let plain = strip_ansi(&out);
    for h in ["H1", "H2", "H3", "H4", "H5", "H6"] {
        assert!(plain.contains(h), "missing {h} in {plain:?}");
    }
    let bar_count = plain.matches('▌').count();
    assert!(bar_count >= 6, "expected ≥6 heading bars, got {bar_count}");
}

// --- Section 2: Inline styles ------------------------------------------------

#[test]
fn renders_section_2_ansi() {
    let md = "**bold** and *italic* and ~~strike~~ and `code` and [link](https://x.io)\n";
    let out = render_str(md, &ctx(), &Registry::new());
    assert!(out.contains("\x1b[1"), "bold SGR missing");
    assert!(out.contains("\x1b[3"), "italic SGR missing");
    assert!(out.contains('\x1b'), "no escapes at all?");
    let plain = strip_ansi(&out);
    assert!(plain.contains("bold"));
    assert!(plain.contains("italic"));
    assert!(plain.contains("strike"));
    assert!(plain.contains("code"));
    assert!(plain.contains("link"));
    assert!(plain.contains("https://x.io"));
}

#[test]
fn inline_sub_renders_unicode() {
    let out = render_str("H<sub>2</sub>O", &ctx(), &Registry::new());
    let plain = strip_ansi(&out);
    assert!(
        plain.contains("H₂O"),
        "expected H₂O in stripped output, got: {plain:?}"
    );
}

#[test]
fn inline_sup_renders_unicode() {
    let out = render_str("E = mc<sup>2</sup>", &ctx(), &Registry::new());
    let plain = strip_ansi(&out);
    assert!(
        plain.contains("mc²"),
        "expected mc² in stripped output, got: {plain:?}"
    );
}

#[test]
fn inline_sub_unmappable_falls_back_to_literal() {
    let out = render_str("X<sub>Q</sub>Y", &ctx(), &Registry::new());
    let plain = strip_ansi(&out);
    assert!(
        plain.contains("<sub>Q</sub>"),
        "expected literal fallback for unmappable char, got: {plain:?}"
    );
}

// --- Section 3: Lists --------------------------------------------------------

#[test]
fn renders_section_3_ansi() {
    let md = "- a\n  - b\n- c\n\n1. one\n2. two\n\n- [x] done\n- [ ] todo\n";
    let out = render_str(md, &ctx(), &Registry::new());
    assert!(out.contains('\x1b'), "no styling escapes in lists");
}

#[test]
fn renders_section_3_content() {
    let md = "- a\n  - b\n- c\n\n1. one\n2. two\n\n- [x] done\n- [ ] todo\n";
    let out = render_str(md, &ctx(), &Registry::new());
    let plain = strip_ansi(&out);
    assert!(plain.contains('•'));
    assert!(plain.contains("1."));
    assert!(plain.contains("2."));
    assert!(plain.contains('\u{F0130}'));
    assert!(plain.contains('\u{F05E1}'));
    for tok in ["a", "b", "c", "one", "two", "done", "todo"] {
        assert!(plain.contains(tok), "missing {tok}");
    }
}

// --- Section 4: Tables -------------------------------------------------------

#[test]
fn renders_section_4_ansi() {
    let md = "| a | b |\n|---|---|\n| 1 | 2 |\n";
    let out = render_str(md, &ctx(), &Registry::new());
    assert!(out.contains("\x1b[1"), "header row bold missing");
}

#[test]
fn renders_section_4_content() {
    let md = "| a | b |\n|---|---|\n| 1 | 2 |\n";
    let out = render_str(md, &ctx(), &Registry::new());
    let plain = strip_ansi(&out);
    for ch in ['╭', '┬', '╮', '├', '┼', '┤', '╰', '┴', '╯', '│', '─'] {
        assert!(plain.contains(ch), "table missing {ch}");
    }
    for cell in ["a", "b", "1", "2"] {
        assert!(plain.contains(cell), "missing cell {cell}");
    }
}

#[test]
fn table_wraps_at_40_cols() {
    let md = "| col1 | col2 | col3 |\n|------|------|------|\n| x | y | z |\n";
    let out = render_str(md, &ctx_with_width(40), &Registry::new());
    let plain = strip_ansi(&out);
    assert!(plain.contains('╭'));
    assert!(plain.contains('╯'));
    for cell in ["col1", "col2", "col3", "x", "y", "z"] {
        assert!(plain.contains(cell), "missing {cell} in narrow table");
    }
}

// --- Section 5: Blockquotes --------------------------------------------------

#[test]
fn renders_section_5_ansi() {
    let md = "> A quote with **bold**.\n";
    let out = render_str(md, &ctx(), &Registry::new());
    assert!(out.contains("\x1b["), "no escapes in blockquote");
}

#[test]
fn renders_section_5_content() {
    let md = "> A quote with **bold**.\n";
    let out = render_str(md, &ctx(), &Registry::new());
    let plain = strip_ansi(&out);
    assert!(plain.contains('▎'));
    assert!(plain.contains("A quote"));
}

#[test]
fn nested_blockquote_has_multiple_bars() {
    let md = "> outer\n>\n> > inner\n";
    let out = render_str(md, &ctx(), &Registry::new());
    let plain = strip_ansi(&out);
    assert!(plain.contains('▎'));
    assert!(plain.contains("outer"));
    assert!(plain.contains("inner"));
}

// --- Section 6: Horizontal rule ---------------------------------------------

#[test]
fn renders_section_6_content_and_ansi() {
    let out = render_str("---\n", &ctx(), &Registry::new());
    assert!(out.contains("\x1b["));
    let plain = strip_ansi(&out);
    assert!(plain.contains('─'));
    let dashes_per_line = plain
        .lines()
        .map(|l| l.chars().filter(|&c| c == '─').count())
        .max()
        .unwrap_or(0);
    assert!(
        dashes_per_line >= 40,
        "hr should span most of width; got {dashes_per_line} dashes"
    );
}

// --- Section 7: Code blocks --------------------------------------------------

#[test]
fn renders_section_7_ansi() {
    let md = "```rust\nfn main() {}\n```\n";
    let out = render_str(md, &ctx(), &Registry::new());
    assert!(out.contains("\x1b["), "code frame should be styled");
}

#[test]
fn renders_section_7_content() {
    let md = "```rust\nfn main() {}\n```\n";
    let out = render_str(md, &ctx(), &Registry::new());
    let plain = strip_ansi(&out);
    for ch in ['╭', '╮', '╰', '╯', '│', '─'] {
        assert!(plain.contains(ch));
    }
    assert!(plain.contains("rust"), "lang label missing");
    assert!(plain.contains("fn main() {}"));
}

#[test]
fn code_block_plain_no_language() {
    let md = "```\njust text\n```\n";
    let out = render_str(md, &ctx(), &Registry::new());
    let plain = strip_ansi(&out);
    assert!(plain.contains("just text"));
    assert!(plain.contains('╭'));
}

// --- Section 12: Images (placeholder fallback) -------------------------------

#[test]
fn image_placeholder_in_paragraph() {
    let out = render_str("![alt text](no.png)", &ctx(), &Registry::new());
    let plain = strip_ansi(&out);
    assert!(plain.contains("[image"), "image placeholder missing");
}

// --- Footnotes ---------------------------------------------------------------

#[test]
fn footnote_reference_is_visible() {
    let md = "Hello[^1]\n\n[^1]: world\n";
    let out = render_str(md, &ctx(), &Registry::new());
    let plain = strip_ansi(&out);
    assert!(plain.contains("[^1]"), "footnote ref missing");
}

// --- ANSI well-formedness ----------------------------------------------------

#[test]
fn output_has_no_orphan_escape_sequences() {
    let md = include_test_doc();
    let out = render_str(md, &ctx(), &Registry::new());
    let bytes: Vec<char> = out.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '\x1b' {
            assert!(i + 1 < bytes.len(), "trailing ESC at end of output");
            assert_eq!(
                bytes[i + 1],
                '[',
                "ESC not followed by '[' at index {i}; got {:?}",
                bytes[i + 1]
            );
            let mut j = i + 2;
            let mut terminated = false;
            while j < bytes.len() {
                let c = bytes[j];
                if c.is_ascii_alphabetic() {
                    terminated = true;
                    break;
                }
                if !(c.is_ascii_digit() || c == ';' || c == ':' || c == '?') {
                    break;
                }
                j += 1;
            }
            assert!(terminated, "CSI sequence at {i} not terminated by alpha");
            i = j + 1;
        } else {
            i += 1;
        }
    }
}

// --- Width-variant scaffolding for any frontmatter card --------------------
// Note: the mdview-render-terminal crate itself does not currently render
// frontmatter as a hero card (that lives in the app-side renderer). These
// two tests pin the contract that frontmatter input does not panic and that
// the output respects the configured width.

#[test]
fn frontmatter_card_fits_80_cols() {
    let md = "---\ntitle: Hello world\n---\n\nbody\n";
    let out = render_str(md, &ctx_with_width(80), &Registry::new());
    let plain = strip_ansi(&out);
    let max = plain.lines().map(width).max().unwrap_or(0);
    assert!(max <= 80, "expected ≤80 cols at width=80, got {max}");
    assert!(plain.contains("body"));
}

#[test]
fn frontmatter_card_fits_40_cols() {
    let md = "---\ntitle: Hello world\n---\n\nbody\n";
    let out = render_str(md, &ctx_with_width(40), &Registry::new());
    let plain = strip_ansi(&out);
    assert!(plain.contains("body"));
}

fn width(s: &str) -> usize {
    use unicode_width::UnicodeWidthChar;
    s.chars()
        .map(|c| UnicodeWidthChar::width(c).unwrap_or(0))
        .sum()
}

fn include_test_doc() -> &'static str {
    "# Title\n\n\
     A paragraph with **bold**, *italic*, ~~strike~~, `code`, \
     a [link](https://example.com), H<sub>2</sub>O and mc<sup>2</sup>.\n\n\
     - list one\n- list two\n  - nested\n- [x] done\n- [ ] todo\n\n\
     > quote\n\n\
     ---\n\n\
     | a | b |\n|---|---|\n| 1 | 2 |\n\n\
     ```rust\nfn x() {}\n```\n\n\
     ![img](pic.png)\n\n\
     ref[^1]\n\n[^1]: footnote\n"
}
