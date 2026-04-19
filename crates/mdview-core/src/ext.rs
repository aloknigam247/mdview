use comrak::nodes::AstNode;
use comrak::ComrakOptions;

use crate::types::{Asset, Html, RenderCtx, TermChunks};

pub trait MdViewExtension: Send + Sync {
    fn name(&self) -> &'static str;

    fn register_parser(&self, _opts: &mut ComrakOptions) {}

    fn pre_parse(&self, _src: &mut String) {}

    fn transform<'a>(&self, _ast: &'a AstNode<'a>) {}

    fn render_html<'a>(&self, _n: &'a AstNode<'a>, _ctx: &RenderCtx<'_>) -> Option<Html> {
        None
    }

    fn render_terminal<'a>(
        &self,
        _n: &'a AstNode<'a>,
        _ctx: &RenderCtx<'_>,
    ) -> Option<TermChunks> {
        None
    }

    fn client_assets(&self) -> &'static [Asset] {
        &[]
    }
}
