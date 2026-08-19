#![deny(unsafe_code)]

//! Frontmatter (YAML) extension for `mdview`.
//!
//! Detects a YAML frontmatter block delimited by `---` at the very top of the
//! markdown source, parses it into a `serde_json::Value`, strips it from the
//! source before comrak runs, and emits a "hero card" prepended to the HTML
//! and terminal renders.
//!
//! TOML (`+++`) frontmatter is not supported in v1.

mod card_html;
mod card_term;
mod parse;

use std::sync::Mutex;

use comrak::nodes::AstNode;
use mdview_core::{Html, MdViewExtension, RenderCtx, TermChunks};
use serde_json::Value;

#[derive(Default)]
pub struct FrontmatterExt {
    inner: Mutex<State>,
}

#[derive(Default, Clone)]
struct State {
    value: Option<Value>,
}

impl FrontmatterExt {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn value(&self) -> Option<Value> {
        self.inner.lock().ok().and_then(|s| s.value.clone())
    }
}

impl MdViewExtension for FrontmatterExt {
    fn name(&self) -> &'static str {
        "frontmatter"
    }

    fn pre_parse(&self, src: &mut String) {
        let extracted = parse::extract(src);
        if let Some(value) = extracted.value {
            *src = extracted.remaining;
            if let Ok(mut guard) = self.inner.lock() {
                guard.value = Some(value);
            }
        }
    }

    fn transform<'a>(&self, _ast: &'a AstNode<'a>) {}

    fn render_html<'a>(&self, _n: &'a AstNode<'a>, _ctx: &RenderCtx<'_>) -> Option<Html> {
        None
    }

    fn render_terminal<'a>(&self, _n: &'a AstNode<'a>, _ctx: &RenderCtx<'_>) -> Option<TermChunks> {
        None
    }

    fn pre_render_html(&self, ctx: &RenderCtx<'_>) -> Option<Html> {
        let value = ctx.frontmatter.clone().or_else(|| self.value())?;
        Some(Html(card_html::render(&value)))
    }

    fn pre_render_terminal(&self, ctx: &RenderCtx<'_>) -> Option<TermChunks> {
        let value = ctx.frontmatter.clone().or_else(|| self.value())?;
        let width = ctx.terminal_caps.map(|c| c.width as usize).unwrap_or(80);
        Some(card_term::render(&value, width))
    }
}
