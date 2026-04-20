//! Serves the mermaid fixture HTML + vendored mermaid.min.js on 127.0.0.1:7684.
//!
//! Designed for Playwright: navigate to `/`, wait for `.mermaid svg`,
//! screenshot the page.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;

use comrak::nodes::NodeValue;
use comrak::{parse_document, Arena, ComrakOptions};
use mdview_ext_mermaid::{MdViewExtension, Mermaid, RenderCtx, Theme};

const ADDR: &str = "127.0.0.1:7684";

struct Site {
    html: String,
    vendor: HashMap<String, Vec<u8>>,
}

fn main() -> std::io::Result<()> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = fs::read_to_string(crate_dir.join("fixtures/mermaid.md"))?;

    let mut vendor = HashMap::new();
    let vendor_dir = crate_dir.join("assets/vendor");
    for entry in fs::read_dir(&vendor_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        vendor.insert(name, fs::read(entry.path())?);
    }

    let site = Arc::new(Site {
        html: build_page(&src),
        vendor,
    });

    let listener = TcpListener::bind(ADDR)?;
    eprintln!("listening on http://{ADDR}");

    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let site = Arc::clone(&site);
        std::thread::spawn(move || {
            let _ = handle(stream, &site);
        });
    }
    Ok(())
}

fn build_page(src: &str) -> String {
    let ext = Mermaid;
    let theme = Theme::default();
    let ctx = RenderCtx::new(&theme);
    let arena = Arena::new();
    let root = parse_document(&arena, src, &ComrakOptions::default());

    let mut out = String::new();
    out.push_str("<!doctype html><html><head><meta charset=\"utf-8\">");
    out.push_str("<title>mermaid demo</title>");
    out.push_str("<style>body{font-family:ui-sans-serif,system-ui;padding:2rem;background:#fff;}.mermaid{margin:1rem 0;}</style>");
    out.push_str("</head><body>\n");
    for node in root.descendants() {
        if matches!(node.data.borrow().value, NodeValue::CodeBlock(_)) {
            if let Some(html) = ext.render_html(node, &ctx) {
                out.push_str(&html.0);
                out.push('\n');
            }
        }
    }
    out.push_str("<script src=\"/vendor/mermaid.min.js\"></script>\n");
    out.push_str("<script src=\"/vendor/mdv-mermaid-init.js\"></script>\n");
    out.push_str("</body></html>\n");
    out
}

fn handle(mut stream: TcpStream, site: &Site) -> std::io::Result<()> {
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let path = req
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .split('?')
        .next()
        .unwrap_or("/");

    if path == "/" || path == "/index.html" {
        return respond(
            &mut stream,
            200,
            "text/html; charset=utf-8",
            site.html.as_bytes(),
        );
    }
    if let Some(name) = path.strip_prefix("/vendor/") {
        if name.contains("..") {
            return respond(&mut stream, 400, "text/plain", b"bad path");
        }
        if let Some(bytes) = site.vendor.get(name) {
            return respond(&mut stream, 200, "application/javascript", bytes);
        }
    }
    respond(&mut stream, 404, "text/plain", b"not found")
}

fn respond(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let status_line = match status {
        200 => "200 OK",
        400 => "400 Bad Request",
        404 => "404 Not Found",
        _ => "500 Internal Server Error",
    };
    let header = format!(
        "HTTP/1.1 {status_line}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}
