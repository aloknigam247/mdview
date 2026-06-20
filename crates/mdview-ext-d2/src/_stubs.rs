pub use comrak::nodes::AstNode;
pub use mdview_core::{
    Asset, Html, MdViewExtension, RenderCtx, StyleSpec, TermChunk, TermChunks, Theme,
};

pub trait SixelRenderer: Send + Sync {
    fn render_image(&self, svg: &[u8]) -> Result<Vec<u8>, String>;
}
