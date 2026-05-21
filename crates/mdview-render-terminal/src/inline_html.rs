const SUB_TABLE: &[(char, char)] = &[
    ('(', '₍'),
    (')', '₎'),
    ('+', '₊'),
    ('-', '₋'),
    ('0', '₀'),
    ('1', '₁'),
    ('2', '₂'),
    ('3', '₃'),
    ('4', '₄'),
    ('5', '₅'),
    ('6', '₆'),
    ('7', '₇'),
    ('8', '₈'),
    ('9', '₉'),
    ('=', '₌'),
    ('a', 'ₐ'),
    ('e', 'ₑ'),
    ('h', 'ₕ'),
    ('i', 'ᵢ'),
    ('j', 'ⱼ'),
    ('k', 'ₖ'),
    ('l', 'ₗ'),
    ('m', 'ₘ'),
    ('n', 'ₙ'),
    ('o', 'ₒ'),
    ('p', 'ₚ'),
    ('r', 'ᵣ'),
    ('s', 'ₛ'),
    ('t', 'ₜ'),
    ('u', 'ᵤ'),
    ('v', 'ᵥ'),
    ('x', 'ₓ'),
];

const SUP_TABLE: &[(char, char)] = &[
    ('(', '⁽'),
    (')', '⁾'),
    ('+', '⁺'),
    ('-', '⁻'),
    ('0', '⁰'),
    ('1', '¹'),
    ('2', '²'),
    ('3', '³'),
    ('4', '⁴'),
    ('5', '⁵'),
    ('6', '⁶'),
    ('7', '⁷'),
    ('8', '⁸'),
    ('9', '⁹'),
    ('=', '⁼'),
    ('a', 'ᵃ'),
    ('b', 'ᵇ'),
    ('c', 'ᶜ'),
    ('d', 'ᵈ'),
    ('e', 'ᵉ'),
    ('f', 'ᶠ'),
    ('g', 'ᵍ'),
    ('h', 'ʰ'),
    ('i', 'ⁱ'),
    ('j', 'ʲ'),
    ('k', 'ᵏ'),
    ('l', 'ˡ'),
    ('m', 'ᵐ'),
    ('n', 'ⁿ'),
    ('o', 'ᵒ'),
    ('p', 'ᵖ'),
    ('r', 'ʳ'),
    ('s', 'ˢ'),
    ('t', 'ᵗ'),
    ('u', 'ᵘ'),
    ('v', 'ᵛ'),
    ('w', 'ʷ'),
    ('x', 'ˣ'),
    ('y', 'ʸ'),
    ('z', 'ᶻ'),
];

fn map_char(table: &[(char, char)], c: char) -> Option<char> {
    table.iter().find(|(k, _)| *k == c).map(|(_, v)| *v)
}

pub fn unicode_sub(s: &str) -> String {
    map_or_fallback(s, SUB_TABLE, "sub")
}

pub fn unicode_sup(s: &str) -> String {
    map_or_fallback(s, SUP_TABLE, "sup")
}

fn map_or_fallback(s: &str, table: &[(char, char)], tag: &str) -> String {
    let mapped: Option<String> = s
        .chars()
        .map(|c| map_char(table, c))
        .collect::<Option<String>>();
    match mapped {
        Some(m) => m,
        None => format!("<{0}>{1}</{0}>", tag, s),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HtmlTag {
    SubOpen,
    SubClose,
    SupOpen,
    SupClose,
    Other,
}

pub fn classify(tag_text: &str) -> HtmlTag {
    let t = tag_text.trim().to_ascii_lowercase();
    match t.as_str() {
        "<sub>" => HtmlTag::SubOpen,
        "</sub>" => HtmlTag::SubClose,
        "<sup>" => HtmlTag::SupOpen,
        "</sup>" => HtmlTag::SupClose,
        _ => HtmlTag::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sub_digits_map_to_unicode() {
        assert_eq!(unicode_sub("2"), "₂");
        assert_eq!(unicode_sub("12"), "₁₂");
    }

    #[test]
    fn sup_digits_map_to_unicode() {
        assert_eq!(unicode_sup("2"), "²");
        assert_eq!(unicode_sup("10"), "¹⁰");
    }

    #[test]
    fn unmappable_falls_back_to_literal() {
        assert_eq!(unicode_sub("Q"), "<sub>Q</sub>");
        assert_eq!(unicode_sup("Q"), "<sup>Q</sup>");
    }

    #[test]
    fn classify_known_tags() {
        assert_eq!(classify("<sub>"), HtmlTag::SubOpen);
        assert_eq!(classify("</sub>"), HtmlTag::SubClose);
        assert_eq!(classify("<sup>"), HtmlTag::SupOpen);
        assert_eq!(classify("</sup>"), HtmlTag::SupClose);
        assert_eq!(classify("<SUB>"), HtmlTag::SubOpen);
        assert_eq!(classify("<div>"), HtmlTag::Other);
    }
}
