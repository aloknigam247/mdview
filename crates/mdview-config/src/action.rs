use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Action {
    Quit,
    ToggleBionic,
    ToggleCodemap,
    ToggleTheme,
    ToggleToc,
}

impl Action {
    pub const ALL: &'static [Action] = &[
        Action::Quit,
        Action::ToggleBionic,
        Action::ToggleCodemap,
        Action::ToggleTheme,
        Action::ToggleToc,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Action::Quit => "quit",
            Action::ToggleBionic => "toggle-bionic",
            Action::ToggleCodemap => "toggle-codemap",
            Action::ToggleTheme => "toggle-theme",
            Action::ToggleToc => "toggle-toc",
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown action: {0}")]
pub struct UnknownAction(pub String);

impl FromStr for Action {
    type Err = UnknownAction;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "quit" => Ok(Action::Quit),
            "toggle-bionic" => Ok(Action::ToggleBionic),
            "toggle-codemap" => Ok(Action::ToggleCodemap),
            "toggle-theme" => Ok(Action::ToggleTheme),
            "toggle-toc" => Ok(Action::ToggleToc),
            other => Err(UnknownAction(other.to_string())),
        }
    }
}
