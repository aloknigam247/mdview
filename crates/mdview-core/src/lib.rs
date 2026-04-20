#![forbid(unsafe_code)]

pub mod builtins;
pub mod ext;
pub mod parser;
pub mod registry;
pub mod types;

pub use builtins::builtin_extensions;
pub use ext::MdViewExtension;
pub use parser::parse;
pub use registry::Registry;
pub use types::{
    Asset, Html, Radii, RenderCtx, StyleSpec, TermChunk, TermChunks, TerminalCaps, Theme,
    Typography,
};

pub use comrak;
