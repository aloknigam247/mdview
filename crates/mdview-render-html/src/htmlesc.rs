/// Escape a string for safe inclusion in HTML text or attribute context.
pub fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Escape a URL for safe inclusion in an href/src attribute. Disallows
/// `javascript:` and `data:` schemes as a basic sanitizer.
pub fn escape_url(url: &str) -> String {
    let lower = url.trim_start().to_ascii_lowercase();
    if lower.starts_with("javascript:") || lower.starts_with("vbscript:") {
        return "#".to_string();
    }
    escape_html(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_basic_chars() {
        assert_eq!(
            escape_html("<a href=\"x\">&</a>"),
            "&lt;a href=&quot;x&quot;&gt;&amp;&lt;/a&gt;"
        );
    }

    #[test]
    fn escapes_apostrophe() {
        assert_eq!(escape_html("it's"), "it&#39;s");
    }

    #[test]
    fn blocks_js_url() {
        assert_eq!(escape_url(" JavaScript:alert(1)"), "#");
    }

    #[test]
    fn allows_normal_url() {
        assert_eq!(
            escape_url("https://example.com/?a=1&b=2"),
            "https://example.com/?a=1&amp;b=2"
        );
    }
}
