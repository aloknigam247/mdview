//! Terminal / nvim pipelines.
//!
//! These orchestrate the read → parse → render → output path. Real parsing /
//! rendering / paging live in sibling crates; the `stubs` feature supplies
//! minimal replacements so this binary can build and test in isolation.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use crate::cli::Cli;

#[cfg(feature = "stubs")]
use crate::_stubs as backend;

pub fn run_terminal(cli: &Cli) -> Result<()> {
    let file = cli
        .file
        .as_ref()
        .context("terminal mode requires a FILE argument")?;
    render_once(file, cli)?;

    if cli.watch {
        run_watch_loop(file, cli)?;
    }
    Ok(())
}

fn render_once(file: &Path, cli: &Cli) -> Result<()> {
    let src =
        std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let doc = backend::parse(&src);
    let rendered = backend::render_terminal(&doc, cli.theme.as_deref());

    if cli.no_pager {
        use std::io::Write;
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        lock.write_all(rendered.ansi.as_bytes())?;
    } else {
        backend::write_to_pager(&rendered.ansi)?;
    }
    Ok(())
}

fn run_watch_loop(file: &Path, cli: &Cli) -> Result<()> {
    use notify::{recommended_watcher, Event, RecursiveMode, Watcher};

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;
    watcher.watch(file, RecursiveMode::NonRecursive)?;

    loop {
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(Ok(_)) => {
                render_once(file, cli)?;
            }
            Ok(Err(e)) => eprintln!("watch error: {e}"),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    Ok(())
}

pub async fn run_nvim(cli: &Cli) -> Result<()> {
    let sock = cli
        .nvim_socket
        .as_ref()
        .context("nvim mode requires --nvim-socket")?;
    backend::nvim_listen_stub(sock).await
}

pub async fn run_tauri_child(cli: &Cli) -> Result<()> {
    let port = backend::pick_auto_port();
    let file: Option<PathBuf> = cli.file.clone();
    let server = tokio::spawn(async move { backend::serve_stub(port, file.as_deref()).await });

    #[cfg(feature = "tauri-shell")]
    {
        run_tauri_event_loop(port)?;
    }

    #[cfg(not(feature = "tauri-shell"))]
    {
        // Headless fallback for isolated builds / CI without system webview.
        // Block briefly so the spawned server task has a chance to initialise.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let _ = port;
    }

    server.abort();
    Ok(())
}

#[cfg(feature = "tauri-shell")]
fn run_tauri_event_loop(port: u16) -> Result<()> {
    let url = format!("http://127.0.0.1:{port}");
    tauri::Builder::default()
        .setup(move |app| {
            use tauri::WebviewWindowBuilder;
            let _ = WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::External(url.parse().unwrap()),
            )
            .title("mdview")
            .inner_size(1200.0, 800.0)
            .min_inner_size(600.0, 400.0)
            .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|e| anyhow::anyhow!("tauri: {e}"))?;
    Ok(())
}
