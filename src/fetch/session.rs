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

use crate::js::ScriptBody;

use super::request::{derive_headers, Lineage};
use super::{
    build_transport, dispatch, CookieJar, FetchError, FetchResult, Intent, Request, SharedJar,
    Transport, MAX_REDIRECTS, TIMEOUT_SECS,
};

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

    /// Open the invocation's top-level navigation, anchored at the caller's
    /// `url`. No request is issued until [`Navigation::load`].
    pub fn navigation(&self, url: &str) -> Navigation {
        Navigation {
            session: self.clone(),
            anchor: url.to_string(),
            hops: MAX_REDIRECTS,
        }
    }

    /// GET a safe-GET subresource resolved to `url`, initiated from `initiator`
    /// (the page URL, whose origin scopes the `-H` credentials), under `timeout`
    /// (derived by the caller from its phase deadline). Deduped against the
    /// invocation cache: a repeat request re-serves the frozen response with no
    /// network and no budget spent; a miss is fetched through the shared pool
    /// and frozen for the invocation.
    pub fn subresource(
        &self,
        url: &str,
        initiator: &str,
        intent: Intent,
        timeout: Duration,
    ) -> Result<FetchResult, FetchError> {
        let cacheable = !intent.is_image();
        if let Some(hit) = self.cache.lock().unwrap().get(url).filter(|_| cacheable) {
            return Ok(hit.clone());
        }
        let req = Request {
            url,
            intent,
            initiator,
            referrer: Some(initiator),
            timeout,
            body: None,
        };
        let result = self.dispatch(&req, &mut { MAX_REDIRECTS })?;
        if cacheable && vary_permits_reuse(&result.headers) {
            self.cache
                .lock()
                .unwrap()
                .insert(url.to_string(), result.clone());
        }
        Ok(result)
    }

    /// POST the page script's own `body` to `url` — the request the script at
    /// `initiator` chose to issue (`docs/design/challenge.md` §2/§3). Always a
    /// fetch/XHR, never a navigation or any other destination; never cached (a
    /// POST is not idempotent, and the cache is keyed by URL alone); `Set-Cookie`
    /// on the response reaches the jar like any other response's. There is no
    /// way to call this without a [`ScriptBody`], and only `crate::js::subfetch`
    /// can construct one.
    pub(crate) fn submit(
        &self,
        url: &str,
        initiator: &str,
        body: &ScriptBody,
        timeout: Duration,
    ) -> Result<FetchResult, FetchError> {
        let req = Request {
            url,
            intent: Intent::FetchXhr,
            initiator,
            referrer: Some(initiator),
            timeout,
            body: Some(body),
        };
        self.dispatch(&req, &mut { MAX_REDIRECTS })
    }

    /// The one request path: every request rides the shared pool, the jar, and
    /// the caller overrides.
    fn dispatch(&self, req: &Request, hops: &mut usize) -> Result<FetchResult, FetchError> {
        dispatch(&self.transport, &self.jar, &self.headers, req, hops)
    }
}

/// The invocation's top-level navigation — the **one** path by which frot
/// issues a document GET, first or repeated. It holds the caller's URL (the
/// `-H` origin anchor, so no later hop can carry caller credentials
/// cross-origin) and one hop budget ([`MAX_REDIRECTS`]) that every request of
/// every load draws on: an HTTP redirect and a document-directed hop
/// ([`Navigation::follow`]) cost the same, and exhausting it is the one
/// `fetch.redirect` error. Never cached — the navigation is the impression, not
/// a reusable resource. State within a call (VISION principle 1): it lives and
/// dies with the invocation's session.
pub struct Navigation {
    session: FetchSession,
    anchor: String,
    hops: usize,
}

impl Navigation {
    /// GET the caller's URL as a user navigation (no referrer). Calling it again
    /// re-issues that same URL on the same session and budget.
    pub fn load(&mut self) -> Result<FetchResult, FetchError> {
        let anchor = self.anchor.clone();
        self.go(&anchor, None)
    }

    /// GET `target` because the document at `from` directed the navigation
    /// there (a zero-delay meta refresh): `from` is the referrer, at every hop.
    pub fn follow(&mut self, target: &str, from: &str) -> Result<FetchResult, FetchError> {
        self.go(target, Some(from))
    }

    /// A [`Reading`] of the request [`load`](Self::load) would send right now
    /// — the caller's URL, the jar's applicable cookies, the caller `-H`
    /// overrides — sorted, because it is a reading to compare, not a wire
    /// order, beside the page script's share of its cookies.
    pub fn request(&self) -> Reading {
        let (jar, anchor) = (self.session.jar.lock().unwrap(), &self.anchor);
        let cookie = jar.header_for(anchor, &[], Intent::Navigation);
        let caller = &self.session.headers;
        let mut headers = derive_headers(
            Intent::Navigation,
            anchor,
            anchor,
            &Lineage::new(None),
            caller,
            cookie.as_deref(),
            None,
        );
        headers.sort();
        let scripted = jar.scripted_for(anchor);
        Reading { headers, scripted }
    }

    fn go(&mut self, url: &str, referrer: Option<&str>) -> Result<FetchResult, FetchError> {
        let req = Request {
            url,
            intent: Intent::Navigation,
            initiator: &self.anchor,
            referrer,
            timeout: Duration::from_secs(TIMEOUT_SECS),
            body: None,
        };
        self.session.dispatch(&req, &mut self.hops)
    }
}

/// The request a re-issue of the caller's URL would send, read at one moment
/// (`docs/design/challenge.md` §12).
#[derive(Debug)]
pub struct Reading {
    headers: Vec<(String, String)>,
    /// The `Cookie` line of the cookies the page script last wrote.
    scripted: Option<String>,
}

impl Reading {
    /// Whether the page's script, since `before`, changed the request: it
    /// differs, and the script's share of it differs. An identical request is
    /// never re-sent, and a cookie the server set — on the navigation, a
    /// `<script src>`, a stylesheet — earns nothing.
    pub fn earned_since(&self, before: &Reading) -> bool {
        self.headers != before.headers && self.scripted != before.scripted
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
