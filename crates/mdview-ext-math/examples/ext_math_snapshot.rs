//! Golden-file snapshot demo for `mdview-ext-math`.
//!
//! Renders a math fixture to HTML and terminal form and writes them under
//! `__snapshots__/` next to this example. Compare against committed
//! `.expected` files when integrating.

use std::fs;
use std::path::PathBuf;

use comrak::nodes::NodeValue;
use comrak::{parse_document, Arena, ComrakOptions};
use mdview_ext_math::{AstNode, Math, MdViewExtension, RenderCtx, Theme};

fn main() {
    let fixture = find_fixture();
    let src = fs::read_to_string(&fixture).expect("read fixture");

    let mut opts = ComrakOptions::default();
    Math.register_parser(&mut opts);

    let arena = Arena::new();
    let root = parse_document(&arena, &src, &opts);

    let mut html = String::from("<!doctype html>\n<html><head><meta charset=\"utf-8\"><link rel=\"stylesheet\" href=\"vendor/katex.min.css\"><script defer src=\"vendor/katex.min.js\"></script><script defer src=\"vendor/mdv-math-init.js\"></script></head><body>\n");
    let mut term = String::new();
    let theme = Theme::default();
    let ctx = RenderCtx::new(&theme);
    walk(root, &Math, &ctx, &mut html, &mut term);
    html.push_str("\n</body></html>\n");

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let snap = manifest.join("__snapshots__");
    fs::create_dir_all(&snap).expect("mkdir snapshots");
    fs::write(snap.join("math.html.actual"), &html).expect("write html");
    fs::write(snap.join("math.term.actual"), &term).expect("write term");
    println!("snapshots written to {}", snap.display());
}

fn find_fixture() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest.join("../../fixtures/math.md");
    if repo_root.exists() {
        return repo_root;
    }
    manifest.join("fixtures/math.md")
}

fn walk<'a>(
    node: &'a AstNode<'a>,
    ext: &Math,
    ctx: &RenderCtx<'_>,
    html_out: &mut String,
    term_out: &mut String,
) {
    let rendered = matches!(node.data.borrow().value, NodeValue::Math(_));
    if rendered {
        if let Some(h) = ext.render_html(node, ctx) {
            html_out.push_str(&h.0);
            html_out.push('\n');
        }
        if let Some(t) = ext.render_terminal(node, ctx) {
            for ch in t {
                term_out.push_str(&ch.text);
            }
            term_out.push('\n');
        }
    }
    for child in node.children() {
        walk(child, ext, ctx, html_out, term_out);
    }
}
