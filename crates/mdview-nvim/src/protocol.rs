use crate::_stubs::NvimHl;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum Message {
    Update {
        text: String,
        #[serde(default)]
        path: Option<PathBuf>,
    },
    Theme {
        colorscheme: String,
        version: String,
        #[serde(default)]
        hl: BTreeMap<String, NvimHl>,
        #[serde(default)]
        force: bool,
    },
    Close,
}

#[derive(thiserror::Error, Debug)]
pub enum ProtocolError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("msgpack decode error: {0}")]
    Decode(#[from] rmpv::decode::Error),
    #[error("msgpack encode error: {0}")]
    Encode(#[from] rmpv::encode::Error),
    #[error("serde error: {0}")]
    Serde(String),
    #[error("frame too large: {0} bytes")]
    FrameTooLarge(u32),
}

pub const MAX_FRAME_BYTES: u32 = 64 * 1024 * 1024;
