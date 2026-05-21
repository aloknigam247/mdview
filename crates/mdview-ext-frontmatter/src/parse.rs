use serde_json::Value;

pub struct Extracted {
    pub remaining: String,
    pub value: Option<Value>,
}

pub fn extract(src: &str) -> Extracted {
    let Some(rest) = strip_prefix_with_newline(src, "---") else {
        return Extracted {
            remaining: src.to_string(),
            value: None,
        };
    };
    let Some((yaml, after)) = find_closing(rest) else {
        return Extracted {
            remaining: src.to_string(),
            value: None,
        };
    };
    match serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yaml) {
        Ok(v) => match yaml_to_json(v) {
            Ok(jv) => Extracted {
                remaining: after.to_string(),
                value: Some(jv),
            },
            Err(e) => {
                tracing::warn!("frontmatter: yaml->json conversion failed: {e}");
                Extracted {
                    remaining: src.to_string(),
                    value: None,
                }
            }
        },
        Err(e) => {
            tracing::warn!("frontmatter: yaml parse failed: {e}");
            Extracted {
                remaining: src.to_string(),
                value: None,
            }
        }
    }
}

fn strip_prefix_with_newline<'a>(src: &'a str, marker: &str) -> Option<&'a str> {
    if let Some(rest) = src.strip_prefix(&format!("{marker}\r\n")) {
        return Some(rest);
    }
    src.strip_prefix(&format!("{marker}\n"))
}

fn find_closing(src: &str) -> Option<(&str, &str)> {
    let mut idx = 0usize;
    for line in src.split_inclusive('\n') {
        let stripped = line.trim_end_matches('\n').trim_end_matches('\r');
        if stripped == "---" {
            let yaml = &src[..idx];
            let after_start = idx + line.len();
            let after = &src[after_start..];
            return Some((yaml, after));
        }
        idx += line.len();
    }
    None
}

fn yaml_to_json(v: serde_yaml_ng::Value) -> Result<Value, String> {
    use serde_yaml_ng::Value as Y;
    Ok(match v {
        Y::Null => Value::Null,
        Y::Bool(b) => Value::Bool(b),
        Y::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::from(i)
            } else if let Some(u) = n.as_u64() {
                Value::from(u)
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        }
        Y::String(s) => Value::String(s),
        Y::Sequence(seq) => {
            let mut out = Vec::with_capacity(seq.len());
            for item in seq {
                out.push(yaml_to_json(item)?);
            }
            Value::Array(out)
        }
        Y::Mapping(map) => {
            let mut out = serde_json::Map::new();
            for (k, val) in map {
                let key = match k {
                    Y::String(s) => s,
                    Y::Number(n) => n.to_string(),
                    Y::Bool(b) => b.to_string(),
                    _ => continue,
                };
                out.insert(key, yaml_to_json(val)?);
            }
            Value::Object(out)
        }
        Y::Tagged(t) => yaml_to_json(t.value)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_frontmatter_passes_through() {
        let r = extract("# Hello\n");
        assert!(r.value.is_none());
        assert_eq!(r.remaining, "# Hello\n");
    }

    #[test]
    fn empty_frontmatter_block() {
        let r = extract("---\n---\n# Body\n");
        assert!(r.value.is_some());
        assert_eq!(r.remaining, "# Body\n");
    }

    #[test]
    fn parses_simple_yaml() {
        let r = extract("---\ntitle: Hello\n---\n# Body\n");
        let v = r.value.unwrap();
        assert_eq!(v["title"], "Hello");
        assert_eq!(r.remaining, "# Body\n");
    }

    #[test]
    fn handles_crlf_delimiters() {
        let src = "---\r\ntitle: Hi\r\n---\r\n# Body\r\n";
        let r = extract(src);
        assert!(r.value.is_some());
        assert!(r.remaining.starts_with("# Body"));
    }

    #[test]
    fn malformed_yaml_returns_none_and_keeps_src() {
        let src = "---\n:::: not yaml ::::\n---\n# Body\n";
        let r = extract(src);
        assert!(r.value.is_none());
        assert_eq!(r.remaining, src);
    }

    #[test]
    fn missing_close_keeps_src() {
        let src = "---\ntitle: x\n# Body\n";
        let r = extract(src);
        assert!(r.value.is_none());
        assert_eq!(r.remaining, src);
    }
}
