mod support;

use std::time::Duration;

use tokio::process::Command;

#[tokio::test]
async fn daemonize_spawns_detached_child() {
    let bin = env!("CARGO_BIN_EXE_mdview");
    let config = tempfile::tempdir().expect("isolated config");
    let fixture = std::env::current_dir()
        .unwrap()
        .join("../../fixtures/gfm.md");
    let mut command = Command::new(bin);
    command
        .arg(&fixture)
        .env("XDG_CONFIG_HOME", config.path())
        .env_remove("MDVIEW_DETACHED")
        .env_remove("MDVIEW_NO_DAEMONIZE")
        .env_remove("MDV_PROFILE");
    let (out, elapsed) = support::run(command, Duration::from_secs(10))
        .await
        .expect("run daemonizing parent within its deadline");
    let status = out.status;

    assert!(
        status.success(),
        "parent exit status should be 0; got {status:?}"
    );
    // Debug binaries on Windows have nontrivial startup; 10s is generous
    // but still catches a truly hung parent (blocked on the event loop).
    assert!(
        elapsed < Duration::from_secs(10),
        "parent should return promptly after daemonising (took {elapsed:?})"
    );
}
