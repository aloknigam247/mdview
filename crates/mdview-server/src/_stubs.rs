// TODO: replace with mdview_core / mdview_theme / mdview_render_html after integration

use std::collections::BTreeMap;

use bytes::Bytes;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Html(pub String);

impl Html {
    pub fn new<S: Into<String>>(s: S) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Default)]
pub struct Theme {
    pub name: String,
    pub css: String,
}

impl Theme {
    pub fn new<N: Into<String>, C: Into<String>>(name: N, css: C) -> Self {
        Self {
            name: name.into(),
            css: css.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Asset {
    pub path: String,
    pub bytes: Bytes,
    pub content_type: &'static str,
}

impl Asset {
    pub fn new<P: Into<String>, B: Into<Bytes>>(
        path: P,
        bytes: B,
        content_type: &'static str,
    ) -> Self {
        Self {
            path: path.into(),
            bytes: bytes.into(),
            content_type,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RenderCtx {
    pub theme_name: String,
    pub flags: BTreeMap<String, String>,
}
