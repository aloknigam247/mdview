use serde_json::Value;

pub fn render(value: &Value) -> String {
    let Value::Object(map) = value else {
        return String::new();
    };

    let mut out = String::new();
    out.push_str("<style>");
    out.push_str(BASE_CSS);
    out.push_str("</style>");

    let title = scalar_str(map.get("title"));
    let subtitle = scalar_str(map.get("subtitle")).or_else(|| scalar_str(map.get("description")));
    let date = scalar_str(map.get("date"));
    let author_name = author_display(map.get("author"));
    let draft = matches!(map.get("draft"), Some(Value::Bool(true)));

    out.push_str("<details class=\"mdview-frontmatter\" open>");
    out.push_str("<summary class=\"mdview-frontmatter__summary\">");
    if let Some(t) = &title {
        out.push_str("<span class=\"mdview-frontmatter__title\">");
        out.push_str(&escape(t));
        out.push_str("</span>");
    }
    if draft {
        out.push_str(" <span class=\"mdview-frontmatter__draft\">draft</span>");
    }
    out.push_str("</summary>");

    out.push_str("<div class=\"mdview-frontmatter__body\">");

    if let Some(sub) = subtitle {
        out.push_str("<p class=\"mdview-frontmatter__subtitle\">");
        out.push_str(&escape(&sub));
        out.push_str("</p>");
    }

    if date.is_some() || author_name.is_some() {
        out.push_str("<div class=\"mdview-frontmatter__meta\">");
        if let Some(d) = &date {
            out.push_str("<span class=\"mdview-frontmatter__date\">");
            out.push_str(&escape(d));
            out.push_str("</span>");
        }
        if date.is_some() && author_name.is_some() {
            out.push_str(" <span class=\"mdview-frontmatter__sep\">·</span> ");
        }
        if let Some(a) = &author_name {
            out.push_str("<span class=\"mdview-frontmatter__author\">");
            out.push_str(&escape(a));
            out.push_str("</span>");
        }
        out.push_str("</div>");
    }

    if let Some(tags) = render_tag_list(map.get("tags")) {
        out.push_str(&tags);
    }

    if let Some(Value::Object(ao)) = map.get("author") {
        let extras: Vec<(&String, &Value)> =
            ao.iter().filter(|(k, _)| k.as_str() != "name").collect();
        if !extras.is_empty() {
            let mut sub = serde_json::Map::new();
            for (k, v) in extras {
                sub.insert(k.clone(), v.clone());
            }
            out.push_str(&render_group("author", &Value::Object(sub)));
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
    let mut other_scalars: Vec<(&String, &Value)> = map
        .iter()
        .filter(|(k, v)| !recognized.contains(&k.as_str()) && is_scalar(v))
        .collect();
    other_scalars.sort_by(|a, b| a.0.cmp(b.0));
    if !other_scalars.is_empty() {
        out.push_str("<dl class=\"mdview-frontmatter__other\">");
        for (k, v) in other_scalars {
            out.push_str("<dt>");
            out.push_str(&escape(k));
            out.push_str("</dt><dd>");
            out.push_str(&escape(&scalar_to_string(v)));
            out.push_str("</dd>");
        }
        out.push_str("</dl>");
    }

    let mut nested: Vec<(&String, &Value)> = map
        .iter()
        .filter(|(k, v)| !recognized.contains(&k.as_str()) && !is_scalar(v))
        .collect();
    nested.sort_by(|a, b| a.0.cmp(b.0));
    for (k, v) in nested {
        out.push_str(&render_group(k, v));
    }

    out.push_str("</div></details>");
    out
}

fn group_class(key: &str) -> &'static str {
    let h = key
        .bytes()
        .fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32))
        % 6;
    match h {
        0 => "mdview-frontmatter__group--c0",
        1 => "mdview-frontmatter__group--c1",
        2 => "mdview-frontmatter__group--c2",
        3 => "mdview-frontmatter__group--c3",
        4 => "mdview-frontmatter__group--c4",
        _ => "mdview-frontmatter__group--c5",
    }
}

fn render_group(key: &str, value: &Value) -> String {
    let mut s = String::new();
    s.push_str("<details class=\"mdview-frontmatter__group ");
    s.push_str(group_class(key));
    s.push_str("\" open><summary>");
    s.push_str(&escape(key));
    s.push_str("</summary>");
    match value {
        Value::Object(m) => {
            s.push_str("<dl>");
            let mut entries: Vec<(&String, &Value)> = m.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            for (k, v) in entries {
                s.push_str("<dt>");
                s.push_str(&escape(k));
                s.push_str("</dt><dd>");
                s.push_str(&render_value_inline(v));
                s.push_str("</dd>");
            }
            s.push_str("</dl>");
        }
        Value::Array(arr) => {
            if !arr.is_empty() && arr.iter().all(is_link_object) {
                s.push_str("<ul>");
                for v in arr {
                    if let Value::Object(o) = v {
                        let title = o.get("title").and_then(|x| x.as_str()).unwrap_or("");
                        let url = o
                            .get("url")
                            .or_else(|| o.get("href"))
                            .and_then(|x| x.as_str())
                            .unwrap_or("#");
                        s.push_str("<li><a href=\"");
                        s.push_str(&escape_attr(url));
                        s.push_str("\">");
                        s.push_str(&escape(title));
                        s.push_str("</a></li>");
                    }
                }
                s.push_str("</ul>");
            } else if arr.iter().all(|v| matches!(v, Value::Object(_))) {
                let mut first = true;
                for v in arr {
                    if !first {
                        s.push_str("<hr>");
                    }
                    first = false;
                    s.push_str(&render_value_inline(v));
                }
            } else {
                s.push_str("<p>");
                let parts: Vec<String> = arr.iter().map(|v| escape(&scalar_to_string(v))).collect();
                s.push_str(&parts.join(" · "));
                s.push_str("</p>");
            }
        }
        _ => {
            s.push_str("<p>");
            s.push_str(&escape(&scalar_to_string(value)));
            s.push_str("</p>");
        }
    }
    s.push_str("</details>");
    s
}

fn render_value_inline(v: &Value) -> String {
    match v {
        Value::Object(m) => {
            let mut s = String::from("<dl>");
            let mut entries: Vec<(&String, &Value)> = m.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            for (k, val) in entries {
                s.push_str("<dt>");
                s.push_str(&escape(k));
                s.push_str("</dt><dd>");
                s.push_str(&render_value_inline(val));
                s.push_str("</dd>");
            }
            s.push_str("</dl>");
            s
        }
        Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(|x| escape(&scalar_to_string(x))).collect();
            parts.join(" · ")
        }
        _ => escape(&scalar_to_string(v)),
    }
}

fn render_tag_list(v: Option<&Value>) -> Option<String> {
    let v = v?;
    let items: Vec<String> = match v {
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
    let mut s = String::from("<ul class=\"mdview-frontmatter__tags\">");
    for it in items {
        s.push_str("<li>");
        s.push_str(&escape(&it));
        s.push_str("</li>");
    }
    s.push_str("</ul>");
    Some(s)
}

fn is_scalar(v: &Value) -> bool {
    !matches!(v, Value::Array(_) | Value::Object(_))
}

fn is_link_object(v: &Value) -> bool {
    let Value::Object(m) = v else { return false };
    m.contains_key("title") && (m.contains_key("url") || m.contains_key("href"))
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

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn escape_attr(s: &str) -> String {
    escape(s)
}

const BASE_CSS: &str = ".mdview-frontmatter{background:var(--mdv-frontmatter-card-bg,#45475a);border:1px solid var(--mdv-frontmatter-card-border,#11111b);border-left:3px solid var(--mdv-accent-mauve,#cba6f7);border-radius:12px;padding:1em 1.25em 1em 1em;margin:0 0 1.5em;}.mdview-frontmatter__summary{cursor:pointer;list-style:none;}.mdview-frontmatter__summary::-webkit-details-marker{display:none;}.mdview-frontmatter__title{font-size:1.5em;font-weight:700;color:var(--mdv-frontmatter-heading-fg,#cdd6f4);}.mdview-frontmatter__draft{background:#ef4444;color:#fff;border-radius:6px;font-size:0.7em;padding:0.1em 0.4em;vertical-align:middle;margin-left:0.5em;}.mdview-frontmatter__subtitle{color:var(--mdv-frontmatter-subtitle-fg,#bac2de);font-style:italic;margin:0.25em 0 0.75em;font-size:1em;}.mdview-frontmatter__meta{color:var(--mdv-frontmatter-muted-fg,#a6adc8);font-size:0.9em;margin-bottom:0.5em;}.mdview-frontmatter__date{color:var(--mdv-accent-peach,#fab387);}.mdview-frontmatter__author{color:var(--mdv-accent-green,#a6e3a1);}.mdview-frontmatter__sep{opacity:0.6;}.mdview-frontmatter__tags{list-style:none;padding:0;display:flex;flex-wrap:wrap;gap:0.4em;margin:0.5em 0 0;}.mdview-frontmatter__tags li{background:color-mix(in srgb,var(--mdv-frontmatter-tag-bg,#cba6f7) 25%,transparent);color:var(--mdv-frontmatter-tag-fg,#cba6f7);border-radius:999px;padding:0.15em 0.6em;font-size:0.85em;}.mdview-frontmatter__group{margin-top:0.75em;}.mdview-frontmatter__group summary{cursor:pointer;color:var(--mdv-frontmatter-muted-fg,#a6adc8);font-family:var(--mdv-font-mono,ui-monospace,monospace);font-size:0.85em;text-transform:uppercase;letter-spacing:0.05em;}.mdview-frontmatter__group--c0>summary{color:var(--mdv-accent-mauve,#cba6f7);}.mdview-frontmatter__group--c1>summary{color:var(--mdv-accent-teal,#94e2d5);}.mdview-frontmatter__group--c2>summary{color:var(--mdv-accent-blue,#89b4fa);}.mdview-frontmatter__group--c3>summary{color:var(--mdv-accent-peach,#fab387);}.mdview-frontmatter__group--c4>summary{color:var(--mdv-accent-yellow,#f9e2af);}.mdview-frontmatter__group--c5>summary{color:var(--mdv-accent-green,#a6e3a1);}.mdview-frontmatter__group dl,.mdview-frontmatter__other{display:grid;grid-template-columns:max-content 1fr;gap:0.25em 1em;margin:0.25em 0;}.mdview-frontmatter__group dt,.mdview-frontmatter__other dt{color:var(--mdv-frontmatter-muted-fg,#a6adc8);font-family:var(--mdv-font-mono,ui-monospace,monospace);font-size:0.85em;}.mdview-frontmatter__group dd,.mdview-frontmatter__other dd{color:var(--mdv-fg,#cdd6f4);margin:0;}.mdview-frontmatter__group ul{list-style:disc;padding-left:1.25em;margin:0.25em 0;}";
