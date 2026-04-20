//! Render `fixtures/mermaid.md` via the mermaid extension's HTML path only.

use std::fs;
use std::path::PathBuf;

use mdview_ext_mermaid::{scan, AstNode, MdViewExtension, Mermaid, RenderCtx};

fn main() -> std::io::Result<()> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = crate_dir.join("fixtures/mermaid.md");
    let src = fs::read_to_string(&fixture)?;

    let ext = Mermaid;
    let ctx = RenderCtx::default();

    let mut out = String::new();
    out.push_str("<!doctype html><html><head><meta charset=\"utf-8\"><title>mermaid demo</title></head><body>\n");
    for block in scan::mermaid_blocks(&src) {
        let node = AstNode::FencedCode {
            info: "mermaid".into(),
            literal: block,
        };
        if let Some(html) = ext.render_html(&node, &ctx) {
            out.push_str(&html.0);
            out.push('\n');
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
