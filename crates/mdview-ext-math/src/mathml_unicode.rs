//! Minimal MathML → Unicode text converter.
//!
//! Accepts the MathML output of `latex2mathml` and renders a best-effort
//! Unicode string. This is intentionally small: it handles the constructs we
//! can losslessly map (scripts, fractions, roots, greek letters, operators).

pub fn mathml_to_unicode(mathml: &str) -> Option<String> {
    let mut parser = Parser::new(mathml);
    let root = parser.parse_element()?;
    Some(render(&root))
}

#[derive(Debug)]
struct Element {
    name: String,
    children: Vec<Node>,
}

#[derive(Debug)]
enum Node {
    Elem(Element),
    Text(String),
}

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src: src.as_bytes(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            if b.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn starts_with(&self, s: &str) -> bool {
        self.src[self.pos..].starts_with(s.as_bytes())
    }

    fn parse_element(&mut self) -> Option<Element> {
        self.skip_ws();
        if self.starts_with("<?") {
            while !self.starts_with("?>") && self.pos < self.src.len() {
                self.pos += 1;
            }
            self.pos += 2;
            self.skip_ws();
        }
        while self.starts_with("<!") {
            while self.pos < self.src.len() && self.peek() != Some(b'>') {
                self.pos += 1;
            }
            self.pos += 1;
            self.skip_ws();
        }
        self.parse_tag_body()
    }

    fn parse_tag_body(&mut self) -> Option<Element> {
        if self.peek() != Some(b'<') {
            return None;
        }
        self.bump();
        let name = self.parse_name();
        self.skip_attributes();
        let self_close = self.peek() == Some(b'/');
        if self_close {
            self.bump();
        }
        if self.peek() != Some(b'>') {
            return None;
        }
        self.bump();
        if self_close {
            return Some(Element {
                name,
                children: vec![],
            });
        }
        let mut children = Vec::new();
        loop {
            if self.pos >= self.src.len() {
                break;
            }
            if self.starts_with(&format!("</{}", name)) {
                while self.pos < self.src.len() && self.peek() != Some(b'>') {
                    self.pos += 1;
                }
                self.bump();
                break;
            }
            if self.peek() == Some(b'<') {
                if let Some(child) = self.parse_tag_body() {
                    children.push(Node::Elem(child));
                } else {
                    break;
                }
            } else {
                let start = self.pos;
                while let Some(b) = self.peek() {
                    if b == b'<' {
                        break;
                    }
                    self.pos += 1;
                }
                let slice = std::str::from_utf8(&self.src[start..self.pos]).unwrap_or("");
                children.push(Node::Text(decode_entities(slice)));
            }
        }
        Some(Element { name, children })
    }

    fn skip_attributes(&mut self) {
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'>') | Some(b'/') | None => return,
                _ => {}
            }
            // attr name
            while let Some(b) = self.peek() {
                if b == b'=' || b.is_ascii_whitespace() || b == b'>' || b == b'/' {
                    break;
                }
                self.pos += 1;
            }
            self.skip_ws();
            if self.peek() == Some(b'=') {
                self.bump();
                self.skip_ws();
                let q = self.peek();
                if q == Some(b'"') || q == Some(b'\'') {
                    let quote = q.unwrap();
                    self.bump();
                    while let Some(b) = self.peek() {
                        self.bump();
                        if b == quote {
                            break;
                        }
                    }
                } else {
                    while let Some(b) = self.peek() {
                        if b.is_ascii_whitespace() || b == b'>' || b == b'/' {
                            break;
                        }
                        self.pos += 1;
                    }
                }
            }
        }
    }

    fn parse_name(&mut self) -> String {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_alphanumeric() || b == b':' || b == b'-' || b == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        String::from_utf8_lossy(&self.src[start..self.pos]).into_owned()
    }
}

fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let after = &rest[amp..];
        if let Some(semi) = after.find(';') {
            let entity = &after[1..semi];
            let replacement = match entity {
                "lt" => Some('<'),
                "gt" => Some('>'),
                "amp" => Some('&'),
                "apos" => Some('\''),
                "quot" => Some('"'),
                _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                    u32::from_str_radix(&entity[2..], 16)
                        .ok()
                        .and_then(char::from_u32)
                }
                _ if entity.starts_with('#') => {
                    entity[1..].parse::<u32>().ok().and_then(char::from_u32)
                }
                _ => None,
            };
            if let Some(c) = replacement {
                out.push(c);
                rest = &after[semi + 1..];
                continue;
            }
        }
        out.push('&');
        rest = &after[1..];
    }
    out.push_str(rest);
    out
}

fn render(elem: &Element) -> String {
    render_node(elem).trim().to_string()
}

fn render_node(elem: &Element) -> String {
    let local = elem.name.rsplit(':').next().unwrap_or(&elem.name);
    match local {
        "math" | "mrow" | "mstyle" | "semantics" | "mpadded" | "mphantom" => render_children(elem),
        "mi" | "mn" | "mo" | "mtext" => collect_text(elem),
        "mspace" => " ".to_string(),
        "msup" => {
            let kids = render_children_separate(elem);
            if kids.len() >= 2 {
                format!("{}{}", kids[0], to_superscript(&kids[1]))
            } else {
                kids.join("")
            }
        }
        "msub" => {
            let kids = render_children_separate(elem);
            if kids.len() >= 2 {
                format!("{}{}", kids[0], to_subscript(&kids[1]))
            } else {
                kids.join("")
            }
        }
        "msubsup" => {
            let kids = render_children_separate(elem);
            if kids.len() >= 3 {
                format!(
                    "{}{}{}",
                    kids[0],
                    to_subscript(&kids[1]),
                    to_superscript(&kids[2])
                )
            } else {
                kids.join("")
            }
        }
        "munder" | "mover" | "munderover" => render_children_separate(elem).join(""),
        "mfrac" => {
            let kids = render_children_separate(elem);
            if kids.len() >= 2 {
                format!("({})/({})", kids[0], kids[1])
            } else {
                kids.join("")
            }
        }
        "msqrt" => {
            let inner = render_children(elem);
            format!("√({})", inner)
        }
        "mroot" => {
            let kids = render_children_separate(elem);
            if kids.len() >= 2 {
                format!("{}√({})", to_superscript(&kids[1]), kids[0])
            } else {
                kids.join("")
            }
        }
        "mfenced" => {
            let inner = render_children(elem);
            format!("({})", inner)
        }
        "mtable" => render_children(elem),
        "mtr" => {
            let row = render_children(elem);
            format!("{}\n", row)
        }
        "mtd" => format!("{} ", render_children(elem)),
        "annotation" | "annotation-xml" => String::new(),
        _ => render_children(elem),
    }
}

fn render_children(elem: &Element) -> String {
    let mut out = String::new();
    for child in &elem.children {
        match child {
            Node::Elem(e) => out.push_str(&render_node(e)),
            Node::Text(t) => out.push_str(&normalize_text(t)),
        }
    }
    out
}

fn render_children_separate(elem: &Element) -> Vec<String> {
    let mut out = Vec::new();
    for child in &elem.children {
        match child {
            Node::Elem(e) => out.push(render_node(e)),
            Node::Text(t) => {
                let n = normalize_text(t);
                if !n.trim().is_empty() {
                    out.push(n);
                }
            }
        }
    }
    out
}

fn collect_text(elem: &Element) -> String {
    let mut out = String::new();
    for child in &elem.children {
        match child {
            Node::Text(t) => out.push_str(&normalize_text(t)),
            Node::Elem(e) => out.push_str(&render_node(e)),
        }
    }
    out
}

fn normalize_text(t: &str) -> String {
    t.chars()
        .filter(|c| !c.is_whitespace() || *c == ' ')
        .collect()
}

fn to_superscript(s: &str) -> String {
    map_script(s, superscript_char, '^')
}

fn to_subscript(s: &str) -> String {
    map_script(s, subscript_char, '_')
}

fn map_script(s: &str, mapper: fn(char) -> Option<char>, sigil: char) -> String {
    if s.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match mapper(c) {
            Some(m) => out.push(m),
            None => return format!("{sigil}({s})"),
        }
    }
    out
}

fn superscript_char(c: char) -> Option<char> {
    Some(match c {
        '0' => '\u{2070}',
        '1' => '\u{00B9}',
        '2' => '\u{00B2}',
        '3' => '\u{00B3}',
        '4' => '\u{2074}',
        '5' => '\u{2075}',
        '6' => '\u{2076}',
        '7' => '\u{2077}',
        '8' => '\u{2078}',
        '9' => '\u{2079}',
        '+' => '\u{207A}',
        '-' => '\u{207B}',
        '=' => '\u{207C}',
        '(' => '\u{207D}',
        ')' => '\u{207E}',
        'a' => '\u{1D43}',
        'b' => '\u{1D47}',
        'c' => '\u{1D9C}',
        'i' => '\u{2071}',
        'n' => '\u{207F}',
        'x' => '\u{02E3}',
        'y' => '\u{02B8}',
        _ => return None,
    })
}

fn subscript_char(c: char) -> Option<char> {
    Some(match c {
        '0' => '\u{2080}',
        '1' => '\u{2081}',
        '2' => '\u{2082}',
        '3' => '\u{2083}',
        '4' => '\u{2084}',
        '5' => '\u{2085}',
        '6' => '\u{2086}',
        '7' => '\u{2087}',
        '8' => '\u{2088}',
        '9' => '\u{2089}',
        '+' => '\u{208A}',
        '-' => '\u{208B}',
        '=' => '\u{208C}',
        '(' => '\u{208D}',
        ')' => '\u{208E}',
        'a' => '\u{2090}',
        'e' => '\u{2091}',
        'i' => '\u{1D62}',
        'n' => '\u{2099}',
        'o' => '\u{2092}',
        'x' => '\u{2093}',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_superscript() {
        let mathml = "<math><mi>a</mi><msup><mi>b</mi><mn>2</mn></msup></math>";
        let out = mathml_to_unicode(mathml).unwrap();
        assert!(out.contains('\u{00B2}'), "got {out:?}");
    }

    #[test]
    fn frac() {
        let mathml = "<math><mfrac><mi>a</mi><mi>b</mi></mfrac></math>";
        let out = mathml_to_unicode(mathml).unwrap();
        assert!(out.contains('/'), "got {out:?}");
    }
}
