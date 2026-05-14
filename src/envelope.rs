//! Output envelope: machine-first JSON shape for frot's stdout.
//!
//! Field names and shape are load-bearing — callers parse against them.
//! Later capability flags (`--css`, `--js`) and views (`ax`, `bboxes`)
//! extend the existing fields without changing them.

use serde::{Deserialize, Serialize};

pub const ENVELOPE_VERSION: &str = "0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum View {
    Dom,
    Text,
    Ax,
    Links,
    Forms,
    Bboxes,
    Meta,
}

impl View {
    pub const ALL: &'static [(&'static str, View)] = &[
        ("dom", View::Dom),
        ("text", View::Text),
        ("ax", View::Ax),
        ("links", View::Links),
        ("forms", View::Forms),
        ("bboxes", View::Bboxes),
        ("meta", View::Meta),
    ];

    pub fn parse(s: &str) -> Option<View> {
        View::ALL
            .iter()
            .find(|(name, _)| *name == s)
            .map(|(_, v)| *v)
    }

    pub fn as_str(&self) -> &'static str {
        View::ALL
            .iter()
            .find(|(_, v)| v == self)
            .map(|(n, _)| *n)
            .expect("View::ALL covers every variant")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NeedsKind {
    Js,
    Css,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusKind {
    Ok,
    Needs,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlBlock {
    pub requested: String,
    #[serde(rename = "final", skip_serializing_if = "Option::is_none", default)]
    pub final_url: Option<String>,
}

impl UrlBlock {
    pub fn requested(url: impl Into<String>) -> Self {
        Self {
            requested: url.into(),
            final_url: None,
        }
    }

    pub fn resolved(requested: impl Into<String>, final_url: impl Into<String>) -> Self {
        Self {
            requested: requested.into(),
            final_url: Some(final_url.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub kind: String,
    pub message: String,
}

impl ErrorInfo {
    pub fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub frot: String,
    pub url: UrlBlock,
    pub view: View,
    pub status: StatusKind,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub out: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub needs: Option<Vec<NeedsKind>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<ErrorInfo>,
}

impl Envelope {
    pub fn ok(url: UrlBlock, view: View, out: serde_json::Value) -> Self {
        Self {
            frot: ENVELOPE_VERSION.into(),
            url,
            view,
            status: StatusKind::Ok,
            out: Some(out),
            needs: None,
            error: None,
        }
    }

    pub fn needs(
        url: UrlBlock,
        view: View,
        needs: Vec<NeedsKind>,
        out: Option<serde_json::Value>,
    ) -> Self {
        Self {
            frot: ENVELOPE_VERSION.into(),
            url,
            view,
            status: StatusKind::Needs,
            out,
            needs: Some(needs),
            error: None,
        }
    }

    pub fn error(url: UrlBlock, view: View, error: ErrorInfo) -> Self {
        Self {
            frot: ENVELOPE_VERSION.into(),
            url,
            view,
            status: StatusKind::Error,
            out: None,
            needs: None,
            error: Some(error),
        }
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).expect("envelope is always serializable")
    }
}

pub mod kinds {
    pub const USAGE: &str = "usage";
    pub const FETCH_URL: &str = "fetch.url";
    pub const FETCH_DNS: &str = "fetch.dns";
    pub const FETCH_CONNECT: &str = "fetch.connect";
    pub const FETCH_TLS: &str = "fetch.tls";
    pub const FETCH_TIMEOUT: &str = "fetch.timeout";
    pub const FETCH_REDIRECT: &str = "fetch.redirect";
    pub const FETCH_BODY: &str = "fetch.body";
    pub const FETCH_ENCODING: &str = "fetch.encoding";
    pub const PARSE: &str = "parse";
    pub const INTERNAL: &str = "internal";
}

#[cfg(test)]
mod tests;
