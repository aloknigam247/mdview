use unicode_width::UnicodeWidthChar;

pub fn wrap_ansi(input: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines: Vec<String> = Vec::new();
    for paragraph in input.split('\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        lines.extend(wrap_line(paragraph, width));
    }
    lines
}

fn wrap_line(line: &str, width: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_w: usize = 0;
    let mut word = String::new();
    let mut word_w: usize = 0;
    let mut active_style: String = String::new();

    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            let mut esc = String::from("\x1b");
            if let Some(&nxt) = chars.peek() {
                esc.push(nxt);
                chars.next();
                if nxt == '[' {
                    for ec in chars.by_ref() {
                        esc.push(ec);
                        if ec.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            }
            if esc == "\x1b[0m" {
                active_style.clear();
            } else {
                active_style.push_str(&esc);
            }
            word.push_str(&esc);
            continue;
        }

        if c == ' ' {
            if !word.is_empty() {
                let sep = if current_w > 0 { 1 } else { 0 };
                if current_w + sep + word_w > width && current_w > 0 {
                    if !active_style.is_empty() {
                        current.push_str("\x1b[0m");
                    }
                    out.push(current);
                    current = active_style.clone();
                    current.push_str(&word);
                    current_w = word_w;
                } else {
                    if current_w > 0 {
                        current.push(' ');
                        current_w += 1;
                    }
                    current.push_str(&word);
                    current_w += word_w;
                }
                word.clear();
                word_w = 0;
            }
        } else {
            word.push(c);
            word_w += UnicodeWidthChar::width(c).unwrap_or(0);
        }
    }

    if !word.is_empty() {
        let sep = if current_w > 0 { 1 } else { 0 };
        if current_w + sep + word_w > width && current_w > 0 {
            if !active_style.is_empty() {
                current.push_str("\x1b[0m");
            }
            out.push(current);
            current = active_style.clone();
            current.push_str(&word);
        } else {
            if current_w > 0 {
                current.push(' ');
            }
            current.push_str(&word);
        }
    }

    if !current.is_empty() || out.is_empty() {
        if !active_style.is_empty() {
            current.push_str("\x1b[0m");
        }
        out.push(current);
    }
    out
}

pub fn visible_width(s: &str) -> usize {
    let mut w = 0usize;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if let Some(&nxt) = chars.peek() {
                chars.next();
                if nxt == '[' {
                    for ec in chars.by_ref() {
                        if ec.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            }
            continue;
        }
        w += UnicodeWidthChar::width(c).unwrap_or(0);
    }
    w
}
