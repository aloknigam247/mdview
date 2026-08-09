use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "mdview",
    version,
    about = "Modern Markdown renderer (Tauri desktop + terminal + Neovim).",
    long_about = "By default, mdview launches a Tauri window with FILE loaded and live-reload \
enabled, then detaches and returns control to the shell. Use --terminal for the pager, or \
--nvim-socket to run as a headless Neovim bridge."
)]
pub struct Cli {
    /// Render to the terminal (stdout + pager) instead of Tauri.
    #[arg(short = 't', long)]
    pub terminal: bool,

    /// Watch FILE for changes (applies to either mode).
    #[arg(short = 'w', long)]
    pub watch: bool,

    /// Accept input from Neovim over pipe/socket PATH (headless mode).
    #[arg(long = "nvim-socket", value_name = "PATH")]
    pub nvim_socket: Option<PathBuf>,

    /// Terminal mode only: dump to stdout and exit (pipe-friendly).
    #[arg(long = "no-pager")]
    pub no_pager: bool,

    /// Serve-only mode: bind this fixed port instead of an ephemeral one.
    #[arg(long, value_name = "PORT", requires = "serve_only")]
    pub port: Option<u16>,

    /// Serve rendered markdown over HTTP and block; no window, no daemonize.
    #[arg(long = "serve-only", conflicts_with_all = ["nvim_socket", "terminal"])]
    pub serve_only: bool,

    /// Markdown file to render.
    #[arg(value_name = "FILE")]
    pub file: Option<PathBuf>,
}

impl Cli {
    pub fn mode(&self) -> Mode {
        if self.serve_only {
            Mode::Serve
        } else if self.nvim_socket.is_some() {
            Mode::Nvim
        } else if self.terminal {
            Mode::Terminal
        } else {
            Mode::Tauri
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Nvim,
    Serve,
    Tauri,
    Terminal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_file_only() {
        let cli = Cli::try_parse_from(["mdview", "README.md"]).unwrap();
        assert_eq!(cli.file.as_deref(), Some(std::path::Path::new("README.md")));
        assert!(!cli.terminal);
        assert!(!cli.watch);
        assert!(!cli.no_pager);
        assert!(cli.nvim_socket.is_none());
        assert_eq!(cli.mode(), Mode::Tauri);
    }

    #[test]
    fn parses_terminal_short_flag() {
        let cli = Cli::try_parse_from(["mdview", "-t", "x.md"]).unwrap();
        assert!(cli.terminal);
        assert_eq!(cli.mode(), Mode::Terminal);
    }

    #[test]
    fn parses_terminal_long_flag() {
        let cli = Cli::try_parse_from(["mdview", "--terminal", "x.md"]).unwrap();
        assert!(cli.terminal);
    }

    #[test]
    fn parses_watch_short_flag() {
        let cli = Cli::try_parse_from(["mdview", "-w", "x.md"]).unwrap();
        assert!(cli.watch);
    }

    #[test]
    fn parses_watch_long_flag() {
        let cli = Cli::try_parse_from(["mdview", "--watch", "x.md"]).unwrap();
        assert!(cli.watch);
    }

    #[test]
    fn parses_nvim_socket() {
        let cli = Cli::try_parse_from(["mdview", "--nvim-socket", "/tmp/nvim.sock"]).unwrap();
        assert_eq!(
            cli.nvim_socket.as_deref(),
            Some(std::path::Path::new("/tmp/nvim.sock"))
        );
        assert_eq!(cli.mode(), Mode::Nvim);
    }

    #[test]
    fn rejects_theme_flag() {
        assert!(Cli::try_parse_from(["mdview", "--theme", "dracula", "x.md"]).is_err());
    }

    #[test]
    fn parses_no_pager() {
        let cli = Cli::try_parse_from(["mdview", "--terminal", "--no-pager", "x.md"]).unwrap();
        assert!(cli.no_pager);
        assert!(cli.terminal);
    }

    #[test]
    fn parses_all_flags_together() {
        let cli = Cli::try_parse_from([
            "mdview",
            "-t",
            "-w",
            "--nvim-socket",
            "/tmp/n.sock",
            "--no-pager",
            "input.md",
        ])
        .unwrap();
        assert!(cli.terminal);
        assert!(cli.watch);
        assert!(cli.no_pager);
        assert_eq!(
            cli.nvim_socket.as_deref(),
            Some(std::path::Path::new("/tmp/n.sock"))
        );
        assert_eq!(cli.file.as_deref(), Some(std::path::Path::new("input.md")));
        // nvim-socket takes precedence over --terminal.
        assert_eq!(cli.mode(), Mode::Nvim);
    }

    #[test]
    fn file_is_optional() {
        let cli = Cli::try_parse_from(["mdview"]).unwrap();
        assert!(cli.file.is_none());
    }

    #[test]
    fn rejects_unknown_flag() {
        assert!(Cli::try_parse_from(["mdview", "--bogus"]).is_err());
    }

    #[test]
    fn parses_serve_only() {
        let cli = Cli::try_parse_from(["mdview", "--serve-only", "x.md"]).unwrap();
        assert!(cli.serve_only);
        assert!(cli.port.is_none());
        assert_eq!(cli.mode(), Mode::Serve);
    }

    #[test]
    fn parses_serve_only_with_port() {
        let cli =
            Cli::try_parse_from(["mdview", "--serve-only", "--port", "7681", "x.md"]).unwrap();
        assert_eq!(cli.port, Some(7681));
        assert_eq!(cli.mode(), Mode::Serve);
    }

    #[test]
    fn rejects_non_numeric_port() {
        assert!(Cli::try_parse_from(["mdview", "--serve-only", "--port", "abc", "x.md"]).is_err());
    }

    #[test]
    fn rejects_port_without_serve_only() {
        assert!(Cli::try_parse_from(["mdview", "--port", "7681", "x.md"]).is_err());
    }

    #[test]
    fn rejects_serve_only_with_nvim_socket() {
        assert!(
            Cli::try_parse_from(["mdview", "--serve-only", "--nvim-socket", "/tmp/n.sock"])
                .is_err()
        );
    }

    #[test]
    fn rejects_serve_only_with_terminal() {
        assert!(Cli::try_parse_from(["mdview", "--serve-only", "--terminal", "x.md"]).is_err());
    }
}
