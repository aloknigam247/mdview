#![forbid(unsafe_code)]

mod _stubs;

pub use _stubs::{Html, RenderCtx, Theme};

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;
use tower_http::compression::CompressionLayer;

const TEMPLATE: &str = include_str!("template.html");
const TEMPLATE_SLOT: &str = "__MDVIEW_INITIAL_HTML__";
const BROADCAST_CAPACITY: usize = 64;

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("join error: {0}")]
    Join(#[from] tokio::task::JoinError),
}

pub type Result<T> = std::result::Result<T, ServerError>;

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub initial_html: Html,
    pub theme: Theme,
    pub assets: BTreeMap<String, Asset>,
}

#[derive(Clone, Debug)]
pub struct Asset {
    pub bytes: Bytes,
    pub content_type: &'static str,
    pub path: String,
}

impl Asset {
    pub fn new<P: Into<String>, B: Into<Bytes>>(
        path: P,
        bytes: B,
        content_type: &'static str,
    ) -> Self {
        Self {
            bytes: bytes.into(),
            content_type,
            path: path.into(),
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_html(mut self, html: Html) -> Self {
        self.initial_html = html;
        self
    }

    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn with_assets<I: IntoIterator<Item = Asset>>(mut self, assets: I) -> Self {
        for asset in assets {
            self.assets.insert(asset.path.clone(), asset);
        }
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum LiveEvent {
    Full { html: String },
    Patch { html: String },
    Theme { css: String },
    Close,
}

#[derive(Debug)]
struct SharedState {
    doc: RwLock<Html>,
    theme_css: RwLock<String>,
    assets: RwLock<BTreeMap<String, Asset>>,
    tx: broadcast::Sender<LiveEvent>,
}

impl SharedState {
    fn new(cfg: Config) -> Arc<Self> {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Arc::new(Self {
            doc: RwLock::new(cfg.initial_html),
            theme_css: RwLock::new(mdview_theme::emit_css(&cfg.theme)),
            assets: RwLock::new(cfg.assets),
            tx,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Updater {
    state: Arc<SharedState>,
}

impl Updater {
    pub async fn push_doc(&self, html: Html) {
        let payload = html.0.clone();
        *self.state.doc.write().await = html;
        let _ = self.state.tx.send(LiveEvent::Patch { html: payload });
    }

    pub async fn push_full(&self, html: Html) {
        let payload = html.0.clone();
        *self.state.doc.write().await = html;
        let _ = self.state.tx.send(LiveEvent::Full { html: payload });
    }

    pub async fn push_theme(&self, css: String) {
        *self.state.theme_css.write().await = css.clone();
        let _ = self.state.tx.send(LiveEvent::Theme { css });
    }

    pub async fn upsert_asset(&self, asset: Asset) {
        self.state
            .assets
            .write()
            .await
            .insert(asset.path.clone(), asset);
    }

    pub fn close(&self) {
        let _ = self.state.tx.send(LiveEvent::Close);
    }
}

pub struct Server {
    pub handle: ServerHandle,
}

impl Server {
    pub async fn start(cfg: Config) -> Result<Self> {
        Ok(Self {
            handle: serve(cfg).await?,
        })
    }

    pub fn handle(&self) -> &ServerHandle {
        &self.handle
    }

    pub async fn shutdown(self) -> Result<()> {
        self.handle.shutdown().await
    }
}

pub struct ServerHandle {
    addr: SocketAddr,
    updater: Updater,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    join: Option<JoinHandle<std::io::Result<()>>>,
}

impl ServerHandle {
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    pub fn updater(&self) -> Updater {
        self.updater.clone()
    }

    pub async fn shutdown(mut self) -> Result<()> {
        self.updater.close();
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(join) = self.join.take() {
            join.await??;
        }
        Ok(())
    }
}

pub async fn serve(cfg: Config) -> Result<ServerHandle> {
    let state = SharedState::new(cfg);
    let updater = Updater {
        state: state.clone(),
    };

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/theme.css", get(theme_handler))
        .route("/assets/*path", get(asset_handler))
        .route("/__mdview_live", get(ws_handler))
        .with_state(state)
        .layer(CompressionLayer::new());

    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let addr = listener.local_addr()?;
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let join = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await
    });

    Ok(ServerHandle {
        addr,
        updater,
        shutdown: Some(shutdown_tx),
        join: Some(join),
    })
}

async fn root_handler(State(state): State<Arc<SharedState>>) -> Response {
    let doc = state.doc.read().await;
    let body = TEMPLATE.replacen(TEMPLATE_SLOT, doc.as_str(), 1);
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        body,
    )
        .into_response()
}

async fn theme_handler(State(state): State<Arc<SharedState>>) -> Response {
    let theme_css = state.theme_css.read().await;
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        theme_css.clone(),
    )
        .into_response()
}

async fn asset_handler(
    State(state): State<Arc<SharedState>>,
    Path(path): Path<String>,
) -> Response {
    let assets = state.assets.read().await;
    match assets.get(&path) {
        Some(asset) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, asset.content_type)],
            asset.bytes.clone(),
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "asset not found").into_response(),
    }
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<SharedState>>) -> Response {
    ws.on_upgrade(move |socket| ws_session(socket, state))
}

async fn ws_session(mut socket: WebSocket, state: Arc<SharedState>) {
    let mut rx = state.tx.subscribe();
    let full = {
        let doc = state.doc.read().await;
        LiveEvent::Full {
            html: doc.0.clone(),
        }
    };
    if !send_event(&mut socket, &full).await {
        return;
    }

    loop {
        tokio::select! {
            biased;
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => continue,
                    Some(Err(_)) => break,
                }
            }
            event = rx.recv() => {
                match event {
                    Ok(evt) => {
                        let is_close = matches!(evt, LiveEvent::Close);
                        if !send_event(&mut socket, &evt).await {
                            break;
                        }
                        if is_close {
                            let _ = socket.send(Message::Close(None)).await;
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}

async fn send_event(socket: &mut WebSocket, event: &LiveEvent) -> bool {
    let Ok(text) = serde_json::to_string(event) else {
        return false;
    };
    socket.send(Message::Text(text)).await.is_ok()
}
