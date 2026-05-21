use std::io::{IsTerminal, Read, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Rough approximation of what the host terminal can render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCaps {
    pub sixel: bool,
    pub kitty: bool,
    pub truecolor: bool,
    pub width: u16,
    pub height: u16,
}

impl Default for TerminalCaps {
    fn default() -> Self {
        Self {
            sixel: false,
            kitty: false,
            truecolor: false,
            width: 0,
            height: 0,
        }
    }
}

/// Probe the current tty for sixel / kitty / truecolor support and terminal
/// dimensions. On a non-tty or on error, returns an all-false, zero-sized
/// `TerminalCaps`.
pub fn probe() -> TerminalCaps {
    let mut caps = TerminalCaps::default();

    if let Ok((w, h)) = crossterm::terminal::size() {
        caps.width = w;
        caps.height = h;
    }

    if let Ok(v) = std::env::var("COLORTERM") {
        let v = v.to_ascii_lowercase();
        if v.contains("truecolor") || v.contains("24bit") {
            caps.truecolor = true;
        }
    }

    caps.kitty = detect_kitty();

    if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        caps.sixel = query_da1_sixel().unwrap_or(false);
    }

    caps
}

fn detect_kitty() -> bool {
    if std::env::var_os("KITTY_WINDOW_ID").is_some() {
        return true;
    }
    match std::env::var("TERM") {
        Ok(t) => {
            let t = t.to_ascii_lowercase();
            t.contains("kitty") || t.contains("ghostty") || t.contains("wezterm")
        }
        Err(_) => false,
    }
}

fn query_da1_sixel() -> std::io::Result<bool> {
    let raw_enabled = crossterm::terminal::enable_raw_mode().is_ok();

    let mut out = std::io::stdout();
    out.write_all(b"\x1b[c")?;
    out.flush()?;

    // Read on a helper thread so the main thread can apply a timeout even if
    // the terminal never responds (e.g. redirected stdin, uncooperative tty).
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    thread::spawn(move || {
        let mut buf = Vec::with_capacity(64);
        let mut stdin = std::io::stdin();
        let mut byte = [0u8; 1];
        while stdin.read(&mut byte).map(|n| n > 0).unwrap_or(false) {
            buf.push(byte[0]);
            if byte[0] == b'c' {
                break;
            }
            if buf.len() >= 64 {
                break;
            }
        }
        let _ = tx.send(buf);
    });

    let buf = rx
        .recv_timeout(Duration::from_millis(150))
        .unwrap_or_default();

    if raw_enabled {
        let _ = crossterm::terminal::disable_raw_mode();
    }

    let s = String::from_utf8_lossy(&buf);
    Ok(s.contains(";4;") || s.contains(";4c") || s.contains("[?4;") || s.contains(";4,"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_is_safe_on_non_tty() {
        let caps = probe();
        assert!(!caps.sixel);
    }

    #[test]
    fn default_caps_are_all_false() {
        let caps = TerminalCaps::default();
        assert!(!caps.sixel && !caps.kitty && !caps.truecolor);
        assert_eq!(caps.width, 0);
        assert_eq!(caps.height, 0);
    }
}
