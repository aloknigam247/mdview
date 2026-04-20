//! Tiny scanner used by the example binaries.
//!
//! We avoid pulling in comrak for the HTML-path demos so examples stay cheap
//! and the snapshot is fully deterministic.

/// Extracts the body of every ` ```mermaid ` fenced code block in `src`.
pub fn mermaid_blocks(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut lines = src.lines();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix("```") else {
            continue;
        };
        let info = rest.trim();
        let is_mermaid = info
            .split_whitespace()
            .next()
            .map(|t| t.eq_ignore_ascii_case("mermaid"))
            .unwrap_or(false);
        if !is_mermaid {
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
        let src = "pre\n```mermaid\ngraph TD; A-->B;\n```\npost\n";
        let blocks = mermaid_blocks(src);
        assert_eq!(blocks, vec!["graph TD; A-->B;\n"]);
    }

    #[test]
    fn ignores_non_mermaid_fences() {
        let src = "```rust\nfn main(){}\n```\n";
        assert!(mermaid_blocks(src).is_empty());
    }

    #[test]
    fn accepts_info_with_trailing_words() {
        let src = "```mermaid theme=default\ngraph LR; X-->Y;\n```\n";
        assert_eq!(mermaid_blocks(src).len(), 1);
    }
}
