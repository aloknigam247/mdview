//! Serves a rendered math fixture + the bundled KaTeX assets on
//! 127.0.0.1:7683 for Playwright screenshot tests.

use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

use comrak::nodes::NodeValue;
use comrak::{parse_document, Arena, ComrakOptions};
use mdview_ext_math::{AstNode, Math, MdViewExtension, RenderCtx};
use tiny_http::{Header, Response, Server};

const ADDR: &str = "127.0.0.1:7683";

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = {
        let repo = manifest.join("../../fixtures/math.md");
        if repo.exists() {
            repo
        } else {
            manifest.join("fixtures/math.md")
        }
    };
    let src = fs::read_to_string(&fixture).expect("read fixture");

    let html_page = render_page(&src);
    let assets_dir = manifest.join("assets").join("vendor");

    let server = Server::http(ADDR).expect("bind tiny_http");
    eprintln!("mdview-ext-math demo_serve listening on http://{}", ADDR);

    for request in server.incoming_requests() {
        let url = request.url().to_string();
        let (body, mime): (Vec<u8>, &str) = if url == "/" || url == "/index.html" {
            (html_page.as_bytes().to_vec(), "text/html; charset=utf-8")
        } else if let Some(p) = url.strip_prefix("/vendor/") {
            let path = assets_dir.join(p);
            match fs::read(&path) {
                Ok(bytes) => (bytes, guess_mime(p)),
                Err(_) => (b"not found".to_vec(), "text/plain"),
            }
        } else {
            (b"not found".to_vec(), "text/plain")
        };
        let header = Header::from_bytes(&b"Content-Type"[..], mime.as_bytes()).unwrap();
        let len = body.len();
        let response = Response::new(200.into(), vec![header], Cursor::new(body), Some(len), None);
        let _ = request.respond(response);
    }
}

fn guess_mime(p: &str) -> &'static str {
    if p.ends_with(".js") {
        "application/javascript"
    } else if p.ends_with(".css") {
        "text/css"
    } else if p.ends_with(".woff2") {
        "font/woff2"
    } else if p.ends_with(".woff") {
        "font/woff"
    } else {
        "application/octet-stream"
    }
}

fn render_page(src: &str) -> String {
    let mut opts = ComrakOptions::default();
    Math.register_parser(&mut opts);
    let arena = Arena::new();
    let root = parse_document(&arena, src, &opts);

    let mut body = String::new();
    let ctx = RenderCtx::default();
    walk(root, &Math, &ctx, &mut body);

    format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\"><title>mdview math</title>\n\
         <link rel=\"stylesheet\" href=\"/vendor/katex.min.css\">\n\
         <script defer src=\"/vendor/katex.min.js\"></script>\n\
         <script defer src=\"/vendor/mdv-math-init.js\"></script>\n\
         <style>body{{font-family:ui-sans-serif,system-ui;line-height:1.7;max-width:720px;margin:2em auto;padding:0 1em;color:#222;}}</style>\n\
         </head><body>\n<h1>Math fixture</h1>\n{body}\n</body></html>\n"
    )
}

fn walk<'a>(node: &'a AstNode<'a>, ext: &Math, ctx: &RenderCtx, out: &mut String) {
    if matches!(node.data.borrow().value, NodeValue::Math(_)) {
        if let Some(h) = ext.render_html(node, ctx) {
            out.push_str(&h.0);
            out.push('\n');
            return;
        }
    }
    for child in node.children() {
        walk(child, ext, ctx, out);
    }
}
