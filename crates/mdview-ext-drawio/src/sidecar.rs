use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

use thiserror::Error;

pub const SIDECAR_BIN: &str = "mdview-sidecar";
pub const SIDECAR_ENV: &str = "MDVIEW_SIDECAR";

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

/// Locate the sidecar binary. Honors the `MDVIEW_SIDECAR` override before
/// falling back to a PATH lookup, matching the other diagram extensions.
pub fn locate_sidecar() -> Result<std::path::PathBuf, SidecarError> {
    if let Some(env_path) = std::env::var_os(SIDECAR_ENV) {
        let p = std::path::PathBuf::from(env_path);
        if p.exists() {
            return Ok(p);
        }
        return Err(SidecarError::NotFound);
    }
    which::which(SIDECAR_BIN).map_err(|_| SidecarError::NotFound)
}

pub fn run_sidecar(kind: &str, source: &str) -> Result<String, SidecarError> {
    let path = locate_sidecar()?;
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
