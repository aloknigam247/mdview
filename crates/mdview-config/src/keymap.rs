use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};

use crate::action::Action;

#[derive(Debug, thiserror::Error)]
pub enum KeyParseError {
    #[error("empty key string")]
    Empty,
    #[error("trailing or leading '+' in key string: {0:?}")]
    StrayPlus(String),
    #[error("duplicate modifier {modifier} in {input:?}")]
    DuplicateModifier {
        modifier: &'static str,
        input: String,
    },
    #[error("unknown key token {0:?}")]
    UnknownToken(String),
    #[error("missing key after modifiers in {0:?}")]
    MissingKey(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    Char(char),
    Backspace,
    Delete,
    Down,
    End,
    Enter,
    Esc,
    F(u8),
    Home,
    Left,
    PageDown,
    PageUp,
    Right,
    Space,
    Tab,
    Up,
}

impl Key {
    fn parse(token: &str) -> Result<Self, KeyParseError> {
        if token.is_empty() {
            return Err(KeyParseError::Empty);
        }
        if token.len() == 1 {
            let c = token.chars().next().unwrap();
            if c.is_ascii_alphanumeric() {
                return Ok(Key::Char(c.to_ascii_uppercase()));
            }
            if c.is_ascii_graphic() {
                return Ok(Key::Char(c));
            }
        }
        let lower = token.to_ascii_lowercase();
        let key = match lower.as_str() {
            "backspace" => Key::Backspace,
            "delete" => Key::Delete,
            "down" => Key::Down,
            "end" => Key::End,
            "enter" => Key::Enter,
            "esc" | "escape" => Key::Esc,
            "home" => Key::Home,
            "left" => Key::Left,
            "pagedown" => Key::PageDown,
            "pageup" => Key::PageUp,
            "right" => Key::Right,
            "space" => Key::Space,
            "tab" => Key::Tab,
            "up" => Key::Up,
            other if other.starts_with('f') && other.len() > 1 => {
                let n: u8 = other[1..]
                    .parse()
                    .map_err(|_| KeyParseError::UnknownToken(token.to_string()))?;
                if !(1..=12).contains(&n) {
                    return Err(KeyParseError::UnknownToken(token.to_string()));
                }
                Key::F(n)
            }
            _ => return Err(KeyParseError::UnknownToken(token.to_string())),
        };
        Ok(key)
    }

    fn matches_code(self, code: KeyCode) -> bool {
        match (self, code) {
            (Key::Char(want), KeyCode::Char(got)) => want.eq_ignore_ascii_case(&got),
            (Key::Backspace, KeyCode::Backspace) => true,
            (Key::Delete, KeyCode::Delete) => true,
            (Key::Down, KeyCode::Down) => true,
            (Key::End, KeyCode::End) => true,
            (Key::Enter, KeyCode::Enter) => true,
            (Key::Esc, KeyCode::Esc) => true,
            (Key::F(want), KeyCode::F(got)) => want == got,
            (Key::Home, KeyCode::Home) => true,
            (Key::Left, KeyCode::Left) => true,
            (Key::PageDown, KeyCode::PageDown) => true,
            (Key::PageUp, KeyCode::PageUp) => true,
            (Key::Right, KeyCode::Right) => true,
            (Key::Space, KeyCode::Char(' ')) => true,
            (Key::Tab, KeyCode::Tab) => true,
            (Key::Up, KeyCode::Up) => true,
            _ => false,
        }
    }

    fn display(self) -> String {
        match self {
            Key::Char(c) => c.to_string(),
            Key::Backspace => "Backspace".into(),
            Key::Delete => "Delete".into(),
            Key::Down => "Down".into(),
            Key::End => "End".into(),
            Key::Enter => "Enter".into(),
            Key::Esc => "Esc".into(),
            Key::F(n) => format!("F{n}"),
            Key::Home => "Home".into(),
            Key::Left => "Left".into(),
            Key::PageDown => "PageDown".into(),
            Key::PageUp => "PageUp".into(),
            Key::Right => "Right".into(),
            Key::Space => "Space".into(),
            Key::Tab => "Tab".into(),
            Key::Up => "Up".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyBinding {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub super_: bool,
    pub key: Key,
}

impl KeyBinding {
    pub fn matches(&self, ev: &KeyEvent) -> bool {
        let m = ev.modifiers;
        self.ctrl == m.contains(KeyModifiers::CONTROL)
            && self.alt == m.contains(KeyModifiers::ALT)
            && self.super_ == m.contains(KeyModifiers::SUPER)
            && self.shift_matches(ev)
            && self.key.matches_code(ev.code)
    }

    // Crossterm reports SHIFT for printable upper-case chars on some platforms
    // but not others; treat an uppercase ASCII-letter binding without an
    // explicit Shift modifier as matching regardless of the SHIFT flag.
    fn shift_matches(&self, ev: &KeyEvent) -> bool {
        if matches!(self.key, Key::Char(c) if c.is_ascii_uppercase() && !self.shift) {
            return true;
        }
        self.shift == ev.modifiers.contains(KeyModifiers::SHIFT)
    }
}

impl fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ctrl {
            f.write_str("Ctrl+")?;
        }
        if self.shift {
            f.write_str("Shift+")?;
        }
        if self.alt {
            f.write_str("Alt+")?;
        }
        if self.super_ {
            f.write_str("Super+")?;
        }
        f.write_str(&self.key.display())
    }
}

impl FromStr for KeyBinding {
    type Err = KeyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(KeyParseError::Empty);
        }
        if s.starts_with('+') || s.ends_with('+') {
            return Err(KeyParseError::StrayPlus(s.to_string()));
        }
        let parts: Vec<&str> = s.split('+').collect();
        if parts.iter().any(|p| p.is_empty()) {
            return Err(KeyParseError::StrayPlus(s.to_string()));
        }
        let (mods, key_tok) = parts.split_at(parts.len() - 1);
        let key_tok = key_tok[0];

        let mut ctrl = false;
        let mut shift = false;
        let mut alt = false;
        let mut super_ = false;
        for m in mods {
            match m.to_ascii_lowercase().as_str() {
                "alt" | "meta" => {
                    if alt {
                        return Err(KeyParseError::DuplicateModifier {
                            modifier: "Alt",
                            input: s.to_string(),
                        });
                    }
                    alt = true;
                }
                "ctrl" | "control" => {
                    if ctrl {
                        return Err(KeyParseError::DuplicateModifier {
                            modifier: "Ctrl",
                            input: s.to_string(),
                        });
                    }
                    ctrl = true;
                }
                "shift" => {
                    if shift {
                        return Err(KeyParseError::DuplicateModifier {
                            modifier: "Shift",
                            input: s.to_string(),
                        });
                    }
                    shift = true;
                }
                "super" | "win" | "cmd" => {
                    if super_ {
                        return Err(KeyParseError::DuplicateModifier {
                            modifier: "Super",
                            input: s.to_string(),
                        });
                    }
                    super_ = true;
                }
                _ => return Err(KeyParseError::UnknownToken((*m).to_string())),
            }
        }
        if key_tok.is_empty() {
            return Err(KeyParseError::MissingKey(s.to_string()));
        }
        let key = Key::parse(key_tok)?;
        Ok(KeyBinding {
            ctrl,
            shift,
            alt,
            super_,
            key,
        })
    }
}

impl Serialize for KeyBinding {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for KeyBinding {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Keymap {
    pub bindings: BTreeMap<Action, KeyBinding>,
}

impl Keymap {
    /// Bindings are opt-in — no built-in defaults. An empty keymap is the
    /// expected starting state until the user populates their config.
    pub fn defaults() -> Self {
        Keymap {
            bindings: BTreeMap::new(),
        }
    }

    pub fn lookup(&self, ev: &KeyEvent) -> Option<Action> {
        for (action, binding) in &self.bindings {
            if binding.matches(ev) {
                return Some(*action);
            }
        }
        None
    }

    pub fn get(&self, action: Action) -> Option<&KeyBinding> {
        self.bindings.get(&action)
    }
}
