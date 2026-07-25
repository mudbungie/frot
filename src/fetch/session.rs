//! One explicit per-invocation fetch session (bl-5191) — the narrow authority
//! every request passes through (`docs/design/identity.md` §13).
//!
//! A single invocation builds one [`FetchSession`]. It owns:
//!
//! - **the connection pool** — one [`Transport`] (built once in
//!   [`super::build_transport`]), held behind an `Arc`, so the session is a
//!   cheap `Clone` handle and every clone — the scoped CSS workers, the JS
//!   subfetch cache — shares the *same* `hyper_util` pool and the *same* tokio
//!   runtime. Same-origin requests reuse a pooled connection instead of
//!   re-presenting the TLS fingerprint per request (§3.5, the most
//!   browser-unlike behaviour measured); a separate invocation gets a separate
//!   pool and cannot share it. Reuse is **eventual on HTTP/1.1** — hyper checks
//!   an h1 connection back in from a spawned task, so a saturated host can make
//!   a sequential pair open two sockets. That is a declared residual, not a bug
//!   to chase: identity.md §6.5 (`bl-fa12`) weighs owning the pool and declines;
//!   h2 is unaffected (checked in inline);
//! - **the caller header overrides** — the `-H` set, layered by the request
//!   derivation ([`super::request::derive_headers`]) as the final override and
//!   scoped same-origin to the anchor origin, so credentials and custom headers
//!   never leak cross-origin;
//! - **request intent + initiator URL** ([`Intent`]) — carried on every request
//!   so the derivation builds the protocol-correct per-destination header set
//!   and Fetch Metadata from one place, without each call site knowing the
//!   policy; the session passes intent + initiator through to `dispatch`;
//! - **the invocation-local resource cache** — equivalent safe-GET subresources
//!   are fetched once per invocation and re-served frozen; the top-level
//!   navigation is never cached. A cache hit costs no network and no phase
//!   budget, so `--css --js` fetches an unchanged stylesheet once across the
//!   JS-phase gather and the final cascade, while a JS-inserted new stylesheet
//!   is still a miss and is fetched. A content-negotiated response (a `Vary` on
//!   any header frot's requests differ on) is **refused reuse** rather than
//!   risk the wrong body ([`vary_permits_reuse`], bl-08f6).
//!
//! - **the per-invocation cookie jar** (bl-6dad) — born empty, the single
//!   authority for `Set-Cookie`, applicable `Cookie` headers, and
//!   `document.cookie`; shared by `dispatch` (wire) and the JS syscalls (via
//!   [`FetchSession::cookie_jar`]) so one page's cookies are coherent, and dropped
//!   with the session so a fresh invocation starts empty.
//!
//! The per-destination header derivation (bl-20ec) lives in [`super::request`],
//! reached through the [`Intent`]/initiator seam this module hands to `dispatch`
//! — the session holds no header policy of its own.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{
    build_transport, dispatch, CookieJar, FetchError, FetchResult, SharedJar, Transport,
    TIMEOUT_SECS,
};

/// A request's role in the page load (`identity.md` §4.2: destination ∈
/// {document, style, script, empty}). The session carries it on every request as
/// the seam [`super::request::derive_headers`] derives per-destination headers
/// and protocol metadata from. It gates caching (a navigation is never cached;
/// every subresource is a cacheable safe GET) and selects the per-class request
/// metadata (`identity.md` §4.1) — navigation vs style vs classic-script vs
/// module vs fetch/XHR each differ in `Accept`, Fetch Metadata, and `Priority`.
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
    /// The born-empty cookie jar (bl-6dad): the single authority for `Set-Cookie`,
    /// applicable `Cookie` headers, and `document.cookie`. Shared across every
    /// clone (gather threads, JS syscalls) so one page's cookies are coherent;
    /// dropped with the session, so a fresh invocation starts empty.
    jar: SharedJar,
}

impl FetchSession {
    /// Open a session with the caller's `-H` overrides, building the one shared
    /// transport ([`super::build_transport`]) and an empty cookie jar. One call
    /// per invocation, in `run.rs`. The transport owns the tokio runtime; dropping
    /// the session drops it, so no runtime outlives the invocation.
    pub fn new(headers: Vec<(String, String)>) -> Self {
        FetchSession {
            transport: Arc::new(build_transport()),
            headers: Arc::new(headers),
            cache: Arc::new(Mutex::new(HashMap::new())),
            jar: Arc::new(Mutex::new(CookieJar::new())),
        }
    }

    /// A handle to the shared cookie jar, so the JS layer's `document.cookie`
    /// syscalls read/write the *same* jar the transport does (bl-6dad).
    pub(crate) fn cookie_jar(&self) -> SharedJar {
        self.jar.clone()
    }

    /// GET the top-level document: the derivation builds the Firefox navigation
    /// header set with the caller's `-H` as the final override, under the default
    /// [`TIMEOUT_SECS`] ceiling. Never cached — the navigation is the impression,
    /// not a reusable resource.
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
        let result = dispatch(
            &self.transport,
            &self.jar,
            url,
            intent,
            initiator,
            &self.headers,
            timeout,
        )?;
        if cacheable && vary_permits_reuse(&result.headers) {
            self.cache
                .lock()
                .unwrap()
                .insert(url.to_string(), result.clone());
        }
        Ok(result)
    }
}

/// Whether a response may be re-served for a later equivalent-URL request
/// (bl-08f6). A `Vary` that names any request header frot's requests differ on —
/// `Accept`, `Sec-Fetch-*`, `Cookie`/credentials, `Priority` — means the body is
/// content-negotiated, so the cached copy could be the *wrong* body for the next
/// request's intent or cookies: frot conservatively REFUSES to cache it rather
/// than risk a mismatched reuse (task acceptance). `Vary: Accept-Encoding` alone
/// is safe — the derivation sends one fixed `Accept-Encoding` on every request,
/// so it never varies; an empty/absent `Vary` is unconditionally reusable;
/// `Vary: *` never reuses.
fn vary_permits_reuse(headers: &[(String, String)]) -> bool {
    match super::header_value(headers, "vary") {
        None => true,
        Some(v) => v
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .all(|t| t.eq_ignore_ascii_case("accept-encoding")),
    }
}

#[cfg(test)]
mod cache_tests;

#[cfg(test)]
mod policy_tests;

#[cfg(test)]
mod tests;
