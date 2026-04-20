use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;

use mdview_render_html::{htmlesc, render_markdown, Registry, RenderCtx};

fn main() {
    let addr = "127.0.0.1:7681";
    let listener = TcpListener::bind(addr).unwrap_or_else(|e| {
        eprintln!("bind {addr}: {e}");
        std::process::exit(1);
    });
    eprintln!("mdview-render-html demo_serve listening on http://{addr}");

    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let local = crate_dir.join("fixtures");
    let fixtures_dir = if local.exists() {
        local
    } else {
        crate_dir.join("../../fixtures")
    };
    let fixtures = load_fixtures(&fixtures_dir);

    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut request_line = String::new();
        if reader.read_line(&mut request_line).is_err() {
            continue;
        }
        let mut path = "/".to_string();
        if let Some(p) = request_line.split_whitespace().nth(1) {
            path = p.to_string();
        }
        let mut content_length: usize = 0;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).is_err() {
                break;
            }
            if line == "\r\n" || line.is_empty() {
                break;
            }
            if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = v.trim().parse().unwrap_or(0);
            }
        }
        if content_length > 0 {
            let mut body = vec![0u8; content_length];
            let _ = reader.read_exact(&mut body);
        }

        let (status, content_type, body) = route(&path, &fixtures);
        let resp = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n",
            status = status,
            content_type = content_type,
            len = body.len(),
        );
        let _ = stream.write_all(resp.as_bytes());
        let _ = stream.write_all(&body);
        let _ = stream.flush();
    }
}

fn load_fixtures(dir: &std::path::Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Ok(rd) = fs::read_dir(dir) else {
        return out;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            if let Ok(src) = fs::read_to_string(&path) {
                out.insert(name, src);
            }
        }
    }
    out
}

fn route(path: &str, fixtures: &BTreeMap<String, String>) -> (&'static str, &'static str, Vec<u8>) {
    let ctx = RenderCtx::default();
    let registry = Registry::new();
    if path == "/" || path == "/index" || path == "/index.html" {
        let body = render_index(fixtures);
        return ("200 OK", "text/html; charset=utf-8", body.into_bytes());
    }
    let name = path.trim_start_matches('/').trim_end_matches(".html");
    if let Some(src) = fixtures.get(name) {
        let html = render_markdown(src, &ctx, &registry);
        return ("200 OK", "text/html; charset=utf-8", html.into_bytes());
    }
    (
        "404 Not Found",
        "text/plain; charset=utf-8",
        b"not found".to_vec(),
    )
}

fn render_index(fixtures: &BTreeMap<String, String>) -> String {
    let mut body = String::new();
    body.push_str("<!doctype html><html><head><meta charset=utf-8><title>mdview fixtures</title>");
    body.push_str("<style>body{font-family:ui-sans-serif,system-ui;max-width:42rem;margin:3rem auto;line-height:1.7;}a{color:#2563eb;}li{margin:.25rem 0;}</style>");
    body.push_str("</head><body><h1>mdview-render-html fixtures</h1><ul>");
    for name in fixtures.keys() {
        body.push_str(&format!(
            "<li><a href=\"/{n}\">{n}.md</a></li>",
            n = htmlesc::escape_html(name)
        ));
    }
    body.push_str("</ul></body></html>");
    body
}
