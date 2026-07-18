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

/// HTTP response signal, present whenever a network response was received
/// (a `file://` read carries none — no response happened). Additive to the
/// envelope: the envelope `status` still describes the impression operation,
/// while `http.status` reports the raw transport code the server returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpInfo {
    pub status: u16,
}

impl HttpInfo {
    pub fn new(status: u16) -> Self {
        Self { status }
    }
}

/// One surfaced JS error message (`docs/design/js.md` §10): `kind` is the error
/// class (`throw` / `report` / `subfetch`), `text` its message. Present only in
/// the `js.messages` array, itself emitted only under `--js-errors`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsMessage {
    pub kind: String,
    pub text: String,
}

impl JsMessage {
    pub fn new(kind: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            text: text.into(),
        }
    }
}

/// The `--js` execution signal (`docs/design/js.md` §10). Additive and emitted
/// only when `--js` ran: `scripts` counts scripts executed, `errors` counts
/// throws / unhandled rejections / refused fetches, `settled` is whether the
/// event loop reached quiescence within the wall-clock budget. `messages` — the
/// bounded per-error detail behind `errors` — is present only under `--js-errors`
/// (`errors` is always the authoritative count). This is the honesty channel for
/// *partial* execution outcome detection can't see.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsInfo {
    pub scripts: u32,
    pub errors: u32,
    pub settled: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub messages: Option<Vec<JsMessage>>,
}

impl JsInfo {
    pub fn new(scripts: u32, errors: u32, settled: bool) -> Self {
        Self {
            scripts,
            errors,
            settled,
            messages: None,
        }
    }

    /// Attach the bounded §10 message detail (`--js-errors`). Additive; without
    /// this the `messages` field is absent, leaving the count-only shape intact.
    pub fn with_messages(mut self, messages: Vec<JsMessage>) -> Self {
        self.messages = Some(messages);
        self
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
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub http: Option<HttpInfo>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub js: Option<JsInfo>,
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
            http: None,
            js: None,
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
            http: None,
            js: None,
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
            http: None,
            js: None,
        }
    }

    /// Attach an HTTP response signal to any envelope variant. Additive and
    /// idempotent — `None` leaves the envelope untouched (e.g. `file://`).
    pub fn with_http(mut self, http: Option<HttpInfo>) -> Self {
        self.http = http;
        self
    }

    /// Attach the `--js` execution signal (`docs/design/js.md` §10). Additive;
    /// `None` (no `--js`) leaves the envelope untouched.
    pub fn with_js(mut self, js: Option<JsInfo>) -> Self {
        self.js = js;
        self
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).expect("envelope is always serializable")
    }
}

pub mod kinds {
    // Non-2xx/3xx responses (status >= 400) flip the envelope to `error` with a
    // dynamic `http.<code>` kind (e.g. `http.404`, `http.500`) built at the call
    // site; `ErrorInfo.kind` is a `String`, so no const is needed here.
    pub const USAGE: &str = "usage";
    pub const FETCH_URL: &str = "fetch.url";
    pub const FETCH_DNS: &str = "fetch.dns";
    pub const FETCH_CONNECT: &str = "fetch.connect";
    pub const FETCH_TLS: &str = "fetch.tls";
    pub const FETCH_TIMEOUT: &str = "fetch.timeout";
    pub const FETCH_REDIRECT: &str = "fetch.redirect";
    pub const FETCH_BODY: &str = "fetch.body";
    pub const FETCH_FILE: &str = "fetch.file";
    pub const FETCH_ENCODING: &str = "fetch.encoding";
    pub const PARSE: &str = "parse";
    pub const INTERNAL: &str = "internal";
}

#[cfg(test)]
mod tests;
