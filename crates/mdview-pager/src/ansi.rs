use ansitok::{parse_ansi, parse_ansi_sgr, AnsiColor, ElementKind, Output, VisualAttribute};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::PagerError;

pub(crate) fn lines(input: &str) -> Result<Vec<Line<'static>>, PagerError> {
    let mut lines = Vec::new();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut style = Style::default();

    for token in parse_ansi(input) {
        let text = &input[token.start()..token.end()];
        match token.kind() {
            ElementKind::Sgr => style = sgr_style(style, text)?,
            ElementKind::Text => {
                for part in text.split_inclusive('\n') {
                    let content = part.strip_suffix('\n').unwrap_or(part);
                    if !content.is_empty() {
                        if let Some(last) = spans.last_mut().filter(|span| span.style == style) {
                            last.content.to_mut().push_str(content);
                        } else {
                            spans.push(Span::styled(content.to_owned(), style));
                        }
                    }
                    if part.ends_with('\n') {
                        lines.push(Line::from(std::mem::take(&mut spans)));
                    }
                }
            }
            _ => {}
        }
    }
    if !spans.is_empty() {
        lines.push(Line::from(spans));
    }
    Ok(lines)
}

fn ansi_color(color: AnsiColor, sequence: &str) -> Result<Color, PagerError> {
    let color = match color {
        AnsiColor::Bit24 { b, g, r } => Color::Rgb(r, g, b),
        AnsiColor::Bit4(code) => match code {
            30 | 40 => Color::Black,
            31 | 41 => Color::Red,
            32 | 42 => Color::Green,
            33 | 43 => Color::Yellow,
            34 | 44 => Color::Blue,
            35 | 45 => Color::Magenta,
            36 | 46 => Color::Cyan,
            37 | 47 => Color::Gray,
            39 | 49 => Color::Reset,
            90 | 100 => Color::DarkGray,
            91 | 101 => Color::LightRed,
            92 | 102 => Color::LightGreen,
            93 | 103 => Color::LightYellow,
            94 | 104 => Color::LightBlue,
            95 | 105 => Color::LightMagenta,
            96 | 106 => Color::LightCyan,
            97 | 107 => Color::White,
            _ => return Err(PagerError::Ansi(sequence.to_owned())),
        },
        AnsiColor::Bit8(index) => Color::Indexed(index),
    };
    Ok(color)
}

fn sgr_style(mut style: Style, sequence: &str) -> Result<Style, PagerError> {
    let parameters = sequence
        .strip_prefix("\x1b[")
        .and_then(|text| text.strip_suffix('m'))
        .ok_or_else(|| PagerError::Ansi(sequence.to_owned()))?;
    if parameters.is_empty() {
        return Ok(Style::reset());
    }
    // Reject out-of-range parameters before the parser substitutes defaults.
    if parameters
        .split(';')
        .any(|parameter| !parameter.is_empty() && parameter.parse::<u8>().is_err())
    {
        return Err(PagerError::Ansi(sequence.to_owned()));
    }
    for attribute in parse_ansi_sgr(sequence) {
        let Output::Escape(attribute) = attribute else {
            return Err(PagerError::Ansi(sequence.to_owned()));
        };
        style = match attribute {
            VisualAttribute::BgColor(color) => style.bg(ansi_color(color, sequence)?),
            VisualAttribute::Bold => style.add_modifier(Modifier::BOLD),
            VisualAttribute::Crossedout => style.add_modifier(Modifier::CROSSED_OUT),
            VisualAttribute::DoubleUnderline | VisualAttribute::Underline => {
                style.add_modifier(Modifier::UNDERLINED)
            }
            VisualAttribute::Faint => style.add_modifier(Modifier::DIM),
            VisualAttribute::FgColor(color) => style.fg(ansi_color(color, sequence)?),
            VisualAttribute::Hide => style.add_modifier(Modifier::HIDDEN),
            VisualAttribute::Inverse => style.add_modifier(Modifier::REVERSED),
            VisualAttribute::Italic => style.add_modifier(Modifier::ITALIC),
            VisualAttribute::RapidBlink => style.add_modifier(Modifier::RAPID_BLINK),
            VisualAttribute::Reset(0) => Style::reset(),
            VisualAttribute::Reset(22) => style.remove_modifier(Modifier::BOLD | Modifier::DIM),
            VisualAttribute::Reset(23) => style.remove_modifier(Modifier::ITALIC),
            VisualAttribute::Reset(24) => style.remove_modifier(Modifier::UNDERLINED),
            VisualAttribute::Reset(25) => {
                style.remove_modifier(Modifier::SLOW_BLINK | Modifier::RAPID_BLINK)
            }
            VisualAttribute::Reset(27) => style.remove_modifier(Modifier::REVERSED),
            VisualAttribute::Reset(28) => style.remove_modifier(Modifier::HIDDEN),
            VisualAttribute::Reset(29) => style.remove_modifier(Modifier::CROSSED_OUT),
            VisualAttribute::Reset(39) => style.fg(Color::Reset),
            VisualAttribute::Reset(49) => style.bg(Color::Reset),
            VisualAttribute::SlowBlink => style.add_modifier(Modifier::SLOW_BLINK),
            // Preserve supported state for SGR attributes ratatui cannot display.
            _ => style,
        };
    }
    Ok(style)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn styles(input: &str) -> Vec<Style> {
        let lines = lines(input).unwrap();
        lines
            .iter()
            .flat_map(|line| &line.spans)
            .flat_map(|span| span.content.chars().map(move |_| span.style))
            .collect()
    }

    #[test]
    fn ansi_color_modes() {
        let basic = [
            (30, 40, Color::Black),
            (31, 41, Color::Red),
            (32, 42, Color::Green),
            (33, 43, Color::Yellow),
            (34, 44, Color::Blue),
            (35, 45, Color::Magenta),
            (36, 46, Color::Cyan),
            (37, 47, Color::Gray),
            (39, 49, Color::Reset),
            (90, 100, Color::DarkGray),
            (91, 101, Color::LightRed),
            (92, 102, Color::LightGreen),
            (93, 103, Color::LightYellow),
            (94, 104, Color::LightBlue),
            (95, 105, Color::LightMagenta),
            (96, 106, Color::LightCyan),
            (97, 107, Color::White),
        ];
        for (fg, bg, color) in basic {
            assert_eq!(
                styles(&format!("\x1b[{fg};{bg}mX")),
                [Style::default().fg(color).bg(color)]
            );
        }
        assert_eq!(
            styles("\x1b[38;5;0;48;5;255mX"),
            [Style::default()
                .fg(Color::Indexed(0))
                .bg(Color::Indexed(255))]
        );
        assert_eq!(
            styles("\x1b[38;2;0;127;255;48;2;240;242;255mX"),
            [Style::default()
                .fg(Color::Rgb(0, 127, 255))
                .bg(Color::Rgb(240, 242, 255))]
        );
    }

    #[test]
    fn ansi_controls_keep_plaintext() {
        let cases = [
            ("a\x1b[2Jb\x1b[3;4Hc", "abc"),
            ("a\x1b]0;title\x07b", "ab"),
            ("a\x1b]0;title\x1b\\b", "ab"),
            (
                "\x1b]8;;https://e.x\x1b\\link\x1b]8;;\x1b\\ tail",
                "link tail",
            ),
        ];
        for (input, expected) in cases {
            let lines = lines(input).unwrap();
            assert_eq!(lines.len(), 1, "{input:?}");
            assert_eq!(lines[0].to_string(), expected, "{input:?}");
            assert_eq!(crate::search::strip_ansi(input), expected, "{input:?}");
        }
    }

    #[test]
    fn ansi_empty_and_newline_counts() {
        let cases = [
            ("", vec![]),
            ("\n", vec![""]),
            ("\n\n", vec!["", ""]),
            ("\x1b[0m", vec![]),
            ("a", vec!["a"]),
            ("a\n", vec!["a"]),
            ("a\n\n", vec!["a", ""]),
            ("a\n\x1b[0m", vec!["a"]),
            ("a\n\x1b[0m\n", vec!["a", ""]),
        ];
        for (input, expected) in cases {
            let lines = lines(input).unwrap();
            let actual: Vec<_> = lines.iter().map(ToString::to_string).collect();
            assert_eq!(actual, expected, "{input:?}");
        }
    }

    #[test]
    fn ansi_invalid_sequences_are_reported() {
        for input in [
            "\x1b[200mX",
            "\x1b[38;2;256;0;0mX",
            "\x1b[38;5;256mX",
            "\x1b[999mX",
        ] {
            assert!(
                matches!(lines(input), Err(PagerError::Ansi(_))),
                "{input:?}"
            );
        }
        assert!(matches!(
            ansi_color(AnsiColor::Bit4(1), "invalid"),
            Err(PagerError::Ansi(_))
        ));
        assert!(matches!(
            sgr_style(Style::default(), "invalid"),
            Err(PagerError::Ansi(_))
        ));
    }

    #[test]
    fn ansi_resets_preserve_unrelated_colors_and_clear_all() {
        let actual = styles("\x1b[1;31;44mA\x1b[39mB\x1b[49mC\x1b[mD\x1b[31mE\x1b[0mF");
        assert_eq!(actual.len(), 6);
        assert_eq!(
            actual[0],
            Style::default()
                .fg(Color::Red)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(actual[1], actual[0].fg(Color::Reset));
        assert_eq!(actual[2], actual[1].bg(Color::Reset));
        assert_eq!(actual[3], Style::reset());
        assert_eq!(actual[4], Style::reset().fg(Color::Red));
        assert_eq!(actual[5], Style::reset());
    }

    #[test]
    fn ansi_selective_resets() {
        let cases = [
            (22, Modifier::BOLD | Modifier::DIM),
            (23, Modifier::ITALIC),
            (24, Modifier::UNDERLINED),
            (25, Modifier::SLOW_BLINK | Modifier::RAPID_BLINK),
            (27, Modifier::REVERSED),
            (28, Modifier::HIDDEN),
            (29, Modifier::CROSSED_OUT),
        ];
        let initial = Style::default()
            .fg(Color::Rgb(10, 20, 30))
            .bg(Color::Indexed(17))
            .add_modifier(
                Modifier::BOLD
                    | Modifier::CROSSED_OUT
                    | Modifier::DIM
                    | Modifier::HIDDEN
                    | Modifier::ITALIC
                    | Modifier::RAPID_BLINK
                    | Modifier::REVERSED
                    | Modifier::SLOW_BLINK
                    | Modifier::UNDERLINED,
            );
        for (code, removed) in cases {
            let input = format!("\x1b[1;2;3;4;5;6;7;8;9;38;2;10;20;30;48;5;17mA\x1b[{code}mB");
            assert_eq!(
                styles(&input),
                [initial, initial.remove_modifier(removed)],
                "{code}"
            );
        }
        let actual = styles("\x1b[7mA\x1b[27mB");
        assert!(actual[0].add_modifier.contains(Modifier::REVERSED));
        assert!(!actual[1].add_modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn ansi_styles_cross_lines_and_keep_unicode() {
        let input = "\x1b[31m中e\u{301}\n\u{2764}\u{fe0f}\x1b[0m\nplain";
        let lines = lines(input).unwrap();
        assert_eq!(
            lines.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["中e\u{301}", "\u{2764}\u{fe0f}", "plain"]
        );
        assert_eq!(lines[0].spans[0].style.fg, Some(Color::Red));
        assert_eq!(lines[1].spans[0].style.fg, Some(Color::Red));
        assert_eq!(lines[2].spans[0].style, Style::reset());
    }

    #[test]
    fn ansi_unrepresentable_attributes_keep_supported_styles() {
        assert_eq!(
            styles("\x1b[31;10;20;26;51;52;53;60;61;62;63;64;73;74mX"),
            [Style::default().fg(Color::Red)]
        );
        assert_eq!(
            styles("\x1b[21mX"),
            [Style::default().add_modifier(Modifier::UNDERLINED)]
        );
        let lines = lines("a\x1b[10mb").unwrap();
        assert_eq!(lines[0].spans.len(), 1);
        assert_eq!(lines[0].spans[0].content, "ab");
    }
}
