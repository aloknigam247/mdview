use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use mdview_server::{serve, Asset, Config, Html, LiveEvent, Theme};
use tokio_tungstenite::tungstenite::Message;

fn cfg() -> Config {
    Config::new()
        .with_html(Html::new("<p>hello</p>"))
        .with_theme(Theme::default_dark())
        .with_assets([Asset::new(
            "pixel.txt",
            bytes::Bytes::from_static(b"ok"),
            "text/plain",
        )])
}

#[tokio::test]
async fn serves_index() {
    let handle = serve(cfg()).await.unwrap();
    let url = format!("http://{}/", handle.addr());
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("<p>hello</p>"));
    assert!(body.contains("__mdview_live"));
    handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn serves_theme_and_assets() {
    let handle = serve(cfg()).await.unwrap();
    let theme_url = format!("http://{}/theme.css", handle.addr());
    let theme_body = reqwest::get(&theme_url)
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(theme_body.contains("--mdv-fg"));

    let asset_url = format!("http://{}/assets/pixel.txt", handle.addr());
    let asset_body = reqwest::get(&asset_url)
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(asset_body, "ok");
    handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn ws_pushes_full_then_patch() {
    let handle = serve(cfg()).await.unwrap();
    let ws_url = format!("ws://{}/__mdview_live", handle.addr());
    let (mut socket, _resp) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();

    let first = tokio::time::timeout(Duration::from_secs(2), socket.next())
        .await
        .expect("ws recv timeout")
        .expect("stream closed")
        .expect("ws err");
    let text = match first {
        Message::Text(t) => t.to_string(),
        other => panic!("expected text, got {other:?}"),
    };
    let evt: LiveEvent = serde_json::from_str(&text).unwrap();
    match evt {
        LiveEvent::Full { html } => assert!(html.contains("hello")),
        other => panic!("expected full, got {other:?}"),
    }

    handle.updater().push_doc(Html::new("<p>next</p>")).await;

    let next = tokio::time::timeout(Duration::from_secs(2), socket.next())
        .await
        .expect("ws recv timeout")
        .expect("stream closed")
        .expect("ws err");
    let text = match next {
        Message::Text(t) => t.to_string(),
        other => panic!("expected text, got {other:?}"),
    };
    let evt: LiveEvent = serde_json::from_str(&text).unwrap();
    match evt {
        LiveEvent::Patch { html } => assert!(html.contains("next")),
        other => panic!("expected patch, got {other:?}"),
    }

    let _ = socket.send(Message::Close(None)).await;
    handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn ws_pushes_theme() {
    let handle = serve(cfg()).await.unwrap();
    let ws_url = format!("ws://{}/__mdview_live", handle.addr());
    let (mut socket, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();

    let _full = socket.next().await.unwrap().unwrap();

    handle.updater().push_theme("body{color:#000}".into()).await;
    let msg = tokio::time::timeout(Duration::from_secs(2), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let text = match msg {
        Message::Text(t) => t.to_string(),
        other => panic!("expected text, got {other:?}"),
    };
    let evt: LiveEvent = serde_json::from_str(&text).unwrap();
    match evt {
        LiveEvent::Theme { css } => assert!(css.contains("#000")),
        other => panic!("expected theme, got {other:?}"),
    }
    handle.shutdown().await.unwrap();
}
