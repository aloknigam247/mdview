// Crate-level: forbid unsafe except in the `daemonize` module, which needs raw
// OS calls (fork + setsid on Unix, CreateProcessW on Windows) that have no
// safe wrapper in std.
#![deny(unsafe_code)]

use anyhow::Result;
use clap::Parser;

mod builtins;
mod cli;
#[allow(unsafe_code)]
mod daemonize;
mod pipeline;
mod render;
mod server;

#[cfg(feature = "stubs")]
#[allow(non_snake_case)]
mod _stubs;

use crate::cli::{Cli, Mode};

fn main() -> Result<()> {
    let args = Cli::parse();

    match args.mode() {
        Mode::Terminal => pipeline::run_terminal(&args),
        Mode::Nvim => runtime()?.block_on(pipeline::run_nvim(&args)),
        Mode::Tauri => run_tauri(&args),
    }
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
