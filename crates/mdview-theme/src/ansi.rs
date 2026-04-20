use crate::_stubs::{StyleSpec, Theme};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnsiStyle {
    pub prefix: String,
    pub suffix: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorDepth {
    Truecolor,
    Palette256,
}

pub fn style_for(theme: &Theme, key: &str) -> AnsiStyle {
    style_for_depth(theme, key, ColorDepth::Truecolor)
}

pub fn style_for_depth(theme: &Theme, key: &str, depth: ColorDepth) -> AnsiStyle {
    let spec = match theme.styles.get(key) {
        Some(s) => s,
        None => return AnsiStyle::default(),
    };
    build(spec, depth)
}

fn build(spec: &StyleSpec, depth: ColorDepth) -> AnsiStyle {
    let mut codes: Vec<String> = Vec::new();
    if spec.bold {
        codes.push("1".into());
    }
    if spec.italic {
        codes.push("3".into());
    }
    if spec.underline {
        codes.push("4".into());
    }
    if let Some(hex) = spec.fg.as_deref() {
        if let Some((r, g, b)) = parse_hex(hex) {
            codes.push(fg_code(r, g, b, depth));
        }
    }
    if let Some(hex) = spec.bg.as_deref() {
        if let Some((r, g, b)) = parse_hex(hex) {
            codes.push(bg_code(r, g, b, depth));
        }
    }

    if codes.is_empty() {
        return AnsiStyle::default();
    }
    AnsiStyle {
        prefix: format!("\x1b[{}m", codes.join(";")),
        suffix: "\x1b[0m".into(),
    }
}

fn fg_code(r: u8, g: u8, b: u8, depth: ColorDepth) -> String {
    match depth {
        ColorDepth::Truecolor => format!("38;2;{r};{g};{b}"),
        ColorDepth::Palette256 => format!("38;5;{}", rgb_to_256(r, g, b)),
    }
}

fn bg_code(r: u8, g: u8, b: u8, depth: ColorDepth) -> String {
    match depth {
        ColorDepth::Truecolor => format!("48;2;{r};{g};{b}"),
        ColorDepth::Palette256 => format!("48;5;{}", rgb_to_256(r, g, b)),
    }
}

pub fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
    let s = hex.strip_prefix('#').unwrap_or(hex);
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some((r, g, b))
}

fn rgb_to_256(r: u8, g: u8, b: u8) -> u8 {
    if r == g && g == b {
        if r < 8 {
            return 16;
        }
        if r > 248 {
            return 231;
        }
        return 232 + ((r as u16 - 8) / 10) as u8;
    }
    let q = |c: u8| -> u8 {
        match c {
            0..=47 => 0,
            48..=114 => 1,
            115..=154 => 2,
            155..=194 => 3,
            195..=234 => 4,
            _ => 5,
        }
    };
    16 + 36 * q(r) + 6 * q(g) + q(b)
}
