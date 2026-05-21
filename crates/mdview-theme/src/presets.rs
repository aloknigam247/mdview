use crate::_stubs::Theme;
use crate::themes::{catppuccin_latte, catppuccin_mocha, dark, dracula, light, solarized};

pub fn builtin_themes() -> Vec<&'static Theme> {
    vec![
        catppuccin_latte::get(),
        catppuccin_mocha::get(),
        dark::get(),
        dracula::get(),
        light::get(),
        solarized::get(),
    ]
}

pub fn find(name: &str) -> Option<&'static Theme> {
    builtin_themes().into_iter().find(|t| t.name == name)
}
