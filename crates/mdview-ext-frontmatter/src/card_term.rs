use serde_json::Value;

use mdview_core::{StyleSpec, TermChunk, TermChunks};

const DEFAULT_WIDTH: usize = 80;

pub fn render(value: &Value, width: usize) -> TermChunks {
    let w = if width == 0 {
        DEFAULT_WIDTH
    } else {
        width.min(DEFAULT_WIDTH)
    };
    let inner = w.saturating_sub(4);

    let mut lines: Vec<String> = Vec::new();

    let Value::Object(map) = value else {
        return Vec::new();
    };

    if let Some(title) = scalar_str(map.get("title")) {
        lines.push(title);
    }
    if let Some(sub) =
        scalar_str(map.get("subtitle")).or_else(|| scalar_str(map.get("description")))
    {
        lines.push(sub);
    }

    let date = scalar_str(map.get("date"));
    let author = author_display(map.get("author"));
    let meta_line = match (date, author) {
        (Some(d), Some(a)) => Some(format!("{} \u{00b7} {}", d, a)),
        (Some(d), None) => Some(d),
        (None, Some(a)) => Some(a),
        (None, None) => None,
    };
    if let Some(m) = meta_line {
        lines.push(String::new());
        lines.push(m);
    }

    if let Some(tags) = tags_line(map.get("tags")) {
        lines.push(tags);
    }

    if let Some(Value::Object(ao)) = map.get("author") {
        let extras: Vec<(&String, &Value)> =
            ao.iter().filter(|(k, _)| k.as_str() != "name").collect();
        if !extras.is_empty() {
            lines.push(String::new());
            lines.push("\u{25be} author".to_string());
            let key_w = extras.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
            for (k, v) in &extras {
                lines.push(format!(
                    "    {:<kw$}  {}",
                    k,
                    scalar_to_string(v),
                    kw = key_w
                ));
            }
        }
    }

    let recognized = [
        "title",
        "subtitle",
        "description",
        "date",
        "author",
        "tags",
        "draft",
    ];
    let mut nested: Vec<(&String, &Value)> = map
        .iter()
        .filter(|(k, v)| !recognized.contains(&k.as_str()) && matches!(v, Value::Object(_)))
        .collect();
    nested.sort_by(|a, b| a.0.cmp(b.0));
    for (k, v) in nested {
        lines.push(String::new());
        lines.push(format!("\u{25be} {}", k));
        if let Value::Object(m) = v {
            let key_w = m.keys().map(|k| k.len()).max().unwrap_or(0);
            let mut entries: Vec<(&String, &Value)> = m.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            for (sk, sv) in entries {
                let sval = match sv {
                    Value::Array(arr) => arr
                        .iter()
                        .map(scalar_to_string)
                        .collect::<Vec<_>>()
                        .join(" \u{00b7} "),
                    other => scalar_to_string(other),
                };
                lines.push(format!("    {:<kw$}  {}", sk, sval, kw = key_w));
            }
        }
    }

    let mut out = TermChunks::new();
    let muted = StyleSpec {
        fg: Some("#6b7280".to_string()),
        ..Default::default()
    };
    let title_style = StyleSpec {
        bold: true,
        fg: Some("#111827".to_string()),
        ..Default::default()
    };

    let top = format!("\u{256d}{}\u{256e}", "\u{2500}".repeat(w.saturating_sub(2)));
    let bot = format!("\u{2570}{}\u{256f}", "\u{2500}".repeat(w.saturating_sub(2)));

    out.push(TermChunk::new(top, muted.clone()));
    out.push(TermChunk::plain("\n".to_string()));

    let mut first_line = true;
    for line in &lines {
        let segments = wrap_to(line, inner);
        for seg in segments {
            out.push(TermChunk::new("\u{2502} ".to_string(), muted.clone()));
            let style = if first_line && !seg.is_empty() {
                title_style.clone()
            } else {
                StyleSpec::default()
            };
            let pad = inner.saturating_sub(visible_len(&seg));
            out.push(TermChunk::new(seg, style));
            if pad > 0 {
                out.push(TermChunk::plain(" ".repeat(pad)));
            }
            out.push(TermChunk::new(" \u{2502}".to_string(), muted.clone()));
            out.push(TermChunk::plain("\n".to_string()));
            first_line = false;
        }
    }

    out.push(TermChunk::new(bot, muted));
    out.push(TermChunk::plain("\n".to_string()));

    out
}

fn tags_line(v: Option<&Value>) -> Option<String> {
    let items: Vec<String> = match v? {
        Value::String(s) => vec![s.clone()],
        Value::Array(arr) => {
            if arr.iter().any(|x| matches!(x, Value::Object(_))) {
                return None;
            }
            arr.iter().map(scalar_to_string).collect()
        }
        _ => return None,
    };
    if items.is_empty() {
        return None;
    }
    let parts: Vec<String> = items.iter().map(|t| format!("\u{25cf} {}", t)).collect();
    Some(parts.join("   "))
}

fn author_display(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) => Some(s.clone()),
        Value::Object(m) => m
            .get("name")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}

fn scalar_str(v: Option<&Value>) -> Option<String> {
    v.and_then(|x| match x {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    })
}

fn scalar_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        _ => String::new(),
    }
}

fn wrap_to(s: &str, width: usize) -> Vec<String> {
    if s.is_empty() {
        return vec![String::new()];
    }
    if width == 0 {
        return vec![s.to_string()];
    }
    let mut out = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let end = (i + width).min(chars.len());
        out.push(chars[i..end].iter().collect());
        i = end;
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn visible_len(s: &str) -> usize {
    s.chars().count()
}
