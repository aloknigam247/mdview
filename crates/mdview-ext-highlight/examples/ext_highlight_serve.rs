use comrak::nodes::NodeValue;
use comrak::{parse_document, Arena, ComrakOptions};
use mdview_ext_highlight::{Highlight, MdViewExtension, RenderCtx, Theme};
use std::fs;
use std::path::PathBuf;
use tiny_http::{Header, Response, Server};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/code.md")
}

fn render_page(src: &str) -> String {
    let arena = Arena::new();
    let opts = ComrakOptions::default();
    let root = parse_document(&arena, src, &opts);
    let theme = Theme {
        name: "light".to_string(),
        ..Theme::default()
    };
    let ctx = RenderCtx::new(&theme);
    let mut body = String::new();
    for child in root.children() {
        if matches!(child.data.borrow().value, NodeValue::CodeBlock(_)) {
            if let Some(h) = Highlight.render_html(child, &ctx) {
                body.push_str(&h.0);
                body.push('\n');
            }
        }
    }
    format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\"><title>mdview highlight</title><style>\
body{{font-family:ui-sans-serif,system-ui;padding:48px;background:#fafafa;color:#24292f;line-height:1.7;}}\
h1{{font-size:28px;margin-bottom:24px;}}\
pre.mdv-code{{border-radius:12px;padding:20px;background:#f4f4f5;box-shadow:0 2px 8px rgba(0,0,0,0.08);overflow:auto;margin:16px 0;}}\
code{{font-family:ui-monospace,Menlo,Consolas,monospace;font-size:14px;}}\
</style></head><body><h1>mdview highlight demo</h1>{body}</body></html>\n"
    )
}

fn main() {
    let src = fs::read_to_string(fixture_path()).expect("read fixture");
    let page = render_page(&src);
    let server = Server::http("127.0.0.1:7682").expect("bind 127.0.0.1:7682");
    eprintln!("listening on http://127.0.0.1:7682");
    for req in server.incoming_requests() {
        let content_type =
            Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap();
        let resp = Response::from_string(page.clone()).with_header(content_type);
        let _ = req.respond(resp);
    }
}
