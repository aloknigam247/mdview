//! Terminal / nvim pipelines.
//!
//! These orchestrate the read → parse → render → output path. Real parsing /
//! rendering / paging live in sibling crates; the `stubs` feature supplies
//! minimal replacements so this binary can build and test in isolation.

use anyhow::{Context, Result};
use std::path::Path;
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
    let ansi = crate::render_terminal::render_ansi(&src)?;

    if cli.no_pager {
        use std::io::Write;
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        lock.write_all(ansi.as_bytes())?;
    } else {
        backend::write_to_pager(&ansi)?;
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
    let file = cli
        .file
        .as_ref()
        .context("tauri mode requires a FILE argument")?;
    let src = std::fs::read_to_string(file)
        .with_context(|| format!("reading {}", file.display()))?;
    let title = file
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("mdview")
        .to_string();
    let html = crate::render::render_page(&src, &title)?;
    let srv = crate::server::serve_html(html).await?;
    let port = srv.port;
    eprintln!("mdview: serving on http://127.0.0.1:{port}");

    let url = format!("http://127.0.0.1:{port}");

    #[cfg(feature = "gui")]
    {
        // wry runs the event loop on the current thread and blocks until the
        // window is closed. Keep `srv` alive for the whole session.
        run_gui_event_loop(&url)?;
    }

    #[cfg(not(feature = "gui"))]
    {
        open_in_browser(&url);
        eprintln!("mdview: opened {url} in default browser — press Ctrl+C to stop.");
        tokio::signal::ctrl_c().await.ok();
    }

    drop(srv);
    Ok(())
}

#[cfg(not(feature = "gui"))]
fn open_in_browser(url: &str) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

#[cfg(feature = "gui")]
fn run_gui_event_loop(url: &str) -> Result<()> {
    use tao::dpi::LogicalSize;
    use tao::event::{Event, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoop};
    use tao::window::WindowBuilder;
    use wry::WebViewBuilder;

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("mdview")
        .with_inner_size(LogicalSize::new(1200.0, 800.0))
        .with_min_inner_size(LogicalSize::new(600.0, 400.0))
        .build(&event_loop)
        .map_err(|e| anyhow::anyhow!("window build: {e}"))?;

    let _webview = WebViewBuilder::new()
        .with_url(url)
        .build(&window)
        .map_err(|e| anyhow::anyhow!("webview build: {e}"))?;

    event_loop.run(move |event, _target, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });
}
