use comrak::nodes::AstNode;
use comrak::ComrakOptions;

use crate::ext::MdViewExtension;

#[derive(Default)]
pub struct Registry {
    extensions: Vec<Box<dyn MdViewExtension>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            extensions: Vec::new(),
        }
    }

    pub fn register(&mut self, ext: Box<dyn MdViewExtension>) {
        self.extensions.push(ext);
    }

    pub fn extensions(&self) -> &[Box<dyn MdViewExtension>] {
        &self.extensions
    }

    pub fn html_renderers(&self) -> impl Iterator<Item = &dyn MdViewExtension> {
        self.extensions.iter().map(|e| e.as_ref())
    }

    pub fn terminal_renderers(&self) -> impl Iterator<Item = &dyn MdViewExtension> {
        self.extensions.iter().map(|e| e.as_ref())
    }

    pub fn apply_parser_opts(&self, opts: &mut ComrakOptions) {
        for e in &self.extensions {
            e.register_parser(opts);
        }
    }

    pub fn apply_pre_parse(&self, src: &mut String) {
        for e in &self.extensions {
            e.pre_parse(src);
        }
    }

    pub fn apply_transforms<'a>(&self, ast: &'a AstNode<'a>) {
        for e in &self.extensions {
            e.transform(ast);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;
    use crate::ext::MdViewExtension;

    struct Counting {
        pre: Arc<AtomicUsize>,
        xf: Arc<AtomicUsize>,
    }

    impl MdViewExtension for Counting {
        fn name(&self) -> &'static str {
            "counting"
        }

        fn pre_parse(&self, src: &mut String) {
            self.pre.fetch_add(1, Ordering::SeqCst);
            src.push_str("\n<!-- pp -->");
        }

        fn transform<'a>(&self, _ast: &'a AstNode<'a>) {
            self.xf.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn register_and_iterate() {
        let mut r = Registry::new();
        r.register(Box::new(Counting {
            pre: Arc::new(AtomicUsize::new(0)),
            xf: Arc::new(AtomicUsize::new(0)),
        }));
        assert_eq!(r.extensions().len(), 1);
        assert_eq!(r.html_renderers().count(), 1);
        assert_eq!(r.terminal_renderers().count(), 1);
    }

    #[test]
    fn pre_parse_runs_for_every_extension() {
        let pre = Arc::new(AtomicUsize::new(0));
        let xf = Arc::new(AtomicUsize::new(0));
        let mut r = Registry::new();
        r.register(Box::new(Counting {
            pre: pre.clone(),
            xf: xf.clone(),
        }));
        r.register(Box::new(Counting {
            pre: pre.clone(),
            xf: xf.clone(),
        }));
        let mut src = String::from("# hi");
        r.apply_pre_parse(&mut src);
        assert_eq!(pre.load(Ordering::SeqCst), 2);
        assert!(src.contains("pp"));
    }
}
