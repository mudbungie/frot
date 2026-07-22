//! One explicit per-invocation fetch session (bl-5191) — the narrow authority
//! every request passes through (`docs/design/identity.md` §13).
//!
//! A single invocation builds one [`FetchSession`]. It owns:
//!
//! - **the connection pool** — one `ureq::Agent` (built once in
//!   [`super::build_agent`]); the agent is `Clone` over an `Arc` pool, so the
//!   session is a cheap `Clone` handle and every clone — the scoped CSS workers,
//!   the JS subfetch cache — shares the *same* pool. Two sequential same-origin
//!   requests reuse one HTTP/1.1 connection; a separate invocation gets a
//!   separate pool and cannot share it (§3.5, the most browser-unlike behaviour
//!   measured);
//! - **the caller header overrides** — the `-H` set, scoped same-origin to the
//!   page for subresources (credentials never leak cross-origin), layered under
//!   the Firefox navigation headers for the top-level GET;
//! - **request intent + initiator URL** ([`Intent`]) — carried on every request
//!   so the profile sibling (bl-20ec) can derive per-destination headers and
//!   protocol metadata from one place, without each call site knowing the
//!   policy. Today intent only distinguishes a navigation (never cached, nav
//!   headers) from a subresource;
//! - **the invocation-local resource cache** — equivalent safe-GET subresources
//!   are fetched once per invocation and re-served frozen; the top-level
//!   navigation is never cached. A cache hit costs no network and no phase
//!   budget, so `--css --js` fetches an unchanged stylesheet once across the
//!   JS-phase gather and the final cascade, while a JS-inserted new stylesheet
//!   is still a miss and is fetched.
//!
//! Cookies and the exact per-destination header derivation are later siblings
//! (bl-6dad, bl-20ec); this module leaves them the [`Intent`]/initiator seam
//! rather than duplicating policy.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{
    build_transport, dispatch, same_origin, FetchError, FetchResult, Transport, TIMEOUT_SECS,
};

/// Firefox document-navigation default headers, layered *under* the caller's
/// `-H` on the top-level page GET so the request shape matches the User-Agent
/// (and `navigator`) it claims. A bare Firefox UA with `accept: */*` and no
/// `Accept-Language`/`Sec-Fetch-*` is an obvious bot tell; this is consistency
/// of the existing masquerade, not anti-bot evasion.
///
/// `Accept-Encoding` is deliberately absent: ureq sets it from its enabled
/// decoders (`gzip, br`) — we only advertise what we can actually inflate.
/// Subresource fetches (`docs/design/js.md` §6, `docs/design/css.md`) do NOT
/// carry this set: navigate-mode `Sec-Fetch-*` on a subresource is itself
/// inconsistent, so they stay honest-minimal.
const DOCUMENT_HEADERS: &[(&str, &str)] = &[
    (
        "Accept",
        "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
    ),
    ("Accept-Language", "en-US,en;q=0.5"),
    ("Upgrade-Insecure-Requests", "1"),
    ("Sec-Fetch-Dest", "document"),
    ("Sec-Fetch-Mode", "navigate"),
    ("Sec-Fetch-Site", "none"),
    ("Sec-Fetch-User", "?1"),
];

/// A request's role in the page load (`identity.md` §4.2: destination ∈
/// {document, style, script, empty}). The session carries it on every request as
/// the seam bl-20ec derives per-destination headers and protocol metadata from.
/// Today it only gates caching and the navigation header set — a navigation is
/// never cached and is the sole carrier of [`DOCUMENT_HEADERS`]; every subresource
/// is a cacheable safe GET. The finer variants are distinct *today* only as the
/// documented seam, so no call site needs pairwise knowledge later.
#[derive(Clone, Copy)]
pub enum Intent {
    /// The top-level document GET — never cached, carries the nav header set.
    Navigation,
    /// An external `<link rel=stylesheet>` (the `--css` gather).
    Style,
    /// An external classic `<script src>`.
    ClassicScript,
    /// An external ES module or an `import` graph edge.
    Module,
    /// A page `fetch()` / `XMLHttpRequest`.
    FetchXhr,
}

/// The one per-invocation fetch session. A cheap `Clone` handle: every clone
/// shares the connection pool, the header overrides, and the resource cache.
#[derive(Clone)]
pub struct FetchSession {
    transport: Arc<Transport>,
    headers: Arc<Vec<(String, String)>>,
    cache: Arc<Mutex<HashMap<String, FetchResult>>>,
}

impl FetchSession {
    /// Open a session with the caller's `-H` overrides, building the one shared
    /// transport ([`super::build_transport`]). One call per invocation, in
    /// `run.rs`. The transport owns the tokio runtime; dropping the session drops
    /// it, so no runtime outlives the invocation.
    pub fn new(headers: Vec<(String, String)>) -> Self {
        FetchSession {
            transport: Arc::new(build_transport()),
            headers: Arc::new(headers),
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// GET the top-level document: the Firefox navigation headers layered under
    /// the caller's `-H`, under the default [`TIMEOUT_SECS`] ceiling. Never
    /// cached — the navigation is the impression, not a reusable resource.
    pub fn navigate(&self, url: &str) -> Result<FetchResult, FetchError> {
        self.run(
            url,
            url,
            Intent::Navigation,
            Duration::from_secs(TIMEOUT_SECS),
        )
    }

    /// GET a safe-GET subresource resolved to `url`, initiated from `initiator`
    /// (the page URL, whose origin scopes the `-H` credentials), under `timeout`
    /// (derived by the caller from its phase deadline). Deduped against the
    /// invocation cache: a repeat request re-serves the frozen response with no
    /// network and no budget spent.
    pub fn subresource(
        &self,
        url: &str,
        initiator: &str,
        intent: Intent,
        timeout: Duration,
    ) -> Result<FetchResult, FetchError> {
        self.run(url, initiator, intent, timeout)
    }

    /// The one request path. A cacheable intent (everything but a navigation) is
    /// served from the invocation cache when present — no network, no budget —
    /// else fetched through the shared pool and frozen for the invocation.
    fn run(
        &self,
        url: &str,
        initiator: &str,
        intent: Intent,
        timeout: Duration,
    ) -> Result<FetchResult, FetchError> {
        let cacheable = !matches!(intent, Intent::Navigation);
        if cacheable {
            if let Some(hit) = self.cache.lock().unwrap().get(url).cloned() {
                return Ok(hit);
            }
        }
        let headers = self.headers_for(url, initiator, intent);
        let result = dispatch(&self.transport, url, &headers, timeout)?;
        if cacheable {
            self.cache
                .lock()
                .unwrap()
                .insert(url.to_string(), result.clone());
        }
        Ok(result)
    }

    /// The request header set for one destination. A navigation layers the nav
    /// defaults under the caller's `-H`; a subresource carries the `-H` only when
    /// it shares the page's origin (credentials never leak cross-origin), else it
    /// goes anonymous. This is the one place the `-H`/nav policy lives.
    fn headers_for(&self, url: &str, initiator: &str, intent: Intent) -> Vec<(String, String)> {
        if matches!(intent, Intent::Navigation) {
            let mut effective = (*self.headers).clone();
            for (name, value) in DOCUMENT_HEADERS {
                if !self
                    .headers
                    .iter()
                    .any(|(n, _)| n.eq_ignore_ascii_case(name))
                {
                    effective.push(((*name).to_string(), (*value).to_string()));
                }
            }
            effective
        } else if same_origin(url, initiator) {
            (*self.headers).clone()
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod policy_tests;

#[cfg(test)]
mod tests;
