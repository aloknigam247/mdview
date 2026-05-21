use mdview_core::MdViewExtension;
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

Body.
"#;

#[test]
fn parses_complex_sample() {
    let ext = FrontmatterExt::new();
    let mut src = SAMPLE.to_string();
    ext.pre_parse(&mut src);
    assert!(src.starts_with("# Hello"), "src = {:?}", src);
    let v = ext.value().expect("frontmatter present");
    assert_eq!(v["title"], "Building mdview");
    assert_eq!(v["author"]["name"], "Alok Nigam");
    assert_eq!(v["tags"][0], "rust");
    assert_eq!(v["seo"]["keywords"][2], "tauri");
    assert_eq!(v["related"][0]["url"], "/posts/tauri-vs-electron");
    assert_eq!(v["metadata"]["reading_time"], 8);
}

#[test]
fn missing_frontmatter_is_passthrough() {
    let ext = FrontmatterExt::new();
    let mut src = "# Hello\n\nbody".to_string();
    ext.pre_parse(&mut src);
    assert_eq!(src, "# Hello\n\nbody");
    assert!(ext.value().is_none());
}

#[test]
fn malformed_yaml_does_not_strip() {
    let ext = FrontmatterExt::new();
    let mut src = "---\nbad: : : yaml\n---\n# Body".to_string();
    let original = src.clone();
    ext.pre_parse(&mut src);
    assert_eq!(src, original);
    assert!(ext.value().is_none());
}
