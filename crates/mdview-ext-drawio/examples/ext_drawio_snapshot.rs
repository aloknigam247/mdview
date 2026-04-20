use std::fs;
use std::path::PathBuf;

use comrak::nodes::NodeValue;
use comrak::{parse_document, Arena, ComrakOptions};
use mdview_ext_drawio::{Drawio, MdViewExtension, RenderCtx};

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest.join("../../fixtures/drawio.md");
    let src = fs::read_to_string(&fixture).expect("read fixture");

    let arena = Arena::new();
    let root = parse_document(&arena, &src, &ComrakOptions::default());
    let ctx = RenderCtx::default();

    let mut out = String::new();
    for node in root.descendants() {
        if matches!(node.data.borrow().value, NodeValue::CodeBlock(_)) {
            if let Some(h) = Drawio.render_html(node, &ctx) {
                out.push_str(&h.0);
                out.push('\n');
            }
        }
    }

    let snapshot_dir = manifest.join("__snapshots__");
    fs::create_dir_all(&snapshot_dir).expect("mkdir snapshots");
    let out_path = snapshot_dir.join("drawio.actual.html");
    fs::write(&out_path, &out).expect("write snapshot");
    println!("wrote {}", out_path.display());
    print!("{}", out);
}
