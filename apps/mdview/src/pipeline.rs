//! Terminal / nvim pipelines.
//!
//! These orchestrate the read → parse → render → output path.

use anyhow::{Context, Result};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use crate::cli::Cli;

fn format_window_title(file: Option<&std::path::Path>) -> String {
    match file {
        Some(p) => {
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .filter(|s| !s.is_empty())
                .unwrap_or("untitled");
            format!("{name} - mdview")
        }
        None => "mdview".into(),
    }
}

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
    let source_dir = file.parent();
    let config = mdview_config::Config::load();
    let ansi =
        crate::render_terminal::render_ansi_with_source(&src, source_dir, config.code.tab_width)?;

    if cli.no_pager {
        use std::io::Write;
        let stdout = std::io::stdout();
        let mut lock = stdout.lock();
        lock.write_all(ansi.as_bytes())?;
    } else {
        let (tx, rx) = mpsc::channel();
        tx.send(vec![mdview_render_terminal::TermChunk::plain(ansi)])
            .context("sending terminal render to pager")?;
        drop(tx);
        let pager_theme = mdview_theme::find("dark").expect("dark theme is built in");
        mdview_pager::run(rx, pager_theme)?;
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
    let _events = mdview_nvim::listen(sock).await?;
    Ok(())
}

pub async fn run_tauri_child(cli: &Cli) -> Result<()> {
    let file = cli
        .file
        .as_ref()
        .context("tauri mode requires a FILE argument")?;
    let src =
        std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let title = file
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("mdview")
        .to_string();
    let loaded = mdview_config::Config::load_full();
    let config = loaded.config;
    let source_dir = file.parent();
    let html = crate::render::render_page_full(
        &src,
        &title,
        &config.theme,
        &config.keymap,
        source_dir,
        &loaded.errors,
        &config.toc,
        &config.codemap,
        &config.code,
    )?;
    crate::profile::log(&format!("html_rendered ({}B)", html.len()));
    let srv = crate::server::serve_html(html).await?;
    let port = srv.port;
    eprintln!("mdview: serving on http://127.0.0.1:{port}");

    let url = format!("http://127.0.0.1:{port}");

    #[cfg(feature = "gui")]
    {
        // wry runs the event loop on the current thread and blocks until the
        // window is closed. Keep `srv` alive for the whole session.
        let win_title = format_window_title(cli.file.as_deref());
        let reload_ctx = ReloadCtx {
            file: file.clone(),
            title: title.clone(),
            config: config.clone(),
            config_errors: loaded.errors.clone(),
            html_store: srv.html.clone(),
        };
        run_gui_event_loop(&url, &config, &win_title, reload_ctx)?;
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

/// Serve rendered markdown over HTTP and block until Ctrl+C.
///
/// No window and no daemonize: the process must stay in the foreground so a
/// supervising harness (Playwright's `webServer`) can detect liveness and kill
/// it on teardown.
pub async fn run_serve(cli: &Cli) -> Result<()> {
    let file = cli
        .file
        .as_ref()
        .context("serve-only mode requires a FILE argument")?;
    let index =
        std::fs::canonicalize(file).with_context(|| format!("resolving {}", file.display()))?;
    let root = std::env::current_dir()?.canonicalize()?;

    let srv = crate::server::serve_dir(root.clone(), index, cli.port, move |path| {
        let src =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let title = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("mdview")
            .to_string();
        let loaded = mdview_config::Config::load_full();
        let config = loaded.config;
        crate::render::render_page_full(
            &src,
            &title,
            &config.theme,
            &config.keymap,
            path.parent(),
            &loaded.errors,
            &config.toc,
            &config.codemap,
            &config.code,
        )
    })
    .await?;

    eprintln!("mdview: serving on http://127.0.0.1:{}", srv.port);
    eprintln!("mdview: serve root {}", root.display());
    tokio::signal::ctrl_c().await.ok();
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
#[derive(Debug, Clone)]
pub enum MdvUserEvent {
    OpenLink(String),
    Quit,
    Reload,
    ThemeChanged(String),
}

#[cfg(feature = "gui")]
struct ReloadCtx {
    file: std::path::PathBuf,
    title: String,
    config: mdview_config::Config,
    config_errors: Vec<mdview_config::ConfigError>,
    html_store: crate::server::HtmlStore,
}

#[cfg(feature = "gui")]
fn load_icon() -> Option<tao::window::Icon> {
    const ICON_PNG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon-256.png"));
    let img = image::load_from_memory(ICON_PNG).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    tao::window::Icon::from_rgba(rgba.into_raw(), w, h).ok()
}

#[cfg(feature = "gui")]
fn run_gui_event_loop(
    url: &str,
    config: &mdview_config::Config,
    title: &str,
    reload_ctx: ReloadCtx,
) -> Result<()> {
    use tao::dpi::LogicalSize;
    use tao::event::{Event, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoopBuilder};
    use tao::window::WindowBuilder;
    use wry::WebViewBuilder;

    let event_loop = EventLoopBuilder::<MdvUserEvent>::with_user_event().build();
    let bg_hex = crate::render::initial_bg_hex(&config.theme);
    let bg_rgba = parse_hex_to_rgba(&bg_hex).unwrap_or((30, 30, 46, 255));
    let mut builder = WindowBuilder::new()
        .with_title(title)
        .with_background_color(bg_rgba)
        .with_inner_size(LogicalSize::new(1200.0, 800.0))
        .with_min_inner_size(LogicalSize::new(600.0, 400.0));
    if let Some(icon) = load_icon() {
        builder = builder.with_window_icon(Some(icon));
    }
    let window = builder
        .build(&event_loop)
        .map_err(|e| anyhow::anyhow!("window build: {e}"))?;
    crate::profile::log("window_built");

    let initial_theme_name = if config.theme.mode.resolve_is_light() {
        config.theme.light.clone()
    } else {
        config.theme.dark.clone()
    };
    #[cfg(windows)]
    {
        let theme = mdview_theme::presets::find(&initial_theme_name)
            .or_else(|| mdview_theme::presets::find("catppuccin-mocha"))
            .or_else(|| mdview_theme::presets::builtin_themes().into_iter().next());
        if let Some(theme) = theme {
            apply_dwm_theme(&window, theme);
        }
    }
    let _ = initial_theme_name;

    let proxy = event_loop.create_proxy();
    let light_name = config.theme.light.clone();
    let dark_name = config.theme.dark.clone();
    let webview = WebViewBuilder::new()
        .with_url(url)
        .with_background_color(bg_rgba)
        .with_custom_protocol("mdview".into(), |_id, req| {
            let path_part = req.uri().path().trim_start_matches('/');
            let decoded = urlencoding::decode(path_part)
                .map(|s| s.into_owned())
                .unwrap_or_else(|_| path_part.to_string());
            let path = std::path::PathBuf::from(decoded);
            match std::fs::read(&path) {
                Ok(bytes) => wry::http::Response::builder()
                    .status(200)
                    .header(wry::http::header::CONTENT_TYPE, mime_for(&path))
                    .body(std::borrow::Cow::Owned(bytes))
                    .unwrap_or_else(|_| empty_404()),
                Err(_) => empty_404(),
            }
        })
        .with_ipc_handler(move |req| {
            let body = req.body().as_str();
            if let Some(event) = body.strip_prefix("profile:") {
                crate::profile::log(event);
                return;
            }
            let evt = match body {
                "theme-light" => Some(MdvUserEvent::ThemeChanged(light_name.clone())),
                "theme-dark" => Some(MdvUserEvent::ThemeChanged(dark_name.clone())),
                "quit" => Some(MdvUserEvent::Quit),
                "reload" => Some(MdvUserEvent::Reload),
                _ => body
                    .strip_prefix("open-link ")
                    .map(|u| MdvUserEvent::OpenLink(u.to_string())),
            };
            if let Some(evt) = evt {
                if let Err(e) = proxy.send_event(evt) {
                    tracing::debug!("send_event failed: {e}");
                }
            }
        })
        .build(&window)
        .map_err(|e| anyhow::anyhow!("webview build: {e}"))?;
    crate::profile::log("webview_built");

    event_loop.run(move |event, _target, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(MdvUserEvent::OpenLink(url)) => {
                open_url_in_browser(&url);
            }
            Event::UserEvent(MdvUserEvent::Quit) => {
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(MdvUserEvent::Reload) => {
                match std::fs::read_to_string(&reload_ctx.file) {
                    Ok(src) => {
                        let source_dir = reload_ctx.file.parent();
                        match crate::render::render_page_full(
                            &src,
                            &reload_ctx.title,
                            &reload_ctx.config.theme,
                            &reload_ctx.config.keymap,
                            source_dir,
                            &reload_ctx.config_errors,
                            &reload_ctx.config.toc,
                            &reload_ctx.config.codemap,
                            &reload_ctx.config.code,
                        ) {
                            Ok(html) => {
                                if let Ok(mut guard) = reload_ctx.html_store.write() {
                                    *guard = std::sync::Arc::new(html);
                                }
                            }
                            Err(e) => tracing::warn!("reload re-render failed: {e}"),
                        }
                    }
                    Err(e) => tracing::warn!(
                        "reload re-read failed for {}: {e}",
                        reload_ctx.file.display()
                    ),
                }
                if let Err(e) = webview.reload() {
                    tracing::debug!("webview reload failed: {e}");
                }
            }
            Event::UserEvent(MdvUserEvent::ThemeChanged(name)) => {
                #[cfg(windows)]
                {
                    if let Some(theme) = mdview_theme::presets::find(&name) {
                        apply_dwm_theme(&window, theme);
                    } else {
                        tracing::warn!("theme {name:?} not found for DWM update");
                    }
                }
                #[cfg(not(windows))]
                {
                    let _ = name;
                }
            }
            _ => {}
        }
    });
}

#[cfg(feature = "gui")]
fn open_url_in_browser(url: &str) {
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
fn empty_404() -> wry::http::Response<std::borrow::Cow<'static, [u8]>> {
    wry::http::Response::builder()
        .status(404)
        .body(std::borrow::Cow::Borrowed(&[][..]))
        .expect("static 404 response is well-formed")
}

#[cfg(feature = "gui")]
fn mime_for(path: &std::path::Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("bmp") => "image/bmp",
        Some("gif") => "image/gif",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

// DwmSetWindowAttribute is an FFI call; safe usage is constrained to passing a
// pointer to a value whose size matches `cbAttribute`, which we control here.
#[cfg(all(windows, feature = "gui"))]
#[allow(unsafe_code)]
fn apply_dwm_theme(window: &tao::window::Window, theme: &mdview_theme::Theme) {
    use tao::platform::windows::WindowExtWindows;
    use windows::Win32::Foundation::{COLORREF, HWND};
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
    };

    let Some(bg_hex) = theme.colors.get("bg") else {
        tracing::debug!("apply_dwm_theme: theme missing 'bg' color");
        return;
    };
    let Some(fg_hex) = theme.colors.get("fg") else {
        tracing::debug!("apply_dwm_theme: theme missing 'fg' color");
        return;
    };
    let Some(bg_ref) = parse_hex_to_colorref(bg_hex) else {
        tracing::debug!("apply_dwm_theme: failed to parse bg {bg_hex}");
        return;
    };
    let Some(fg_ref) = parse_hex_to_colorref(fg_hex) else {
        tracing::debug!("apply_dwm_theme: failed to parse fg {fg_hex}");
        return;
    };

    let hwnd = HWND(window.hwnd() as *mut _);
    let bg = COLORREF(bg_ref);
    let fg = COLORREF(fg_ref);
    let is_dark: u32 = if is_dark_hex(bg_hex) { 1 } else { 0 };

    unsafe {
        if let Err(e) = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CAPTION_COLOR,
            &bg as *const _ as *const _,
            std::mem::size_of::<COLORREF>() as u32,
        ) {
            tracing::debug!("DWMWA_CAPTION_COLOR failed: {e:?}");
        }
        if let Err(e) = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TEXT_COLOR,
            &fg as *const _ as *const _,
            std::mem::size_of::<COLORREF>() as u32,
        ) {
            tracing::debug!("DWMWA_TEXT_COLOR failed: {e:?}");
        }
        if let Err(e) = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR,
            &bg as *const _ as *const _,
            std::mem::size_of::<COLORREF>() as u32,
        ) {
            tracing::debug!("DWMWA_BORDER_COLOR failed: {e:?}");
        }
        if let Err(e) = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &is_dark as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        ) {
            tracing::debug!("DWMWA_USE_IMMERSIVE_DARK_MODE failed: {e:?}");
        }
    }
}

// Parse a `#rrggbb` (or `rrggbb`) hex string into an (R, G, B, 255) tuple, the
// shape both tao's `with_background_color` and wry's `with_background_color`
// accept.
#[cfg(feature = "gui")]
fn parse_hex_to_rgba(hex: &str) -> Option<(u8, u8, u8, u8)> {
    let s = hex.strip_prefix('#').unwrap_or(hex);
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some((r, g, b, 255))
}

// Windows COLORREF packs RGB as 0x00BBGGRR (little-endian byte order: R,G,B,0),
// NOT the usual 0x00RRGGBB. Swap byte positions when converting from a #RRGGBB hex.
fn parse_hex_to_colorref(hex: &str) -> Option<u32> {
    let s = hex.strip_prefix('#').unwrap_or(hex);
    if s.len() != 6 {
        return None;
    }
    let r = u32::from_str_radix(&s[0..2], 16).ok()?;
    let g = u32::from_str_radix(&s[2..4], 16).ok()?;
    let b = u32::from_str_radix(&s[4..6], 16).ok()?;
    Some((b << 16) | (g << 8) | r)
}

fn is_dark_hex(hex: &str) -> bool {
    let s = hex.strip_prefix('#').unwrap_or(hex);
    if s.len() != 6 {
        return true;
    }
    let r = u32::from_str_radix(&s[0..2], 16).unwrap_or(0) as f32;
    let g = u32::from_str_radix(&s[2..4], 16).unwrap_or(0) as f32;
    let b = u32::from_str_radix(&s[4..6], 16).unwrap_or(0) as f32;
    let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
    luminance < 128.0
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "gui")]
    use super::parse_hex_to_rgba;
    use super::{format_window_title, is_dark_hex, parse_hex_to_colorref};
    use std::path::Path;

    #[test]
    fn title_none_is_bare_mdview() {
        assert_eq!(format_window_title(None), "mdview");
    }

    #[test]
    fn title_simple_name() {
        assert_eq!(
            format_window_title(Some(Path::new("README.md"))),
            "README.md - mdview"
        );
    }

    #[test]
    fn title_full_path_uses_basename() {
        assert_eq!(
            format_window_title(Some(Path::new("D:/code/mdview/README.md"))),
            "README.md - mdview"
        );
    }

    #[test]
    fn title_root_path_is_untitled() {
        assert_eq!(
            format_window_title(Some(Path::new("/"))),
            "untitled - mdview"
        );
    }

    #[test]
    fn title_empty_path_is_untitled() {
        assert_eq!(
            format_window_title(Some(Path::new(""))),
            "untitled - mdview"
        );
    }

    #[test]
    fn colorref_byte_order_is_bgr() {
        // #FF8040 → R=0xFF, G=0x80, B=0x40 → COLORREF = 0x004080FF
        assert_eq!(parse_hex_to_colorref("#FF8040"), Some(0x0040_80FF));
    }

    #[test]
    fn colorref_accepts_no_hash() {
        assert_eq!(parse_hex_to_colorref("1e1e2e"), Some(0x002E_1E1E));
    }

    #[test]
    fn colorref_rejects_bad_input() {
        assert_eq!(parse_hex_to_colorref("#zzz"), None);
        assert_eq!(parse_hex_to_colorref("#12345"), None);
    }

    #[test]
    fn mocha_bg_is_dark() {
        assert!(is_dark_hex("#1e1e2e"));
    }

    #[test]
    fn latte_bg_is_light() {
        assert!(!is_dark_hex("#eff1f5"));
    }

    #[cfg(feature = "gui")]
    #[test]
    fn rgba_mocha_matches_window_color() {
        // catppuccin-mocha bg = #1e1e2e.
        assert_eq!(parse_hex_to_rgba("#1e1e2e"), Some((30, 30, 46, 255)));
    }

    #[cfg(feature = "gui")]
    #[test]
    fn rgba_latte_matches_window_color() {
        // catppuccin-latte bg = #eff1f5.
        assert_eq!(parse_hex_to_rgba("#eff1f5"), Some((239, 241, 245, 255)));
    }

    #[cfg(feature = "gui")]
    #[test]
    fn rgba_rejects_bad_input() {
        assert_eq!(parse_hex_to_rgba("#zzz"), None);
        assert_eq!(parse_hex_to_rgba("#12345"), None);
    }
}
