//! Local invocation of the `d2` CLI.
//!
//! We render via `d2 --omit-xml-tag --animate-interval=1000 - <output>`, reading
//! the diagram source from stdin and writing the SVG to a temp file. If the CLI
//! is not on `PATH` we surface a structured error so callers can place a notice
//! in the rendered output.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

pub const D2_BIN: &str = "d2";
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
pub const INSTALL_HINT: &str = "d2 CLI not found on PATH. Install: https://d2lang.com/tour/install";

#[derive(Debug)]
pub enum D2Error {
    NotFound,
    Spawn(String),
    Io(String),
    ExitStatus { code: i32, stderr: String },
}

impl std::fmt::Display for D2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            D2Error::NotFound => write!(f, "{INSTALL_HINT}"),
            D2Error::Spawn(s) => write!(f, "d2 spawn failed: {s}"),
            D2Error::Io(s) => write!(f, "d2 io error: {s}"),
            D2Error::ExitStatus { code, stderr } => {
                write!(f, "d2 exited with status {code}: {stderr}")
            }
        }
    }
}

impl std::error::Error for D2Error {}

/// Locate the `d2` binary.
///
/// Production callers pass `None` and detection falls through to
/// `which::which("d2")`. Tests may pass `Some(path)` to inject a specific
/// binary (or a non-existent path to simulate a missing CLI) without
/// mutating the process environment.
pub fn locate_d2(override_path: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = override_path {
        return if p.exists() {
            Some(p.to_path_buf())
        } else {
            None
        };
    }
    which::which(D2_BIN).ok()
}

/// Render a `d2` diagram to SVG bytes by invoking the local CLI.
pub fn render_svg(source: &str) -> Result<Vec<u8>, D2Error> {
    render_svg_with(source, None, DEFAULT_TIMEOUT)
}

/// Render with an explicit binary-path override and timeout. Production
/// callers should use [`render_svg`]; tests use this to inject a fake or
/// missing binary deterministically.
pub fn render_svg_with(
    source: &str,
    override_path: Option<&Path>,
    _timeout: Duration,
) -> Result<Vec<u8>, D2Error> {
    let Some(bin) = locate_d2(override_path) else {
        return Err(D2Error::NotFound);
    };

    let tmp = tempfile::Builder::new()
        .prefix("mdview-d2-")
        .suffix(".svg")
        .tempfile()
        .map_err(|e| D2Error::Io(e.to_string()))?;
    let out_path = tmp.path().to_path_buf();

    let mut cmd = Command::new(&bin);
    cmd.arg("--omit-xml-tag")
        .arg("--animate-interval=1000")
        .arg("-")
        .arg(&out_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| D2Error::Spawn(e.to_string()))?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| D2Error::Io("stdin unavailable".into()))?;
        stdin
            .write_all(source.as_bytes())
            .map_err(|e| D2Error::Io(e.to_string()))?;
    }
    drop(child.stdin.take());

    let output = child
        .wait_with_output()
        .map_err(|e| D2Error::Io(e.to_string()))?;

    if !output.status.success() {
        // Some `d2` versions reject `--animate-interval` for single-board
        // diagrams. Retry once without it to get a plain SVG.
        let stderr_text = String::from_utf8_lossy(&output.stderr).into_owned();
        if stderr_text.contains("animate-interval") || stderr_text.contains("animation") {
            return render_svg_plain(&bin, source);
        }
        return Err(D2Error::ExitStatus {
            code: output.status.code().unwrap_or(-1),
            stderr: stderr_text,
        });
    }

    let bytes = std::fs::read(&out_path).map_err(|e| D2Error::Io(e.to_string()))?;
    Ok(bytes)
}

fn render_svg_plain(bin: &std::path::Path, source: &str) -> Result<Vec<u8>, D2Error> {
    let tmp = tempfile::Builder::new()
        .prefix("mdview-d2-")
        .suffix(".svg")
        .tempfile()
        .map_err(|e| D2Error::Io(e.to_string()))?;
    let out_path = tmp.path().to_path_buf();

    let mut cmd = Command::new(bin);
    cmd.arg("--omit-xml-tag")
        .arg("-")
        .arg(&out_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| D2Error::Spawn(e.to_string()))?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| D2Error::Io("stdin unavailable".into()))?;
        stdin
            .write_all(source.as_bytes())
            .map_err(|e| D2Error::Io(e.to_string()))?;
    }
    drop(child.stdin.take());

    let output = child
        .wait_with_output()
        .map_err(|e| D2Error::Io(e.to_string()))?;
    if !output.status.success() {
        return Err(D2Error::ExitStatus {
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    std::fs::read(&out_path).map_err(|e| D2Error::Io(e.to_string()))
}
