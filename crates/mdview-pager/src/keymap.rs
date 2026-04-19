use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::Action;

#[derive(Debug, thiserror::Error)]
pub enum KeymapError {
    #[error("failed to read keymap: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid keymap: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("unknown action: {0}")]
    UnknownAction(String),
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct KeymapFile {
    #[serde(default)]
    pub bindings: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Keymap {
    pub bindings: BTreeMap<String, Action>,
}

impl Keymap {
    pub fn defaults() -> Self {
        let mut bindings: BTreeMap<String, Action> = BTreeMap::new();
        bindings.insert("G".into(), Action::Bottom);
        bindings.insert("PgDn".into(), Action::PageDown);
        bindings.insert("PgUp".into(), Action::PageUp);
        bindings.insert("?".into(), Action::Help);
        bindings.insert("C-c".into(), Action::Quit);
        bindings.insert("F".into(), Action::ToggleFollow);
        bindings.insert("Down".into(), Action::LineDown);
        bindings.insert("N".into(), Action::SearchPrev);
        bindings.insert("Up".into(), Action::LineUp);
        bindings.insert("b".into(), Action::PageUp);
        bindings.insert("gg".into(), Action::Top);
        bindings.insert("j".into(), Action::LineDown);
        bindings.insert("k".into(), Action::LineUp);
        bindings.insert("n".into(), Action::SearchNext);
        bindings.insert("q".into(), Action::Quit);
        bindings.insert("space".into(), Action::PageDown);
        bindings.insert("/".into(), Action::SearchStart);
        Keymap { bindings }
    }

    pub fn lookup(&self, key: &str) -> Option<Action> {
        self.bindings.get(key).copied()
    }

    pub fn from_toml_str(s: &str) -> Result<Self, KeymapError> {
        let file: KeymapFile = toml::from_str(s)?;
        let mut map = Keymap::defaults();
        for (k, v) in file.bindings {
            let action = Action::parse(&v).ok_or_else(|| KeymapError::UnknownAction(v.clone()))?;
            map.bindings.insert(k, action);
        }
        Ok(map)
    }

    pub fn load(path: Option<&Path>) -> Result<Self, KeymapError> {
        let resolved = match path {
            Some(p) => Some(p.to_path_buf()),
            None => default_keymap_path(),
        };
        if let Some(p) = resolved {
            if p.exists() {
                let s = std::fs::read_to_string(&p)?;
                return Self::from_toml_str(&s);
            }
        }
        Ok(Self::defaults())
    }
}

pub fn default_keymap_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("mdview").join("keymap.toml"))
}

impl Action {
    pub fn parse(name: &str) -> Option<Action> {
        match name {
            "bottom" => Some(Action::Bottom),
            "help" => Some(Action::Help),
            "line_down" => Some(Action::LineDown),
            "line_up" => Some(Action::LineUp),
            "page_down" => Some(Action::PageDown),
            "page_up" => Some(Action::PageUp),
            "quit" => Some(Action::Quit),
            "search_next" => Some(Action::SearchNext),
            "search_prev" => Some(Action::SearchPrev),
            "search_start" => Some(Action::SearchStart),
            "toggle_follow" => Some(Action::ToggleFollow),
            "top" => Some(Action::Top),
            _ => None,
        }
    }
}
