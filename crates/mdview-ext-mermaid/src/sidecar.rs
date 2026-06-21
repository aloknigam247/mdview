use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use thiserror::Error;

pub const SIDECAR_BIN: &str = "mdview-sidecar";
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Error)]
pub enum SidecarError {
    #[error("sidecar binary not found")]
    NotFound,
    #[error("sidecar spawn failed: {0}")]
    Spawn(String),
    #[error("sidecar io error: {0}")]
    Io(String),
    #[error("sidecar timed out after {0:?}")]
    Timeout(Duration),
    #[error("sidecar exited with status {0}")]
    ExitStatus(i32),
}

/// Locate the sidecar binary.
///
/// Production callers pass `None` and detection falls through to a `PATH`
/// lookup plus a co-located-with-executable check. Tests may pass `Some(p)`
/// to inject a specific binary (or a non-existent path to simulate a missing
/// sidecar) without mutating the process environment.
pub fn locate_sidecar(override_path: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = override_path {
        return if p.exists() {
            Some(p.to_path_buf())
        } else {
            None
        };
    }
    if let Ok(p) = which::which(SIDECAR_BIN) {
        return Some(p);
    }
    if let Ok(current) = std::env::current_exe() {
        if let Some(dir) = current.parent() {
            let mut name = OsString::from(SIDECAR_BIN);
            if cfg!(windows) {
                name.push(".exe");
            }
            let candidate = dir.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Run the sidecar with the given JSON payload. Returns stdout bytes (expected
/// SVG on success).
pub fn run_sidecar(bin: &std::path::Path, payload: &[u8]) -> Result<Vec<u8>, SidecarError> {
    run_sidecar_with_timeout(bin, payload, DEFAULT_TIMEOUT)
}

pub fn run_sidecar_with_timeout(
    bin: &std::path::Path,
    payload: &[u8],
    timeout: Duration,
) -> Result<Vec<u8>, SidecarError> {
    let mut cmd = Command::new(bin);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| SidecarError::Spawn(e.to_string()))?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| SidecarError::Io("stdin unavailable".into()))?;
        stdin
            .write_all(payload)
            .map_err(|e| SidecarError::Io(e.to_string()))?;
    }
    drop(child.stdin.take());

    let (tx, rx) = mpsc::channel();
    let mut stdout = child.stdout.take();
    thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(stream) = stdout.as_mut() {
            let _ = stream.read_to_end(&mut buf);
        }
        let _ = tx.send(buf);
    });

    let stdout_bytes = match rx.recv_timeout(timeout) {
        Ok(b) => b,
        Err(_) => {
            let _ = child.kill();
            return Err(SidecarError::Timeout(timeout));
        }
    };

    let status = child.wait().map_err(|e| SidecarError::Io(e.to_string()))?;
    if !status.success() {
        return Err(SidecarError::ExitStatus(status.code().unwrap_or(-1)));
    }

    Ok(stdout_bytes)
}
