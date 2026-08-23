use crate::_stubs::Theme;
use crate::themes::{catppuccin_latte, catppuccin_mocha};

pub fn builtin_themes() -> Vec<&'static Theme> {
    vec![catppuccin_latte::get(), catppuccin_mocha::get()]
}

pub fn find(name: &str) -> Option<&'static Theme> {
    builtin_themes().into_iter().find(|t| t.name == name)
}
