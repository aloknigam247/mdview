use base64::Engine;

/// Encode `png_bytes` as a sequence of kitty graphics protocol chunks. The
/// payload is base64 encoded and split into 4096-byte pieces; `m=1` marks
/// intermediate chunks, `m=0` the final one.
pub fn kitty_graphics(png_bytes: &[u8]) -> String {
    let encoded = base64::engine::general_purpose::STANDARD.encode(png_bytes);
    let bytes = encoded.as_bytes();
    let chunk_size = 4096;
    let mut out = String::new();
    let total = bytes.len();
    let mut offset = 0;
    let mut first = true;
    while offset < total {
        let end = (offset + chunk_size).min(total);
        let slice = std::str::from_utf8(&bytes[offset..end]).unwrap_or("");
        let more = if end < total { 1 } else { 0 };
        if first {
            out.push_str(&format!("\x1b_Ga=T,f=100,m={more};{slice}\x1b\\"));
            first = false;
        } else {
            out.push_str(&format!("\x1b_Gm={more};{slice}\x1b\\"));
        }
        offset = end;
    }
    out
}

/// Render a rounded box containing `[label]` centered inside a frame `width`
/// columns wide (minimum 6). Lines are separated by `\n`.
pub fn ascii_placeholder(label: &str, width: u16) -> String {
    let width = width.max(6) as usize;
    let inner = width - 2;
    let tag = format!("[{label}]");
    let tag = if tag.chars().count() > inner {
        let mut truncated: String = tag.chars().take(inner.saturating_sub(1)).collect();
        truncated.push('…');
        truncated
    } else {
        tag
    };
    let tag_len = tag.chars().count();
    let total_pad = inner - tag_len;
    let left = total_pad / 2;
    let right = total_pad - left;

    let mut s = String::new();
    s.push('╭');
    for _ in 0..inner {
        s.push('─');
    }
    s.push('╮');
    s.push('\n');

    s.push('│');
    for _ in 0..left {
        s.push(' ');
    }
    s.push_str(&tag);
    for _ in 0..right {
        s.push(' ');
    }
    s.push('│');
    s.push('\n');

    s.push('╰');
    for _ in 0..inner {
        s.push('─');
    }
    s.push('╯');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_placeholder_respects_width() {
        let out = ascii_placeholder("mermaid", 20);
        for line in out.lines() {
            assert_eq!(line.chars().count(), 20);
        }
        assert!(out.contains("[mermaid]"));
    }

    #[test]
    fn ascii_placeholder_enforces_minimum_width() {
        let out = ascii_placeholder("x", 2);
        for line in out.lines() {
            assert_eq!(line.chars().count(), 6);
        }
    }

    #[test]
    fn kitty_graphics_starts_with_apc_and_chunks() {
        let payload = vec![0u8; 10_000];
        let out = kitty_graphics(&payload);
        assert!(out.starts_with("\x1b_Ga=T"));
        assert!(out.contains("m=1"));
        assert!(out.contains("m=0"));
    }
}
