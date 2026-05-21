use mdview_core::{MdViewExtension, RenderCtx, TerminalCaps, Theme};
use mdview_ext_frontmatter::FrontmatterExt;

const SAMPLE: &str = r#"---
title: "Building mdview"
subtitle: "A markdown viewer in Rust"
date: 2026-05-21
author:
  name: Alok Nigam
  email: alok@example.com
  url: https://example.com
tags: [rust, markdown, tooling]
draft: false
seo:
  description: A lightweight markdown renderer
  keywords: [rust, markdown, tauri]
  og_image: /images/cover.png
related:
  - title: "Tauri vs Electron"
    url: /posts/tauri-vs-electron
  - title: "Why Rust"
    url: /posts/why-rust
metadata:
  reading_time: 8
  word_count: 1842
  language: en
---
# Hello
"#;

fn setup() -> FrontmatterExt {
    let ext = FrontmatterExt::new();
    let mut s = SAMPLE.to_string();
    ext.pre_parse(&mut s);
    ext
}

fn body_of(html: &str) -> &str {
    let i = html
        .find("</style>")
        .map(|i| i + "</style>".len())
        .unwrap_or(0);
    &html[i..]
}

#[test]
fn html_card_contains_recognized_fields() {
    let ext = setup();
    let theme = Theme::default();
    let ctx = RenderCtx::new(&theme);
    let html = ext.pre_render_html(&ctx).expect("card").0;
    let body = body_of(&html);

    assert!(body.contains("mdview-frontmatter"));
    assert!(body.contains("Building mdview"));
    assert!(body.contains("A markdown viewer in Rust"));
    assert!(body.contains("2026-05-21"));
    assert!(body.contains("Alok Nigam"));
    assert!(body.contains("<li>rust</li>"));
    assert!(body.contains("<li>markdown</li>"));
    assert!(body.contains("<li>tooling</li>"));
    assert!(body.contains("alok@example.com"));
    assert!(body.contains(">seo<"));
    assert!(body.contains("A lightweight markdown renderer"));
    assert!(body.contains("href=\"/posts/tauri-vs-electron\""));
    assert!(body.contains("Tauri vs Electron"));
    // draft=false so no draft badge in the body
    assert!(!body.contains(">draft<"));
}

#[test]
fn html_card_emits_draft_badge_when_true() {
    let ext = FrontmatterExt::new();
    let mut s = "---\ntitle: Hi\ndraft: true\n---\nbody".to_string();
    ext.pre_parse(&mut s);
    let theme = Theme::default();
    let ctx = RenderCtx::new(&theme);
    let html = ext.pre_render_html(&ctx).expect("card").0;
    let body = body_of(&html);
    assert!(body.contains(">draft<"));
}

#[test]
fn html_card_suppresses_title_row_when_missing() {
    let ext = FrontmatterExt::new();
    let mut s = "---\nsubtitle: only sub\n---\nbody".to_string();
    ext.pre_parse(&mut s);
    let theme = Theme::default();
    let ctx = RenderCtx::new(&theme);
    let html = ext.pre_render_html(&ctx).expect("card").0;
    let body = body_of(&html);
    assert!(!body.contains("mdview-frontmatter__title"));
    assert!(body.contains("only sub"));
}

#[test]
fn terminal_card_uses_box_drawing() {
    let ext = setup();
    let theme = Theme::default();
    let mut ctx = RenderCtx::new(&theme);
    ctx.terminal_caps = Some(TerminalCaps {
        width: 60,
        ..Default::default()
    });
    let chunks = ext.pre_render_terminal(&ctx).expect("chunks");
    let text: String = chunks.iter().map(|c| c.text.clone()).collect();
    assert!(text.contains('\u{256d}'));
    assert!(text.contains('\u{256e}'));
    assert!(text.contains('\u{2570}'));
    assert!(text.contains('\u{256f}'));
    assert!(text.contains("Building mdview"));
    assert!(text.contains("A markdown viewer in Rust"));
    assert!(text.contains("Alok Nigam"));
    assert!(text.contains("\u{25cf} rust"));
    assert!(text.contains("\u{25be} seo"));
}
