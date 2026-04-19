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

    let mut body = String::new();
    for node in root.descendants() {
        if matches!(node.data.borrow().value, NodeValue::CodeBlock(_)) {
            if let Some(h) = Drawio.render_html(node, &ctx) {
                body.push_str(&h.0);
                body.push('\n');
            }
        }
    }

    let viewer_js = fs::read_to_string(manifest.join("vendor/drawio-viewer.js"))
        .expect("read drawio-viewer.js");
    let init_js = fs::read_to_string(manifest.join("vendor/mdv-drawio-init.js"))
        .expect("read mdv-drawio-init.js");

    let html = format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\"><title>drawio demo</title>\n<style>body{{font-family:ui-sans-serif,system-ui;padding:32px;background:#f8fafc;}}.drawio-viewer{{border-radius:12px;box-shadow:0 4px 12px rgba(15,23,42,0.1);padding:16px;background:white;margin:16px 0;}}</style>\n</head><body><h1>drawio fixture</h1>{body}<script>{viewer}</script><script>{init}</script></body></html>",
        body = body,
        viewer = viewer_js,
        init = init_js,
    );

    let addr = "127.0.0.1:7685";
    let server = tiny_http::Server::http(addr).expect("bind");
    eprintln!("serving on http://{}", addr);

    for request in server.incoming_requests() {
        let url = request.url().to_string();
        let response = if url == "/" || url == "/index.html" {
            tiny_http::Response::from_string(html.clone())
                .with_header(
                    "Content-Type: text/html; charset=utf-8"
                        .parse::<tiny_http::Header>()
                        .unwrap(),
                )
                .boxed()
        } else {
            tiny_http::Response::from_string("not found")
                .with_status_code(404)
                .boxed()
        };
        let _ = request.respond(response);
    }
}
