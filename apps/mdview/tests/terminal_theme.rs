mod support;

use mdview_config::ThemeMode;
use std::time::Duration;
use tokio::process::Command;

const DARK_BACKGROUND: &str = "\x1b[48;2;41;45;66m";
const LIGHT_BACKGROUND: &str = "\x1b[48;2;218;227;245m";

fn assert_quote(output: &str, background: &str) {
    let quote = output
        .lines()
        .find(|line| line.contains("quoted line"))
        .unwrap();
    assert!(quote.starts_with(background), "{quote:?}");
    assert!(quote.ends_with("\x1b[0m"), "{quote:?}");
    assert_eq!(
        output
            .lines()
            .find(|line| line.contains("outside"))
            .unwrap(),
        "outside"
    );
}

async fn render_with_config(config: &str) -> String {
    let source = "> quoted line\n\noutside\n";
    let directory = tempfile::tempdir().unwrap();
    let config_dir = directory.path().join("mdview");
    let fixture = directory.path().join("input.md");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), config).unwrap();
    std::fs::write(&fixture, source).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_mdview"));
    command
        .args(["--terminal", "--no-pager"])
        .arg(&fixture)
        .env("XDG_CONFIG_HOME", directory.path());
    let (output, _) = support::run(command, Duration::from_secs(10))
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "render failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[tokio::test]
async fn terminal_quotes_auto_reuses_existing_resolution() {
    let background = if ThemeMode::Auto.resolve_is_light() {
        LIGHT_BACKGROUND
    } else {
        DARK_BACKGROUND
    };
    let output = render_with_config("[theme]\nmode = \"auto\"\n").await;
    assert_quote(&output, background);
}

#[tokio::test]
async fn terminal_quotes_follow_configured_theme() {
    let cases = [
        (
            "dark",
            "catppuccin-latte",
            "catppuccin-mocha",
            DARK_BACKGROUND,
        ),
        (
            "dark",
            "catppuccin-mocha",
            "catppuccin-latte",
            LIGHT_BACKGROUND,
        ),
        (
            "light",
            "catppuccin-latte",
            "catppuccin-mocha",
            LIGHT_BACKGROUND,
        ),
        (
            "light",
            "catppuccin-mocha",
            "catppuccin-latte",
            DARK_BACKGROUND,
        ),
    ];
    for (mode, light, dark, background) in cases {
        let config =
            format!("[theme]\nmode = \"{mode}\"\nlight = \"{light}\"\ndark = \"{dark}\"\n");
        let output = render_with_config(&config).await;
        assert_quote(&output, background);
    }
}

#[tokio::test]
async fn terminal_quotes_keep_existing_theme_fallback() {
    let output =
        render_with_config("[theme]\nmode = \"light\"\nlight = \"missing-preset\"\n").await;
    assert_quote(&output, DARK_BACKGROUND);
}
