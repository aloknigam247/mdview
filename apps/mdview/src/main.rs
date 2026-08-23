// Crate-level: forbid unsafe except in the `daemonize` module, which needs raw
// OS calls (fork + setsid on Unix, CreateProcessW on Windows) that have no
// safe wrapper in std.
#![deny(unsafe_code)]

use anyhow::Result;
use clap::{CommandFactory, Parser};

mod builtins;
mod cli;
#[allow(unsafe_code)]
mod daemonize;
mod pipeline;
mod profile;
mod render;
mod render_terminal;
mod server;

use crate::cli::{Cli, Mode};

fn main() -> Result<()> {
    profile::init();
    profile::log("process_start");
    let args = Cli::parse();
    profile::log("args_parsed");

    // No FILE and no headless mode (nvim) — print help and exit 0.
    // Running `mdview` with nothing to render is almost always a typo.
    if args.file.is_none() && args.nvim_socket.is_none() {
        Cli::command().print_help()?;
        println!();
        return Ok(());
    }

    // Validate FILE up front so every surface (terminal / gui / nvim) reports a
    // consistent, user-friendly error with an exit code before doing any work.
    if let Some(file) = args.file.as_deref() {
        if let Err(code) = validate_file(file) {
            std::process::exit(code);
        }
    }

    // Hard-fail on any config parse / validation error before launching any UI.
    if let Err(code) = preflight_config() {
        std::process::exit(code);
    }

    match args.mode() {
        Mode::Serve => runtime()?.block_on(pipeline::run_serve(&args)),
        Mode::Terminal => pipeline::run_terminal(&args),
        Mode::Nvim => runtime()?.block_on(pipeline::run_nvim(&args)),
        Mode::Tauri => run_tauri(&args),
    }
}

/// Hard-fail config preflight: if the user's `config.toml` has any parse OR
/// validation error, print one `<path>:<line> \u{2014} <msg>` line per error to
/// stderr and return a non-zero exit code so the UI never starts.
fn preflight_config() -> std::result::Result<(), i32> {
    use std::io::Write;
    let loaded = mdview_config::Config::load_full();
    if loaded.errors.is_empty() {
        return Ok(());
    }
    let path = loaded
        .source_path
        .unwrap_or_else(|| std::path::PathBuf::from("config.toml"));
    let mut err = std::io::stderr();
    for e in &loaded.errors {
        let _ = writeln!(err, "{}", e.display_line(&path));
    }
    Err(78)
}

fn validate_file(file: &std::path::Path) -> std::result::Result<(), i32> {
    use std::io::{ErrorKind, Write};
    let mut err = std::io::stderr();
    match std::fs::metadata(file) {
        Ok(md) if md.is_dir() => {
            let _ = writeln!(
                err,
                "mdview: '{}' is a directory, not a file",
                file.display()
            );
            Err(2)
        }
        Ok(_) => {
            let bytes = match std::fs::read(file) {
                Ok(bytes) => bytes,
                Err(e) if e.kind() == ErrorKind::PermissionDenied => {
                    let _ = writeln!(err, "mdview: permission denied: {}", file.display());
                    return Err(13);
                }
                Err(e) => {
                    let _ = writeln!(err, "mdview: cannot read {}: {}", file.display(), e);
                    return Err(1);
                }
            };
            if is_binary_input(&bytes) {
                let _ = writeln!(
                    err,
                    "mdview: {} is not a text/markdown file",
                    file.display()
                );
                Err(2)
            } else {
                Ok(())
            }
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let _ = writeln!(err, "mdview: file not found: {}", file.display());
            Err(2)
        }
        Err(e) if e.kind() == ErrorKind::PermissionDenied => {
            let _ = writeln!(err, "mdview: permission denied: {}", file.display());
            Err(13)
        }
        Err(e) => {
            let _ = writeln!(err, "mdview: cannot access {}: {}", file.display(), e);
            Err(1)
        }
    }
}

fn is_binary_input(bytes: &[u8]) -> bool {
    bytes.contains(&0) || std::str::from_utf8(bytes).is_err()
}

fn runtime() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?)
}

fn run_tauri(args: &Cli) -> Result<()> {
    // MDVIEW_NO_DAEMONIZE lets integration tests drive the Tauri path in-process.
    let skip = std::env::var_os("MDVIEW_NO_DAEMONIZE").is_some();
    if !skip {
        if let daemonize::Spawned::Parent = daemonize::daemonize()? {
            return Ok(());
        }
    }
    runtime()?.block_on(pipeline::run_tauri_child(args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_input_detection_rejects_nul_and_invalid_utf8() {
        assert!(is_binary_input(b"MZ\0\0binary"));
        assert!(is_binary_input(&[0xff, 0xfe, 0xfd]));
        assert!(!is_binary_input(b"# Title\n\nNormal markdown text.\n"));
    }
}
