use crate::_stubs::StyleSpec;

pub const SGR_RESET: &str = "\x1b[0m";

pub fn sgr_open(style: &StyleSpec, truecolor: bool) -> String {
    let mut codes: Vec<String> = Vec::new();
    if style.bold {
        codes.push("1".into());
    }
    if style.italic {
        codes.push("3".into());
    }
    if style.underline {
        codes.push("4".into());
    }
    if let Some(fg) = &style.fg {
        if let Some((r, g, b)) = parse_hex(fg) {
            if truecolor {
                codes.push(format!("38;2;{};{};{}", r, g, b));
            } else {
                codes.push(format!("38;5;{}", rgb_to_256(r, g, b)));
            }
        }
    }
    if let Some(bg) = &style.bg {
        if let Some((r, g, b)) = parse_hex(bg) {
            if truecolor {
                codes.push(format!("48;2;{};{};{}", r, g, b));
            } else {
                codes.push(format!("48;5;{}", rgb_to_256(r, g, b)));
            }
        }
    }
    if codes.is_empty() {
        String::new()
    } else {
        format!("\x1b[{}m", codes.join(";"))
    }
}

pub fn parse_hex(s: &str) -> Option<(u8, u8, u8)> {
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some((r, g, b))
}

pub fn rgb_to_256(r: u8, g: u8, b: u8) -> u8 {
    let to_cube = |v: u8| -> u8 {
        if v < 48 {
            0
        } else if v < 115 {
            1
        } else {
            ((v as u16 - 35) / 40) as u8
        }
    };
    let ri = to_cube(r);
    let gi = to_cube(g);
    let bi = to_cube(b);
    16 + 36 * ri + 6 * gi + bi
}
