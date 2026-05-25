use anyhow::Result;
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub type HtmlStore = Arc<RwLock<Arc<String>>>;

pub struct Server {
    pub port: u16,
    pub html: HtmlStore,
    _handle: tokio::task::JoinHandle<()>,
}

pub async fn serve_html(html: String) -> Result<Server> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let html: HtmlStore = Arc::new(RwLock::new(Arc::new(html)));

    let html_for_loop = html.clone();
    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let body = match html_for_loop.read() {
                Ok(guard) => guard.clone(),
                Err(_) => Arc::new(String::new()),
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 2048];
                let _ = stream.read(&mut buf).await;
                let bytes = body.as_bytes();
                let headers = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n",
                    bytes.len()
                );
                let _ = stream.write_all(headers.as_bytes()).await;
                let _ = stream.write_all(bytes).await;
                let _ = stream.flush().await;
            });
        }
    });

    Ok(Server {
        port,
        html,
        _handle: handle,
    })
}
