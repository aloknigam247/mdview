//! Tiny scanner used by tests and tooling that wants to find d2 blocks
//! without spinning up a full comrak parse.

/// Extracts the body of every ` ```d2 ` fenced code block in `src`.
pub fn d2_blocks(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut lines = src.lines();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("```") else {
            continue;
        };
        let info = rest.trim();
        let is_d2 = info
            .split_whitespace()
            .next()
            .map(|t| t.eq_ignore_ascii_case("d2"))
            .unwrap_or(false);
        if !is_d2 {
            continue;
        }
        let mut body = String::new();
        for inner in lines.by_ref() {
            if inner.trim_start().starts_with("```") {
                break;
            }
            body.push_str(inner);
            body.push('\n');
        }
        out.push(body);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_single_block() {
        let src = "pre\n```d2\na -> b\n```\npost\n";
        let blocks = d2_blocks(src);
        assert_eq!(blocks, vec!["a -> b\n"]);
    }

    #[test]
    fn ignores_non_d2_fences() {
        let src = "```rust\nfn main(){}\n```\n";
        assert!(d2_blocks(src).is_empty());
    }

    #[test]
    fn accepts_info_with_trailing_words() {
        let src = "```d2 theme=default\nx -> y\n```\n";
        assert_eq!(d2_blocks(src).len(), 1);
    }
}
