//! Render `fixtures/mermaid.md` via the mermaid extension's HTML path only.

use std::fs;
use std::path::PathBuf;

use comrak::nodes::NodeValue;
use comrak::{parse_document, Arena, ComrakOptions};
use mdview_ext_mermaid::{MdViewExtension, Mermaid, RenderCtx, Theme};

fn main() -> std::io::Result<()> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = crate_dir.join("fixtures/mermaid.md");
    let src = fs::read_to_string(&fixture)?;

    let ext = Mermaid;
    let theme = Theme::default();
    let ctx = RenderCtx::new(&theme);

    let arena = Arena::new();
    let root = parse_document(&arena, &src, &ComrakOptions::default());

    let mut out = String::new();
    out.push_str("<!doctype html><html><head><meta charset=\"utf-8\"><title>mermaid demo</title></head><body>\n");
    for node in root.descendants() {
        if matches!(node.data.borrow().value, NodeValue::CodeBlock(_)) {
            if let Some(html) = ext.render_html(node, &ctx) {
                out.push_str(&html.0);
                out.push('\n');
            }
        }
    }
    out.push_str("<script src=\"vendor/mermaid.min.js\"></script>\n");
    out.push_str("<script src=\"vendor/mdv-mermaid-init.js\"></script>\n");
    out.push_str("</body></html>\n");

    let snapshot_dir = crate_dir.join("__snapshots__");
    fs::create_dir_all(&snapshot_dir)?;
    let out_path = snapshot_dir.join("mermaid.actual.html");
    fs::write(&out_path, out.as_bytes())?;
    println!("wrote {}", out_path.display());
    Ok(())
}
