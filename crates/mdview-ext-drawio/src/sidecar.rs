use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use thiserror::Error;

pub const SIDECAR_BIN: &str = "mdview-sidecar";

#[derive(Debug, Error)]
pub enum SidecarError {
    #[error("mdview-sidecar binary not found on PATH")]
    NotFound,
    #[error("sidecar spawn failed: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("sidecar returned non-zero status: {0}")]
    NonZero(i32),
    #[error("sidecar produced empty output")]
    Empty,
}

/// Locate the sidecar binary.
///
/// Production callers pass `None` and detection falls through to
/// `which::which("mdview-sidecar")`. Tests may pass `Some(p)` to inject a
/// specific binary (or a non-existent path to simulate a missing sidecar)
/// without mutating the process environment.
pub fn locate_sidecar(override_path: Option<&Path>) -> Result<std::path::PathBuf, SidecarError> {
    if let Some(p) = override_path {
        return if p.exists() {
            Ok(p.to_path_buf())
        } else {
            Err(SidecarError::NotFound)
        };
    }
    which::which(SIDECAR_BIN).map_err(|_| SidecarError::NotFound)
}

pub fn run_sidecar(
    kind: &str,
    source: &str,
    override_path: Option<&Path>,
) -> Result<String, SidecarError> {
    let path = locate_sidecar(override_path)?;
    let payload = serde_json::json!({
        "kind": kind,
        "source": source,
    })
    .to_string();

    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(payload.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(SidecarError::NonZero(output.status.code().unwrap_or(-1)));
    }
    let svg = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if svg.is_empty() {
        return Err(SidecarError::Empty);
    }
    Ok(svg)
}

pub fn _unused_timeout() -> Duration {
    Duration::from_secs(5)
}
