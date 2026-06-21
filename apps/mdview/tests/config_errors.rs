//! Integration test: config errors are a HARD FAIL.
//!
//! Any parse or validation error in `config.toml` must:
//!   1. Print one `<path>:<line> — <message>` line per error to stderr.
//!   2. Exit with a non-zero status.
//!   3. NOT spawn the GUI / detached child (no daemonize fork).

use std::process::{Command, Stdio};

fn write_config(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
    let cfg_dir = dir.join("mdview");
    std::fs::create_dir_all(&cfg_dir).expect("create config dir");
    let cfg_path = cfg_dir.join("config.toml");
    std::fs::write(&cfg_path, body).expect("write config");
    cfg_path
}

fn run_with_config(body: &str) -> (std::process::Output, std::path::PathBuf) {
    let bin = env!("CARGO_BIN_EXE_mdview");
    let tmp = tempfile::tempdir().expect("tempdir");
    let cfg_path = write_config(tmp.path(), body);
    let fixture = std::env::current_dir()
        .unwrap()
        .join("../../fixtures/gfm.md");
    let out = Command::new(bin)
        .arg(&fixture)
        .env("XDG_CONFIG_HOME", tmp.path())
        .env("HOME", tmp.path())
        .env("USERPROFILE", tmp.path())
        // Belt-and-braces: even if the child somehow tries to fork, this
        // env flag keeps it in-process so the parent exit status reflects
        // what main.rs decided. (preflight_config runs before daemonize.)
        .env("MDVIEW_NO_DAEMONIZE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn mdview");
    (out, cfg_path)
}

#[test]
fn malformed_toml_hard_fails_with_line_and_message_on_stderr() {
    // Bad TOML on line 3.
    let body = "[toc]\n#good comment\nthis is = not [valid toml\n";
    let (out, cfg_path) = run_with_config(body);

    assert!(
        !out.status.success(),
        "expected non-zero exit; got {:?}",
        out.status
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let path_str = cfg_path.display().to_string();
    assert!(
        stderr.contains(&path_str),
        "stderr should mention config path {path_str:?}; got:\n{stderr}"
    );
    assert!(
        stderr.contains(" \u{2014} "),
        "stderr should use ' — ' separator; got:\n{stderr}"
    );
    assert!(
        stderr.contains("invalid TOML"),
        "stderr should contain TOML diagnostic; got:\n{stderr}"
    );
    // Format sanity: `<path>:<line> — <msg>` -- there must be a digit between
    // the path and the em-dash separator.
    let prefix = format!("{path_str}:");
    let line = stderr
        .lines()
        .find(|l| l.starts_with(&prefix))
        .expect("at least one error line starting with the config path");
    let after_colon = &line[prefix.len()..];
    let line_no: usize = after_colon
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .expect("a numeric line number after `<path>:`");
    assert!(line_no >= 1, "line number should be >= 1; got {line_no}");
}

#[test]
fn validation_error_hard_fails_with_line_and_message_on_stderr() {
    // [toc] depth = 9 is out of range (1..=6) — value-validation error.
    let body = "[toc]\ndepth = 9\n";
    let (out, cfg_path) = run_with_config(body);

    assert!(
        !out.status.success(),
        "expected non-zero exit; got {:?}",
        out.status
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let path_str = cfg_path.display().to_string();
    assert!(
        stderr.contains(&path_str),
        "stderr should mention config path {path_str:?}; got:\n{stderr}"
    );
    assert!(
        stderr.contains(" \u{2014} "),
        "stderr should use ' — ' separator; got:\n{stderr}"
    );
    assert!(
        stderr.contains("1..=6"),
        "stderr should explain expected range; got:\n{stderr}"
    );
    // The offending key `depth =` sits on line 2 in the fixture.
    let expected_prefix = format!("{path_str}:2 \u{2014}");
    assert!(
        stderr.contains(&expected_prefix),
        "stderr should pin the error to line 2 ({expected_prefix:?}); got:\n{stderr}"
    );
}
