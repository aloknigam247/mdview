use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigErrorSource {
    Codemap,
    Keymap,
    Theme,
    Toc,
    Toml,
}

impl ConfigErrorSource {
    fn as_str(self) -> &'static str {
        match self {
            ConfigErrorSource::Codemap => "codemap",
            ConfigErrorSource::Keymap => "keymap",
            ConfigErrorSource::Theme => "theme",
            ConfigErrorSource::Toc => "toc",
            ConfigErrorSource::Toml => "toml",
        }
    }
}

impl fmt::Display for ConfigErrorSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct ConfigError {
    pub source: ConfigErrorSource,
    pub key: Option<String>,
    pub raw_value: Option<String>,
    pub message: String,
}

impl ConfigError {
    pub fn toml(line_col: Option<(usize, usize)>, message: impl Into<String>) -> Self {
        let key = line_col.map(|(l, c)| format!("config.toml line {l}:{c}"));
        ConfigError {
            source: ConfigErrorSource::Toml,
            key,
            raw_value: None,
            message: message.into(),
        }
    }

    pub fn keymap(
        action: impl Into<String>,
        raw_value: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        ConfigError {
            source: ConfigErrorSource::Keymap,
            key: Some(format!("keymap[{}]", action.into())),
            raw_value,
            message: message.into(),
        }
    }

    pub fn toc(field: &str, raw_value: Option<String>, message: impl Into<String>) -> Self {
        ConfigError {
            source: ConfigErrorSource::Toc,
            key: Some(format!("[toc] {field}")),
            raw_value,
            message: message.into(),
        }
    }

    pub fn codemap(field: &str, raw_value: Option<String>, message: impl Into<String>) -> Self {
        ConfigError {
            source: ConfigErrorSource::Codemap,
            key: Some(format!("[codemap] {field}")),
            raw_value,
            message: message.into(),
        }
    }

    pub fn theme(field: &str, raw_value: Option<String>, message: impl Into<String>) -> Self {
        ConfigError {
            source: ConfigErrorSource::Theme,
            key: Some(format!("[theme] {field}")),
            raw_value,
            message: message.into(),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.key, &self.raw_value) {
            (Some(k), Some(v)) => write!(f, "{k} = {v:?}: {}", self.message),
            (Some(k), None) => write!(f, "{k}: {}", self.message),
            (None, Some(v)) => write!(f, "{v:?}: {}", self.message),
            (None, None) => f.write_str(&self.message),
        }
    }
}
