use std::fs;

use tiny_http::{Header, Response, Server};

fn main() {
    let addr = std::env::var("MDV_PLOTLY_ADDR").unwrap_or_else(|_| "127.0.0.1:7686".into());
    let fixture = mdview_ext_plotly::locate_fixture("plotly.md");
    let md = fs::read_to_string(&fixture).expect("read fixture");
    let html = mdview_ext_plotly::demo_document(&md);

    let server = Server::http(&addr).expect("bind demo server");
    eprintln!("demo server listening on http://{addr}");
    for request in server.incoming_requests() {
        let header = Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
            .expect("content type header");
        let response = Response::from_string(html.clone()).with_header(header);
        let _ = request.respond(response);
    }
}
