#![forbid(unsafe_code)]

//! `mdview-config` — user-facing TOML configuration.
//!
//! Single source of truth for keybindings on every output surface. The file
//! lives at `$XDG_CONFIG_HOME/mdview/config.toml` (with the standard
//! `$HOME/.config/mdview/config.toml` fallback). Bindings are opt-in: an
//! action that isn't listed in the user's config has no binding. On first run
//! a fully commented template is written to disk so users can discover the
//! available actions; malformed config never crashes.

pub mod action;
pub mod error;
pub mod keymap;
pub mod os_theme;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub use action::Action;
pub use error::{ConfigError, ConfigErrorSource};
pub use keymap::{KeyBinding, Keymap};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Config {
    pub keymap: Keymap,
    pub toc: TocConfig,
    pub codemap: CodemapConfig,
    pub theme: ThemeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeConfig {
    pub mode: ThemeMode,
    pub light: String,
    pub dark: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        ThemeConfig {
            mode: ThemeMode::Auto,
            light: default_theme_light(),
            dark: default_theme_dark(),
        }
    }
}

fn default_theme_light() -> String {
    "catppuccin-latte".into()
}

fn default_theme_dark() -> String {
    "catppuccin-mocha".into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeMode {
    Auto,
    Dark,
    Light,
}

impl ThemeMode {
    /// Resolve to a concrete light/dark choice at launch time. `Auto` queries
    /// the OS; on detection failure or non-Windows platforms it falls back to
    /// dark (returns `false`).
    pub fn resolve_is_light(self) -> bool {
        match self {
            ThemeMode::Light => true,
            ThemeMode::Dark => false,
            ThemeMode::Auto => crate::os_theme::os_prefers_light().unwrap_or(false),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TocConfig {
    pub position: TocPosition,
    pub depth: u8,
}

impl Default for TocConfig {
    fn default() -> Self {
        TocConfig {
            position: TocPosition::FloatingRight,
            depth: 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TocPosition {
    FixedLeft,
    FixedRight,
    FloatingCenter,
    FloatingLeft,
    FloatingRight,
    Inline,
}

impl TocPosition {
    pub fn as_kebab(self) -> &'static str {
        match self {
            TocPosition::FixedLeft => "fixed-left",
            TocPosition::FixedRight => "fixed-right",
            TocPosition::FloatingCenter => "floating-center",
            TocPosition::FloatingLeft => "floating-left",
            TocPosition::FloatingRight => "floating-right",
            TocPosition::Inline => "inline",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodemapConfig {
    pub enabled: bool,
}

impl Default for CodemapConfig {
    fn default() -> Self {
        CodemapConfig { enabled: true }
    }
}

#[derive(Debug, Clone)]
pub struct LoadResult {
    pub config: Config,
    pub errors: Vec<ConfigError>,
}

impl Config {
    pub fn defaults() -> Self {
        Config {
            keymap: Keymap::defaults(),
            toc: TocConfig::default(),
            codemap: CodemapConfig::default(),
            theme: ThemeConfig::default(),
        }
    }

    /// Parse from a TOML string. Unknown action keys and unparseable bindings
    /// are dropped with a `tracing::warn` log; the function never errors on
    /// user input that's merely wrong.
    pub fn from_toml_str(s: &str) -> Self {
        Self::from_toml_str_full(s).config
    }

    /// Parse from a TOML string, collecting every key/value error encountered.
    /// Always returns a valid `Config` (falling back to defaults per field).
    pub fn from_toml_str_full(s: &str) -> LoadResult {
        #[derive(Deserialize, Default)]
        struct Raw {
            #[serde(default)]
            keymap: BTreeMap<String, toml::Value>,
            #[serde(default)]
            toc: Option<toml::Value>,
            #[serde(default)]
            codemap: Option<toml::Value>,
            #[serde(default)]
            theme: Option<toml::Value>,
        }

        let mut errors: Vec<ConfigError> = Vec::new();

        let raw: Raw = match toml::from_str(s) {
            Ok(r) => r,
            Err(e) => {
                let line_col = e.span().map(|sp| line_col_at(s, sp.start));
                let msg = format!("invalid TOML; {}", clean_toml_message(&e.to_string()));
                errors.push(ConfigError::toml(line_col, msg));
                return LoadResult {
                    config: Self::defaults(),
                    errors,
                };
            }
        };

        let mut keymap = Keymap::defaults();
        for (action_name, value) in raw.keymap {
            let action: Action = match action_name.parse() {
                Ok(a) => a,
                Err(_) => {
                    errors.push(ConfigError::keymap(
                        &action_name,
                        value.as_str().map(|s| s.to_string()),
                        format!(
                            "unknown action {:?}; expected one of {}",
                            action_name,
                            list_actions()
                        ),
                    ));
                    continue;
                }
            };
            let key_str = match value.as_str() {
                Some(s) => s,
                None => {
                    errors.push(ConfigError::keymap(
                        &action_name,
                        Some(value.to_string()),
                        "binding is not a string; expected \"Ctrl+Shift+Alt+Super+<Key>\""
                            .to_string(),
                    ));
                    continue;
                }
            };
            if key_str.is_empty() {
                errors.push(ConfigError::keymap(
                    &action_name,
                    Some(String::new()),
                    "empty binding; expected \"Ctrl+Shift+Alt+Super+<Key>\"".to_string(),
                ));
                continue;
            }
            match key_str.parse::<KeyBinding>() {
                Ok(b) => {
                    keymap.bindings.insert(action, b);
                }
                Err(e) => {
                    errors.push(ConfigError::keymap(
                        &action_name,
                        Some(key_str.to_string()),
                        binding_error_message(&e),
                    ));
                }
            }
        }

        let mut toc = TocConfig::default();
        match raw.toc {
            None => {}
            Some(toml::Value::Table(t)) => {
                if let Some(v) = t.get("position") {
                    match v.clone().try_into::<TocPosition>() {
                        Ok(p) => toc.position = p,
                        Err(_) => {
                            errors.push(ConfigError::toc(
                                "position",
                                Some(v.to_string()),
                                "unknown position; expected one of fixed-left, fixed-right, floating-center, floating-left, floating-right, inline".to_string(),
                            ));
                        }
                    }
                }
                if let Some(v) = t.get("depth") {
                    match v.as_integer() {
                        Some(i) if (1..=6).contains(&i) => toc.depth = i as u8,
                        Some(i) => {
                            errors.push(ConfigError::toc(
                                "depth",
                                Some(i.to_string()),
                                "out of range; expected 1..=6".to_string(),
                            ));
                            toc.depth = (i.clamp(1, 6)) as u8;
                        }
                        None => {
                            errors.push(ConfigError::toc(
                                "depth",
                                Some(v.to_string()),
                                "not an integer; expected 1..=6".to_string(),
                            ));
                        }
                    }
                }
            }
            Some(other) => {
                errors.push(ConfigError::toc(
                    "",
                    Some(other.to_string()),
                    "expected a table".to_string(),
                ));
            }
        }

        let mut codemap = CodemapConfig::default();
        match raw.codemap {
            None => {}
            Some(toml::Value::Table(t)) => {
                if let Some(v) = t.get("enabled") {
                    match v.as_bool() {
                        Some(b) => codemap.enabled = b,
                        None => {
                            errors.push(ConfigError::codemap(
                                "enabled",
                                Some(v.to_string()),
                                "not a bool; expected true or false".to_string(),
                            ));
                        }
                    }
                }
            }
            Some(other) => {
                errors.push(ConfigError::codemap(
                    "",
                    Some(other.to_string()),
                    "expected a table".to_string(),
                ));
            }
        }

        let mut theme = ThemeConfig::default();
        match raw.theme {
            None => {}
            Some(toml::Value::Table(t)) => {
                if let Some(v) = t.get("mode") {
                    match v.clone().try_into::<ThemeMode>() {
                        Ok(m) => theme.mode = m,
                        Err(_) => {
                            errors.push(ConfigError::theme(
                                "mode",
                                Some(v.to_string()),
                                "unknown mode; expected one of auto, dark, light".to_string(),
                            ));
                        }
                    }
                }
                if let Some(v) = t.get("light") {
                    match v.as_str() {
                        Some(s) if !s.is_empty() => theme.light = s.to_string(),
                        _ => {
                            errors.push(ConfigError::theme(
                                "light",
                                Some(v.to_string()),
                                "expected a non-empty theme name string".to_string(),
                            ));
                        }
                    }
                }
                if let Some(v) = t.get("dark") {
                    match v.as_str() {
                        Some(s) if !s.is_empty() => theme.dark = s.to_string(),
                        _ => {
                            errors.push(ConfigError::theme(
                                "dark",
                                Some(v.to_string()),
                                "expected a non-empty theme name string".to_string(),
                            ));
                        }
                    }
                }
            }
            Some(other) => {
                errors.push(ConfigError::theme(
                    "",
                    Some(other.to_string()),
                    "expected a table".to_string(),
                ));
            }
        }

        LoadResult {
            config: Config {
                keymap,
                toc,
                codemap,
                theme,
            },
            errors,
        }
    }

    /// Resolve the config-file path using XDG rules. Returns `None` only if no
    /// usable home / XDG dir can be derived from the environment.
    pub fn config_path() -> Option<PathBuf> {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            if !xdg.is_empty() {
                return Some(PathBuf::from(xdg).join("mdview").join("config.toml"));
            }
        }
        let home = std::env::var("HOME")
            .ok()
            .or_else(|| std::env::var("USERPROFILE").ok())?;
        if home.is_empty() {
            return None;
        }
        Some(
            PathBuf::from(home)
                .join(".config")
                .join("mdview")
                .join("config.toml"),
        )
    }

    /// Backwards-compatible loader; drops any collected errors after surfacing
    /// them via `tracing::warn!` + stderr.
    pub fn load() -> Self {
        Self::load_full().config
    }

    /// Loader that returns both the resolved config and the list of errors
    /// encountered while parsing the user's TOML.
    pub fn load_full() -> LoadResult {
        match Self::config_path() {
            Some(p) => Self::load_from(&p),
            None => {
                let err = ConfigError {
                    source: ConfigErrorSource::Toml,
                    key: None,
                    raw_value: None,
                    message:
                        "cannot resolve config path (XDG_CONFIG_HOME / HOME unset); using defaults"
                            .into(),
                };
                report(std::slice::from_ref(&err));
                LoadResult {
                    config: Self::defaults(),
                    errors: vec![err],
                }
            }
        }
    }

    pub fn load_from(path: &Path) -> LoadResult {
        if path.exists() {
            match std::fs::read_to_string(path) {
                Ok(s) => {
                    let result = Self::from_toml_str_full(&s);
                    report(&result.errors);
                    return result;
                }
                Err(e) => {
                    let err = ConfigError {
                        source: ConfigErrorSource::Toml,
                        key: Some(path.display().to_string()),
                        raw_value: None,
                        message: format!("cannot read file: {e}; using defaults"),
                    };
                    report(std::slice::from_ref(&err));
                    return LoadResult {
                        config: Self::defaults(),
                        errors: vec![err],
                    };
                }
            }
        }
        if let Err(e) = write_default(path) {
            tracing::warn!(
                "mdview-config: cannot write default config to {}: {e}",
                path.display()
            );
        }
        LoadResult {
            config: Self::defaults(),
            errors: Vec::new(),
        }
    }
}

fn report(errors: &[ConfigError]) {
    for err in errors {
        tracing::warn!("mdview-config: {err}");
        eprintln!("mdview-config-error: {err}");
    }
}

fn list_actions() -> String {
    let mut names: Vec<&str> = Action::ALL.iter().map(|a| a.as_str()).collect();
    names.sort_unstable();
    names.join(", ")
}

fn line_col_at(src: &str, byte_offset: usize) -> (usize, usize) {
    let end = byte_offset.min(src.len());
    let prefix = &src[..end];
    let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
    let last_nl = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let col = src[last_nl..end].chars().count() + 1;
    (line, col)
}

fn clean_toml_message(s: &str) -> String {
    // `toml`'s `Display` includes a multi-line caret diagram; reduce to the
    // first non-empty line so the error stays single-line.
    s.lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or(s)
        .trim()
        .to_string()
}

fn binding_error_message(e: &keymap::KeyParseError) -> String {
    use keymap::KeyParseError;
    match e {
        KeyParseError::Empty => "empty binding; expected \"Ctrl+Shift+Alt+Super+<Key>\"".into(),
        KeyParseError::StrayPlus(_) => {
            "stray '+' in binding; expected \"Ctrl+Shift+Alt+Super+<Key>\"".into()
        }
        KeyParseError::DuplicateModifier { modifier, .. } => {
            format!("duplicate modifier {modifier}; each modifier may appear at most once")
        }
        KeyParseError::UnknownToken(tok) => {
            if looks_like_modifier(tok) {
                format!("unknown modifier {tok:?}; expected one of Ctrl, Shift, Alt, Super")
            } else {
                format!(
                    "unknown token {tok:?}; expected A-Z, 0-9, F1..F12, or a named key (Down, Up, Left, Right, Enter, Tab, Esc, Space, Backspace, Delete, Home, End, PageUp, PageDown)"
                )
            }
        }
        KeyParseError::MissingKey(_) => {
            "missing key after modifiers; expected \"Ctrl+Shift+Alt+Super+<Key>\"".into()
        }
    }
}

fn looks_like_modifier(tok: &str) -> bool {
    let l = tok.to_ascii_lowercase();
    matches!(
        l.as_str(),
        "ctr" | "ctrr" | "ctrll" | "control" | "ctl" | "shft" | "shit" | "shf" | "alti" | "alts"
    ) || (l.len() <= 6
        && (l.starts_with("ctr")
            || l.starts_with("sh")
            || l.starts_with("al")
            || l.starts_with("su")))
}

/// The default template written on first run. All bindings are commented out
/// so that nothing is active until the user opts in.
pub const DEFAULT_CONFIG_TOML: &str = "# mdview configuration\n\
# Reload mdview after editing.\n\
\n\
[toc]\n\
# position: floating-right (default), floating-center, floating-left, fixed-right, fixed-left, inline\n\
# depth:    1..6, default 3\n\
# position = \"floating-right\"\n\
# depth = 3\n\
\n\
[codemap]\n\
# enabled: show the right-edge minimap on launch. Default: true.\n\
# enabled = true\n\
\n\
[theme]\n\
# mode:  auto (default; follows OS), light, or dark\n\
# light: theme preset used for the light slot. Default: catppuccin-latte.\n\
# dark:  theme preset used for the dark slot. Default: catppuccin-mocha.\n\
# mode  = \"auto\"\n\
# light = \"catppuccin-latte\"\n\
# dark  = \"catppuccin-mocha\"\n\
\n\
[keymap]\n\
# Bindings are opt-in. Uncomment a line to enable an action.\n\
# Format: \"Ctrl+Shift+Alt+Super+<Key>\"\n\
# Modifiers (any order, optional): Ctrl, Shift, Alt, Super\n\
# Keys: A-Z, 0-9, or named: Down, Up, Left, Right, Enter, Tab, Esc, Space,\n\
#       Backspace, Delete, Home, End, PageUp, PageDown, F1..F12\n\
#\n\
# Available actions:\n\
#   quit            \u{2014} close the mdview window / exit the pager\n\
#   toggle-codemap  \u{2014} show / hide the right-edge minimap\n\
#   toggle-theme    \u{2014} flip between light and dark themes\n\
#   toggle-toc      \u{2014} show / hide the floating table of contents\n\
#\n\
# quit = \"Ctrl+Q\"\n";

fn write_default(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, DEFAULT_CONFIG_TOML)
}
