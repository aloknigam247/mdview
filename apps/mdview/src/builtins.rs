use mdview_core::MdViewExtension;

// Selective-match extensions (those gated on a specific fenced-block
// info-string) must precede the catch-all `Highlight`, which matches any
// code block and would otherwise shadow the diagram extensions.
#[allow(dead_code)]
pub fn builtin_extensions() -> Vec<Box<dyn MdViewExtension>> {
    vec![
        Box::new(mdview_ext_drawio::Drawio),
        Box::new(mdview_ext_mermaid::Mermaid),
        Box::new(mdview_ext_plotly::Plotly),
        Box::new(mdview_ext_math::Math),
        Box::new(mdview_ext_highlight::Highlight),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_five_extensions_registered() {
        let exts = builtin_extensions();
        assert_eq!(exts.len(), 5);
        let names: Vec<&'static str> = exts.iter().map(|e| e.name()).collect();
        assert!(names.contains(&"drawio"));
        assert!(names.contains(&"highlight"));
        assert!(names.contains(&"math"));
        assert!(names.contains(&"mermaid"));
        assert!(names.contains(&"plotly"));
    }

    #[test]
    fn extension_names_are_unique() {
        let exts = builtin_extensions();
        let mut names: Vec<&'static str> = exts.iter().map(|e| e.name()).collect();
        names.sort();
        let len_before = names.len();
        names.dedup();
        assert_eq!(names.len(), len_before);
    }
}
