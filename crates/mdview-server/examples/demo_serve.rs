use std::time::Duration;

use mdview_server::{serve, Config, Html, Theme};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let html = Html::new(
        "<main id=\"mdv-root\"><h1>mdview demo</h1><p>Live preview server online.</p></main>",
    );
    let theme = Theme::new(
        "demo",
        "body{font-family:ui-sans-serif,system-ui;max-width:48rem;margin:2rem auto;line-height:1.7;} h1{border-radius:12px;padding:.25rem .5rem;background:#f1f5f9;}",
    );
    let cfg = Config::new().with_html(html).with_theme(theme);
    let handle = serve(cfg).await?;
    println!("mdview-server listening on http://{}", handle.addr());
    println!("port={}", handle.port());

    let updater = handle.updater();
    tokio::spawn(async move {
        let mut tick = 0u32;
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            tick += 1;
            updater
                .push_doc(Html::new(format!(
                    "<main id=\"mdv-root\"><h1>mdview demo</h1><p>tick {tick}</p></main>"
                )))
                .await;
        }
    });

    tokio::signal::ctrl_c().await?;
    handle.shutdown().await?;
    Ok(())
}
