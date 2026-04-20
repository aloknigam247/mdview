use comrak::nodes::NodeValue;
use comrak::{parse_document, Arena, ComrakOptions};
use mdview_ext_highlight::{Highlight, MdViewExtension, RenderCtx, Theme};
use std::fs;
use std::path::PathBuf;

fn snapshot_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("__snapshots__")
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/code.md")
}

fn render_html(src: &str) -> String {
    let arena = Arena::new();
    let opts = ComrakOptions::default();
    let root = parse_document(&arena, src, &opts);
    let theme = Theme {
        name: "light".to_string(),
        ..Theme::default()
    };
    let ctx = RenderCtx::new(&theme);
    let mut out = String::from("<!doctype html>\n<html><head><meta charset=\"utf-8\"><title>code demo</title><style>body{font-family:ui-sans-serif,system-ui;padding:24px;background:#fafafa;}pre.mdv-code{border-radius:12px;padding:16px;background:#f4f4f5;box-shadow:0 1px 2px rgba(0,0,0,0.08);overflow:auto;}code{font-family:ui-monospace,Menlo,Consolas,monospace;}</style></head><body>\n");
    for child in root.children() {
        if matches!(child.data.borrow().value, NodeValue::CodeBlock(_)) {
            if let Some(h) = Highlight.render_html(child, &ctx) {
                out.push_str(&h.0);
                out.push('\n');
            }
        }
    }
    out.push_str("</body></html>\n");
    out
}

fn render_ansi(src: &str) -> String {
    let arena = Arena::new();
    let opts = ComrakOptions::default();
    let root = parse_document(&arena, src, &opts);
    let theme = Theme {
        name: "dark".to_string(),
        ..Theme::default()
    };
    let ctx = RenderCtx::new(&theme);
    let mut out = String::new();
    for child in root.children() {
        if matches!(child.data.borrow().value, NodeValue::CodeBlock(_)) {
            if let Some(chunks) = Highlight.render_terminal(child, &ctx) {
                for c in chunks {
                    out.push_str(&c.text);
                }
                out.push('\n');
            }
        }
    }
    out
}

fn main() {
    let fixture = fixture_path();
    let src =
        fs::read_to_string(&fixture).unwrap_or_else(|e| panic!("read {}: {e}", fixture.display()));
    let dir = snapshot_dir();
    fs::create_dir_all(&dir).expect("mkdir snapshots");
    let html = render_html(&src);
    fs::write(dir.join("code.html.actual"), &html).expect("write html");
    let ansi = render_ansi(&src);
    fs::write(dir.join("code.ansi.actual"), &ansi).expect("write ansi");
    println!(
        "wrote {} and {}",
        dir.join("code.html.actual").display(),
        dir.join("code.ansi.actual").display()
    );
}
