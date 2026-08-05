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

    /// Markdown file to render.
    #[arg(value_name = "FILE")]
    pub file: Option<PathBuf>,
}

impl Cli {
    pub fn mode(&self) -> Mode {
        if self.nvim_socket.is_some() {
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
}
