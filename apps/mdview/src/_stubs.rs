//! Local stubs for sibling `mdview-*` crates.
//!
//! TODO: replace with the real `mdview_core`, `mdview_theme`, `mdview_server`,
//! `mdview_render_terminal`, `mdview_pager`, and `mdview_nvim` crates after
//! integration. These stubs exist only so `apps/mdview` can build and test in
//! isolation within a single-unit worktree.

#![allow(dead_code)]

use std::path::Path;

#[derive(Debug, Clone)]
pub struct ParsedDoc {
    pub source: String,
}

pub fn parse(src: &str) -> ParsedDoc {
    ParsedDoc {
        source: src.to_string(),
    }
}

#[derive(Debug, Clone, Default)]
pub struct RenderedTerminal {
    pub ansi: String,
}

pub fn render_terminal(doc: &ParsedDoc, _theme: Option<&str>) -> RenderedTerminal {
    RenderedTerminal {
        ansi: doc.source.clone(),
    }
}

#[derive(Debug, Clone, Default)]
pub struct RenderedHtml {
    pub html: String,
}

pub fn render_html(doc: &ParsedDoc, _theme: Option<&str>) -> RenderedHtml {
    RenderedHtml {
        html: format!("<article>{}</article>", html_escape(&doc.source)),
    }
}

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

pub fn write_to_pager(ansi: &str) -> std::io::Result<()> {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    lock.write_all(ansi.as_bytes())?;
    Ok(())
}

pub fn pick_auto_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    listener.local_addr().expect("local_addr").port()
}

pub async fn serve_stub(_port: u16, _file: Option<&Path>) -> anyhow::Result<()> {
    Ok(())
}

pub async fn nvim_listen_stub(_socket: &Path) -> anyhow::Result<()> {
    Ok(())
}
