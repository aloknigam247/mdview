use comrak::nodes::AstNode;
use comrak::{parse_document, Arena, ComrakOptions};

use crate::registry::Registry;

pub fn parse<'a>(arena: &'a Arena<AstNode<'a>>, src: &str, registry: &Registry) -> &'a AstNode<'a> {
    let mut buf = src.to_string();
    registry.apply_pre_parse(&mut buf);

    let mut opts = ComrakOptions::default();
    configure_gfm(&mut opts);
    registry.apply_parser_opts(&mut opts);

    let root = parse_document(arena, &buf, &opts);
    registry.apply_transforms(root);
    root
}

fn configure_gfm(opts: &mut ComrakOptions) {
    opts.extension.autolink = true;
    opts.extension.footnotes = true;
    opts.extension.front_matter_delimiter = Some("---".to_string());
    opts.extension.math_code = true;
    opts.extension.math_dollars = true;
    opts.extension.strikethrough = true;
    opts.extension.table = true;
    opts.extension.tagfilter = true;
    opts.extension.tasklist = true;
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use comrak::nodes::NodeValue;
    use comrak::Arena;

    use super::*;
    use crate::ext::MdViewExtension;

    struct Tracker {
        pre_calls: Arc<AtomicUsize>,
        transform_calls: Arc<AtomicUsize>,
    }

    impl MdViewExtension for Tracker {
        fn name(&self) -> &'static str {
            "tracker"
        }

        fn pre_parse(&self, src: &mut String) {
            self.pre_calls.fetch_add(1, Ordering::SeqCst);
            src.push_str("\n\nappended\n");
        }

        fn transform<'a>(&self, _ast: &'a AstNode<'a>) {
            self.transform_calls.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn has_heading<'a>(root: &'a AstNode<'a>) -> bool {
        root.descendants()
            .any(|n| matches!(n.data.borrow().value, NodeValue::Heading(_)))
    }

    fn has_table<'a>(root: &'a AstNode<'a>) -> bool {
        root.descendants()
            .any(|n| matches!(n.data.borrow().value, NodeValue::Table(_)))
    }

    #[test]
    fn gfm_sample_parses() {
        let arena = Arena::new();
        let registry = Registry::new();
        let src =
            "# Title\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n~~strike~~ and https://example.com\n";
        let root = parse(&arena, src, &registry);
        assert!(has_heading(root));
        assert!(has_table(root));
    }

    #[test]
    fn pre_parse_is_invoked() {
        let pre = Arc::new(AtomicUsize::new(0));
        let xf = Arc::new(AtomicUsize::new(0));
        let mut registry = Registry::new();
        registry.register(Box::new(Tracker {
            pre_calls: pre.clone(),
            transform_calls: xf.clone(),
        }));
        let arena = Arena::new();
        let _ = parse(&arena, "# hi", &registry);
        assert_eq!(pre.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn transform_is_invoked() {
        let pre = Arc::new(AtomicUsize::new(0));
        let xf = Arc::new(AtomicUsize::new(0));
        let mut registry = Registry::new();
        registry.register(Box::new(Tracker {
            pre_calls: pre,
            transform_calls: xf.clone(),
        }));
        let arena = Arena::new();
        let _ = parse(&arena, "# hi", &registry);
        assert_eq!(xf.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn front_matter_is_accepted() {
        let arena = Arena::new();
        let registry = Registry::new();
        let src = "---\ntitle: t\n---\n\n# body\n";
        let root = parse(&arena, src, &registry);
        assert!(has_heading(root));
    }
}
