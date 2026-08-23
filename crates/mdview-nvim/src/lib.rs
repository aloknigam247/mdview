#![forbid(unsafe_code)]

//! Neovim bridge: named-pipe / Unix-socket server, msgpack frames, theme cache.

pub mod _stubs;
pub mod protocol;

use crate::_stubs::{theme_from_nvim_highlights, NvimHl, Theme};
use crate::protocol::{Message, ProtocolError, MAX_FRAME_BYTES};

use futures_core::Stream;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Update {
        text: String,
        path: Option<PathBuf>,
    },
    Theme {
        colorscheme: String,
        version: String,
        hl: BTreeMap<String, NvimHl>,
        force: bool,
    },
    ThemeReady(Theme),
    Close,
}

pub async fn listen(
    pipe_path: &Path,
) -> Result<impl Stream<Item = Event> + Send + 'static, ProtocolError> {
    let cache = default_cache_dir().ok_or_else(|| {
        ProtocolError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no data dir available",
        ))
    })?;
    listen_with_cache(pipe_path, cache).await
}

pub async fn listen_with_cache(
    pipe_path: &Path,
    cache_dir: PathBuf,
) -> Result<impl Stream<Item = Event> + Send + 'static, ProtocolError> {
    let (tx, rx) = mpsc::channel::<Event>(64);
    spawn_listener(pipe_path.to_path_buf(), cache_dir, tx)?;
    Ok(ReceiverStream::new(rx))
}

#[cfg(unix)]
fn spawn_listener(
    pipe_path: PathBuf,
    cache_dir: PathBuf,
    tx: mpsc::Sender<Event>,
) -> Result<(), ProtocolError> {
    let _ = std::fs::remove_file(&pipe_path);
    let listener = tokio::net::UnixListener::bind(&pipe_path)?;
    tokio::spawn(async move {
        let mut current: Option<tokio::task::JoinHandle<()>> = None;
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    if let Some(handle) = current.take() {
                        handle.abort();
                    }
                    let tx2 = tx.clone();
                    let cache2 = cache_dir.clone();
                    current = Some(tokio::spawn(async move {
                        let (r, _w) = stream.into_split();
                        let _ = serve_client(r, cache2, tx2).await;
                    }));
                }
                Err(_) => break,
            }
        }
    });
    Ok(())
}

#[cfg(windows)]
fn spawn_listener(
    pipe_path: PathBuf,
    cache_dir: PathBuf,
    tx: mpsc::Sender<Event>,
) -> Result<(), ProtocolError> {
    use tokio::net::windows::named_pipe::ServerOptions;

    let pipe_name = pipe_path
        .to_str()
        .ok_or_else(|| {
            ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "pipe path must be valid UTF-8",
            ))
        })?
        .to_string();

    let mut server = ServerOptions::new()
        .first_pipe_instance(true)
        .create(&pipe_name)?;

    tokio::spawn(async move {
        let mut current: Option<tokio::task::JoinHandle<()>> = None;
        loop {
            if server.connect().await.is_err() {
                break;
            }
            let connected = server;
            let next = match ServerOptions::new().create(&pipe_name) {
                Ok(s) => s,
                Err(_) => break,
            };
            server = next;

            if let Some(handle) = current.take() {
                handle.abort();
            }
            let tx2 = tx.clone();
            let cache2 = cache_dir.clone();
            current = Some(tokio::spawn(async move {
                let (r, _w) = tokio::io::split(connected);
                let _ = serve_client(r, cache2, tx2).await;
            }));
        }
    });
    Ok(())
}

async fn serve_client<R>(
    mut reader: R,
    cache_dir: PathBuf,
    tx: mpsc::Sender<Event>,
) -> Result<(), ProtocolError>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    loop {
        let msg = match read_frame(&mut reader).await {
            Ok(m) => m,
            Err(_) => {
                let _ = tx.send(Event::Close).await;
                return Ok(());
            }
        };
        let events = handle_message(msg, &cache_dir).await;
        for ev in events {
            let is_close = matches!(ev, Event::Close);
            if tx.send(ev).await.is_err() {
                return Ok(());
            }
            if is_close {
                return Ok(());
            }
        }
    }
}

async fn handle_message(msg: Message, cache_dir: &Path) -> Vec<Event> {
    match msg {
        Message::Update { text, path } => vec![Event::Update { text, path }],
        Message::Close => vec![Event::Close],
        Message::Theme {
            colorscheme,
            version,
            hl,
            force,
        } => {
            let mut out = vec![Event::Theme {
                colorscheme: colorscheme.clone(),
                version: version.clone(),
                hl: hl.clone(),
                force,
            }];
            if let Ok(theme) = resolve_theme_in(cache_dir, &colorscheme, &version, &hl, force).await
            {
                out.push(Event::ThemeReady(theme));
            }
            out
        }
    }
}

pub async fn read_frame<R>(reader: &mut R) -> Result<Message, ProtocolError>
where
    R: AsyncRead + Unpin,
{
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf);
    if len > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge(len));
    }
    let mut body = vec![0u8; len as usize];
    reader.read_exact(&mut body).await?;
    let value = rmpv::decode::read_value(&mut &body[..])?;
    rmpv::ext::from_value::<Message>(value).map_err(|e| ProtocolError::Serde(e.to_string()))
}

pub async fn write_frame<W>(writer: &mut W, msg: &Message) -> Result<(), ProtocolError>
where
    W: AsyncWrite + Unpin,
{
    let value = rmpv::ext::to_value(msg).map_err(|e| ProtocolError::Serde(e.to_string()))?;
    let mut body: Vec<u8> = Vec::new();
    rmpv::encode::write_value(&mut body, &value)?;
    if body.len() as u64 > MAX_FRAME_BYTES as u64 {
        return Err(ProtocolError::FrameTooLarge(body.len() as u32));
    }
    let len = (body.len() as u32).to_be_bytes();
    writer.write_all(&len).await?;
    writer.write_all(&body).await?;
    writer.flush().await?;
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CacheEntry {
    colorscheme: String,
    version: String,
    theme: Theme,
}

pub fn default_cache_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("mdview").join("theme-cache"))
}

fn cache_file_in(dir: &Path, colorscheme: &str, version: &str) -> PathBuf {
    dir.join(format!("{colorscheme}-{version}.json"))
}

async fn read_cached_theme(dir: &Path, colorscheme: &str, version: &str) -> Option<Theme> {
    let path = cache_file_in(dir, colorscheme, version);
    let bytes = tokio::fs::read(&path).await.ok()?;
    let entry: CacheEntry = serde_json::from_slice(&bytes).ok()?;
    Some(entry.theme)
}

async fn write_cached_theme(
    dir: &Path,
    colorscheme: &str,
    version: &str,
    theme: &Theme,
) -> Result<(), std::io::Error> {
    tokio::fs::create_dir_all(dir).await?;
    let path = cache_file_in(dir, colorscheme, version);
    let entry = CacheEntry {
        colorscheme: colorscheme.to_string(),
        version: version.to_string(),
        theme: theme.clone(),
    };
    let bytes = serde_json::to_vec_pretty(&entry)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    tokio::fs::write(&path, bytes).await
}

pub async fn resolve_theme_in(
    dir: &Path,
    colorscheme: &str,
    version: &str,
    hl: &BTreeMap<String, NvimHl>,
    force: bool,
) -> Result<Theme, std::io::Error> {
    if !force {
        if let Some(theme) = read_cached_theme(dir, colorscheme, version).await {
            return Ok(theme);
        }
    }
    let theme = theme_from_nvim_highlights(colorscheme, version, hl);
    let _ = write_cached_theme(dir, colorscheme, version, &theme).await;
    Ok(theme)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::duplex;

    fn make_update() -> Message {
        Message::Update {
            text: "# hello".to_string(),
            path: Some(PathBuf::from("/tmp/foo.md")),
        }
    }

    fn make_theme(force: bool) -> Message {
        let mut hl = BTreeMap::new();
        hl.insert(
            "Normal".to_string(),
            NvimHl {
                fg: Some(0xffffff),
                bg: Some(0x000000),
                bold: false,
                italic: false,
                underline: false,
            },
        );
        Message::Theme {
            colorscheme: "solarized".to_string(),
            version: "0.10.0".to_string(),
            hl,
            force,
        }
    }

    #[tokio::test]
    async fn roundtrip_update() {
        let (mut a, mut b) = duplex(4096);
        let msg = make_update();
        write_frame(&mut a, &msg).await.unwrap();
        let got = read_frame(&mut b).await.unwrap();
        assert_eq!(got, msg);
    }

    #[tokio::test]
    async fn roundtrip_theme() {
        let (mut a, mut b) = duplex(4096);
        let msg = make_theme(false);
        write_frame(&mut a, &msg).await.unwrap();
        let got = read_frame(&mut b).await.unwrap();
        assert_eq!(got, msg);
    }

    #[tokio::test]
    async fn roundtrip_close() {
        let (mut a, mut b) = duplex(4096);
        write_frame(&mut a, &Message::Close).await.unwrap();
        let got = read_frame(&mut b).await.unwrap();
        assert_eq!(got, Message::Close);
    }

    #[tokio::test]
    async fn serve_client_emits_events_then_close() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path().to_path_buf();
        let (mut client, server) = duplex(8192);
        let (tx, mut rx) = mpsc::channel::<Event>(16);

        let (sr, _sw) = tokio::io::split(server);
        let task = tokio::spawn(async move { serve_client(sr, cache, tx).await });

        write_frame(&mut client, &make_update()).await.unwrap();
        write_frame(&mut client, &make_theme(false)).await.unwrap();
        write_frame(&mut client, &Message::Close).await.unwrap();

        let e1 = rx.recv().await.unwrap();
        assert!(matches!(e1, Event::Update { .. }));
        let e2 = rx.recv().await.unwrap();
        assert!(matches!(e2, Event::Theme { .. }));
        let e3 = rx.recv().await.unwrap();
        assert!(matches!(e3, Event::ThemeReady(_)));
        let e4 = rx.recv().await.unwrap();
        assert!(matches!(e4, Event::Close));

        drop(client);
        let _ = task.await;
    }

    #[tokio::test]
    async fn theme_cache_miss_then_hit() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        let colorscheme = "miss_hit";
        let version = "1.0.0";
        let mut hl = BTreeMap::new();
        hl.insert(
            "Normal".to_string(),
            NvimHl {
                fg: Some(0x112233),
                bg: Some(0x445566),
                bold: false,
                italic: false,
                underline: false,
            },
        );

        let file = cache_file_in(dir, colorscheme, version);
        assert!(!file.exists());

        let t1 = resolve_theme_in(dir, colorscheme, version, &hl, false)
            .await
            .unwrap();
        assert!(file.exists());
        assert_eq!(t1.name, colorscheme);

        let mut hl2 = BTreeMap::new();
        hl2.insert(
            "Normal".to_string(),
            NvimHl {
                fg: Some(0xdeadbe),
                bg: Some(0xefbead),
                bold: false,
                italic: false,
                underline: false,
            },
        );
        let t2 = resolve_theme_in(dir, colorscheme, version, &hl2, false)
            .await
            .unwrap();
        assert_eq!(t1, t2, "second call must come from cache, not new hl");
    }

    #[tokio::test]
    async fn theme_force_bypasses_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        let colorscheme = "force";
        let version = "1.0.0";
        let mut hl = BTreeMap::new();
        hl.insert(
            "Normal".to_string(),
            NvimHl {
                fg: Some(0x111111),
                bg: Some(0x222222),
                bold: false,
                italic: false,
                underline: false,
            },
        );
        let t1 = resolve_theme_in(dir, colorscheme, version, &hl, false)
            .await
            .unwrap();

        let mut hl2 = BTreeMap::new();
        hl2.insert(
            "Normal".to_string(),
            NvimHl {
                fg: Some(0xaaaaaa),
                bg: Some(0xbbbbbb),
                bold: false,
                italic: false,
                underline: false,
            },
        );
        let t2 = resolve_theme_in(dir, colorscheme, version, &hl2, true)
            .await
            .unwrap();

        assert_ne!(t1, t2, "force:true must rebuild from fresh hl");
        assert_eq!(t2.colors.get("fg").map(String::as_str), Some("#aaaaaa"));
    }

    #[tokio::test]
    async fn resolve_theme_in_produces_expected_colors() {
        let tmp = tempfile::tempdir().unwrap();
        let mut hl = BTreeMap::new();
        hl.insert(
            "Normal".to_string(),
            NvimHl {
                fg: Some(0xeeeeee),
                bg: Some(0x111111),
                bold: false,
                italic: false,
                underline: false,
            },
        );
        let theme = resolve_theme_in(tmp.path(), "expected_colors", "1", &hl, false)
            .await
            .unwrap();
        assert_eq!(theme.colors.get("fg").map(String::as_str), Some("#eeeeee"));
        assert_eq!(theme.colors.get("bg").map(String::as_str), Some("#111111"));
    }
}
