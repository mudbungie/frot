//! The one per-invocation cookie jar (bl-6dad, `docs/design/identity.md` §9).
//!
//! Born empty, owned by the [`super::FetchSession`], discarded when the
//! invocation ends — **state within a call**, which VISION principle 1 permits;
//! not a session model (nothing is serialized, no browser profile is read, no
//! `--cookie-jar` flag). It is the single authority for cookies across three
//! surfaces that used to disagree:
//!
//! - **`Set-Cookie`** received on redirects, the final document, and real
//!   subresources — [`CookieJar::store`], keyed to the requested URL;
//! - **`Cookie`** on later wire requests — [`CookieJar::header_for`], honouring
//!   Secure, Domain, Path, Expires/Max-Age, SameSite/site-context, and the
//!   request-credential mode (fetch/XHR is same-origin, subresources credentialed);
//! - **`document.cookie`** at the final document — [`CookieJar::document_cookie`]
//!   reads (HttpOnly and non-applicable cookies omitted) and
//!   [`CookieJar::write_script`] writes (browser-allowed attributes only; a JS
//!   write can never mint an HttpOnly cookie, so HttpOnly never enters JS).
//!
//! **Caller `-H Cookie` policy (decided once).** The jar computes the default
//! `Cookie` line; the caller's `-H Cookie` is layered *last* by
//! [`super::request::apply_caller`] as a same-origin value override, replacing
//! that one line in place — never a duplicate, never sent cross-origin, never
//! stored in the jar.
//!
//! **Challenge boundary.** The transport may `store` a `Set-Cookie` while
//! receiving a declared challenge, but `run.rs` exits before parse/JS/subfetch
//! and the session (hence this jar) then drops. frot never retries, so a
//! challenge cookie is never replayed.

use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use url::Url;

use super::request::registrable;
use super::{same_origin, Intent};

mod parse;

#[cfg(test)]
mod tests;

/// The jar, shared behind an `Arc<Mutex<…>>` because [`super::FetchSession`] is a
/// cheap clone handed to up to six gather threads (`transport.rs`) and to the JS
/// `document.cookie` syscalls — one authority, race-safe under the lock.
pub type SharedJar = Arc<Mutex<CookieJar>>;

/// A cookie's cross-site disposition (RFC 6265bis). Absent/unrecognised is `Lax`,
/// as modern browsers default.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SameSite {
    Strict,
    Lax,
    None,
}

/// One stored cookie. `host_only` distinguishes a `Domain`-less cookie (exact
/// host) from a `Domain=` one (subdomain-matching); `created` is the insertion
/// sequence, the RFC 6265 tie-break when two cookies share a path length.
#[derive(Clone, Debug)]
struct Cookie {
    name: String,
    value: String,
    domain: String,
    host_only: bool,
    path: String,
    secure: bool,
    http_only: bool,
    same_site: SameSite,
    /// `None` is a session cookie (kept for the invocation); `Some` is the
    /// absolute expiry used only at insert time to keep-or-drop.
    expires: Option<SystemTime>,
    created: u64,
}

/// The per-invocation jar. A flat list keyed by (name, domain, path) — the RFC
/// 6265 cookie identity — so a re-set replaces in place and a past expiry deletes.
#[derive(Default)]
pub struct CookieJar {
    cookies: Vec<Cookie>,
    seq: u64,
}

impl CookieJar {
    /// A born-empty jar.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store every `Set-Cookie` in `headers` received from `request_url` (a
    /// redirect hop, the final document, or a subresource). HttpOnly is honoured
    /// (wire cookies may be HttpOnly); an unparseable line or a `Domain` the host
    /// cannot set is ignored, exactly as a browser drops it.
    pub fn store(&mut self, headers: &[(String, String)], request_url: &str) {
        let now = SystemTime::now();
        let Ok(url) = Url::parse(request_url) else {
            return;
        };
        for (n, v) in headers {
            if n.eq_ignore_ascii_case("set-cookie") {
                if let Some(c) = parse::set_cookie(v, &url, now, false) {
                    self.insert(c, now);
                }
            }
        }
    }

    /// A `document.cookie = …` write at `page_url` (js.md §7). Parses the
    /// browser-allowed attributes into the same jar; `script = true` forces
    /// `http_only` off, so JS can never mint an HttpOnly cookie.
    pub fn write_script(&mut self, value: &str, page_url: &str) {
        let now = SystemTime::now();
        let Ok(url) = Url::parse(page_url) else {
            return;
        };
        if let Some(c) = parse::set_cookie(value, &url, now, true) {
            self.insert(c, now);
        }
    }

    /// The `Cookie` header value for a wire request to `target` initiated with
    /// `referrer_source` (the site/origin context) under `intent`, or `None` when
    /// nothing applies. `intent` fixes the credential rules: a navigation is a
    /// top-level context (Lax/Strict sent); fetch/XHR is same-origin only;
    /// subresources are credentialed cross-origin (SameSite still gates).
    pub fn header_for(
        &self,
        target: &str,
        referrer_source: Option<&str>,
        intent: Intent,
    ) -> Option<String> {
        let url = Url::parse(target).ok()?;
        let ctx = referrer_source.unwrap_or(target);
        let cx = Context {
            same_site: same_site(ctx, &url),
            top_level_nav: matches!(intent, Intent::Navigation),
            require_same_origin: matches!(intent, Intent::FetchXhr),
            same_origin: same_origin(ctx, target),
            exclude_http_only: false,
        };
        self.serialize(&url, &cx)
    }

    /// The `document.cookie` read string at `page_url`: applicable cookies at the
    /// final document, HttpOnly and non-applicable ones omitted. The document is
    /// its own same-origin top-level context, so SameSite never withholds here.
    pub fn document_cookie(&self, page_url: &str) -> String {
        let Ok(url) = Url::parse(page_url) else {
            return String::new();
        };
        let cx = Context {
            same_site: true,
            top_level_nav: true,
            require_same_origin: false,
            same_origin: true,
            exclude_http_only: true,
        };
        self.serialize(&url, &cx).unwrap_or_default()
    }

    /// Insert `c`, replacing any cookie of the same (name, domain, path); a cookie
    /// already past its expiry is not stored, which is how a past `Expires` /
    /// `Max-Age<=0` deletes one.
    fn insert(&mut self, mut c: Cookie, now: SystemTime) {
        self.cookies
            .retain(|e| !(e.name == c.name && e.domain == c.domain && e.path == c.path));
        if c.expires.is_none_or(|e| e > now) {
            c.created = self.seq;
            self.seq += 1;
            self.cookies.push(c);
        }
    }

    /// Serialize the applicable cookies for `url` under `cx` into `a=1; b=2`,
    /// longest path first then earliest set (RFC 6265 §5.4), or `None` if none
    /// apply.
    fn serialize(&self, url: &Url, cx: &Context) -> Option<String> {
        let host = url.host_str()?;
        let (secure_ctx, path) = (url.scheme() == "https", url.path());
        let mut hits: Vec<&Cookie> = self
            .cookies
            .iter()
            .filter(|c| {
                !(cx.exclude_http_only && c.http_only) && applies(c, host, path, secure_ctx, cx)
            })
            .collect();
        if hits.is_empty() {
            return None;
        }
        hits.sort_by(|a, b| {
            b.path
                .len()
                .cmp(&a.path.len())
                .then(a.created.cmp(&b.created))
        });
        Some(
            hits.iter()
                .map(|c| format!("{}={}", c.name, c.value))
                .collect::<Vec<_>>()
                .join("; "),
        )
    }
}

/// The request context a cookie is evaluated against — the SameSite/credential
/// facts derived once by [`CookieJar::header_for`]/[`document_cookie`], so
/// [`applies`] stays a pure predicate.
struct Context {
    same_site: bool,
    top_level_nav: bool,
    require_same_origin: bool,
    same_origin: bool,
    exclude_http_only: bool,
}

/// Whether `c` is sent to `host`/`path` under `secure_ctx` and the request
/// `cx`: Domain- and Path-match, Secure only on https, the fetch/XHR same-origin
/// gate, and the SameSite disposition against the site context.
fn applies(c: &Cookie, host: &str, path: &str, secure_ctx: bool, cx: &Context) -> bool {
    if !domain_match(host, &c.domain, c.host_only) {
        return false;
    }
    if !path_match(path, &c.path) {
        return false;
    }
    if c.secure && !secure_ctx {
        return false;
    }
    if cx.require_same_origin && !cx.same_origin {
        return false;
    }
    match c.same_site {
        SameSite::Strict => cx.same_site,
        SameSite::Lax => cx.same_site || cx.top_level_nav,
        SameSite::None => true,
    }
}

/// RFC 6265 §5.1.3 domain-match: a host-only cookie needs an exact host; a
/// `Domain=` cookie matches the host and any subdomain of it.
fn domain_match(host: &str, domain: &str, host_only: bool) -> bool {
    if host_only {
        return host == domain;
    }
    host == domain || host.ends_with(&format!(".{domain}"))
}

/// RFC 6265 §5.1.4 path-match: equal, or the cookie-path is a prefix ending at a
/// `/` boundary (either the cookie-path ends in `/`, or the next request char is).
fn path_match(req: &str, cookie: &str) -> bool {
    if req == cookie {
        return true;
    }
    if !req.starts_with(cookie) {
        return false;
    }
    cookie.ends_with('/') || req[cookie.len()..].starts_with('/')
}

/// Whether `ctx` (a URL string) is same-site with `target`: equal registrable
/// domains, from the one [`registrable`] definition. An unparseable context is
/// treated cross-site (the safe default — Strict/Lax withheld).
fn same_site(ctx: &str, target: &Url) -> bool {
    match Url::parse(ctx) {
        Ok(c) => registrable(&c) == registrable(target),
        Err(_) => false,
    }
}
