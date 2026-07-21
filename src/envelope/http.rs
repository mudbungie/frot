//! The HTTP response signal surfaced in the envelope: the transport status
//! code and a bounded, decision-relevant slice of the response headers.
//!
//! ## Why an allowlist, not the whole set
//!
//! Response headers are the evidence that separates a bot-defence refusal from
//! a genuine response (`docs/design/identity.md` §3.7) — but the raw set is the
//! wrong thing to surface: it is unbounded (machine-first output would bloat),
//! it carries session material (`set-cookie`, owned by the cookie jar `bl-6dad`
//! and never stdout), and it carries per-request volatiles (`date`,
//! request/trace ids) that would make golden captures non-deterministic
//! (`identity.md` §12). So [`SURFACED`] is a fixed allowlist chosen on one
//! principle: *the headers a caller or `needs` uses to tell a bot-defence
//! refusal from a genuine response.* Each entry earns its place from the
//! measured corpus (`identity.md` §3.7): `retry-after` / `cf-mitigated` are the
//! declared-deferral markers `needs::challenge` keys on (`needs.md` §3),
//! `server` and `x-datadome` are the CDN / vendor challenge markers that made
//! reddit and g2 distinguishable, and `content-type` is the response media type.
//!
//! ## One capture, two consumers
//!
//! `needs::challenge` (`needs.md` §3) and `http.headers` read *the same*
//! [`crate::fetch::FetchResult::headers`] — `needs` decides from it, this field
//! exposes the allowlisted slice of it. They cannot disagree because there is
//! one capture; a challenge's declaring header (`retry-after`, `cf-mitigated`)
//! is inside [`SURFACED`], so it is always the one surfaced (pinned in
//! `run::http_tests`).

use serde::{Deserialize, Serialize};

/// One surfaced response header. A flat `{name, value}` pair, not a map entry:
/// HTTP allows a header name to repeat, and an ordered list preserves repeats
/// and wire order honestly where a map would silently collapse them (`bl-acec`).
/// `name` is lower-cased so callers match without case folding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

/// The bounded allowlist of response headers surfaced in `http.headers`,
/// lower-case for case-insensitive matching. Rationale and per-entry
/// provenance: module docs, `docs/design/needs.md` §3, `identity.md` §3.7/§15.
pub const SURFACED: &[&str] = &[
    "retry-after",
    "cf-mitigated",
    "server",
    "x-datadome",
    "content-type",
];

/// HTTP response signal, present whenever a network response was received (a
/// `file://` read carries none — no response happened). Additive to the
/// envelope: the envelope `status` describes the impression operation, `status`
/// here reports the raw transport code the server returned, and `headers`
/// surfaces the allowlisted response headers in wire order (module docs) — the
/// evidence behind frot's own `needs` signal, no longer discarded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpInfo {
    pub status: u16,
    pub headers: Vec<HttpHeader>,
}

impl HttpInfo {
    /// Build from the one response capture: keep the raw status and the
    /// [`SURFACED`] slice of `headers`, in the order the server sent them.
    pub fn new(status: u16, headers: &[(String, String)]) -> Self {
        Self {
            status,
            headers: surfaced(headers),
        }
    }
}

/// The allowlisted response headers, lower-cased name, source order preserved
/// (repeats kept). `set-cookie` and every volatile per-request header fall
/// outside [`SURFACED`] and never appear.
fn surfaced(headers: &[(String, String)]) -> Vec<HttpHeader> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            let name = name.to_ascii_lowercase();
            SURFACED.contains(&name.as_str()).then(|| HttpHeader {
                name,
                value: value.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(n, v)| (n.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn keeps_only_allowlisted_headers_lowercased() {
        let info = HttpInfo::new(
            200,
            &hdr(&[
                ("Server", "snooserv"),
                ("Date", "irrelevant"),
                ("Content-Type", "text/html"),
            ]),
        );
        assert_eq!(info.status, 200);
        assert_eq!(
            info.headers,
            vec![
                HttpHeader {
                    name: "server".into(),
                    value: "snooserv".into(),
                },
                HttpHeader {
                    name: "content-type".into(),
                    value: "text/html".into(),
                },
            ],
        );
    }

    #[test]
    fn strips_set_cookie_session_material() {
        let info = HttpInfo::new(200, &hdr(&[("Set-Cookie", "sid=secret; HttpOnly")]));
        assert!(info.headers.is_empty(), "set-cookie must never surface");
    }

    #[test]
    fn preserves_order_and_repeats() {
        let info = HttpInfo::new(
            200,
            &hdr(&[
                ("retry-after", "0"),
                ("x-datadome", "protected"),
                ("retry-after", "5"),
            ]),
        );
        let seen: Vec<_> = info
            .headers
            .iter()
            .map(|h| (h.name.as_str(), h.value.as_str()))
            .collect();
        assert_eq!(
            seen,
            vec![
                ("retry-after", "0"),
                ("x-datadome", "protected"),
                ("retry-after", "5"),
            ],
        );
    }

    #[test]
    fn serde_round_trip() {
        let info = HttpInfo::new(403, &hdr(&[("server", "cloudflare")]));
        let s = serde_json::to_string(&info).unwrap();
        assert_eq!(
            s,
            "{\"status\":403,\"headers\":[{\"name\":\"server\",\"value\":\"cloudflare\"}]}"
        );
        let back: HttpInfo = serde_json::from_str(&s).unwrap();
        assert_eq!(back, info);
    }

    #[test]
    fn empty_when_no_allowlisted_header_present() {
        let info = HttpInfo::new(200, &hdr(&[("x-frame-options", "DENY")]));
        assert_eq!(
            serde_json::to_string(&info).unwrap(),
            "{\"status\":200,\"headers\":[]}"
        );
    }
}
