pub use mdview_core::{
    MdViewExtension as TerminalRenderer, Registry, RenderCtx, StyleSpec, TermChunk, TermChunks,
    TerminalCaps, Theme,
};

pub trait TermChunksExt {
    fn plain_text(&self) -> String;
    fn push_plain(&mut self, text: impl Into<String>);
    fn push_styled(&mut self, text: impl Into<String>, style: StyleSpec);
    fn to_ansi(&self) -> String;
    fn to_ansi_with(&self, truecolor: bool) -> String;
}

impl TermChunksExt for TermChunks {
    fn plain_text(&self) -> String {
        self.iter().map(|c| c.text.clone()).collect()
    }

    fn push_plain(&mut self, text: impl Into<String>) {
        self.push(TermChunk::plain(text));
    }

    fn push_styled(&mut self, text: impl Into<String>, style: StyleSpec) {
        self.push(TermChunk::new(text, style));
    }

    fn to_ansi(&self) -> String {
        self.to_ansi_with(true)
    }

    fn to_ansi_with(&self, truecolor: bool) -> String {
        let mut out = String::new();
        for chunk in self {
            let open = crate::color::sgr_open(&chunk.style, truecolor);
            if open.is_empty() {
                out.push_str(&chunk.text);
            } else {
                out.push_str(&open);
                out.push_str(&chunk.text);
                out.push_str(crate::color::SGR_RESET);
            }
        }
        out
    }
}
