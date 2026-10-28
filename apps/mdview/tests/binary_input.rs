mod support;

use std::time::Duration;

use tokio::process::Command;

#[tokio::test]
async fn default_gui_mode_rejects_binary_input_before_daemonizing() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("program.exe");
    std::fs::write(&file, b"MZ\0\0binary").unwrap();

    let mut command = Command::new(env!("CARGO_BIN_EXE_mdview"));
    command.arg(&file).env_remove("MDVIEW_NO_DAEMONIZE");
    let (out, _) = support::run(command, Duration::from_secs(10))
        .await
        .expect("run mdview within its deadline");

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "expected failure, stderr: {stderr}");
    assert!(
        stderr.contains("is not a text/markdown file"),
        "stderr did not explain binary rejection: {stderr}"
    );
    assert!(
        stderr.contains(&file.display().to_string()),
        "stderr did not include input path: {stderr}"
    );
}
