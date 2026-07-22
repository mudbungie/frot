//! The one protocol-correct request derivation (`bl-20ec`, identity.md §3.3/§4).
//!
//! From `(Intent, target, anchor, referrer source, caller -H)` this builds the
//! single ordered header description both hyper serializers consume — so header
//! identity is derived from the [persona](super::profile) once, never applied by
//! a transport seam (the §3.4 defect this dissolves). The persona *literals* —
//! `User-Agent`, `Accept` per class, `Accept-Language`, `Accept-Encoding` — come
//! from the profile SSOT; only URL-fact policy (`Sec-Fetch-Site`, `Referer`, and
//! the same-origin caller scope) lives here.
//!
//! ## Host / `:authority`, `Connection`, `TE` — the serializer boundary
//!
//! The list carries **no `Host`**: hyper synthesises `Host` (h1) or the
//! `:authority` pseudo-header (h2) from the request URI, so there is one correct
//! authority per protocol and no duplicate `host` line on h2. `TE: trailers` is
//! Firefox's h2 trailer advertisement, sent last; on the h1 fallback hyper
//! title-cases it to `Te: trailers`. The h1 fallback therefore carries two
//! declared residuals of hyper's pooled abstraction (protocol unknown at
//! build-time): the synthesised `Host` lands **last** rather than at Firefox's
//! first position, and it lacks Firefox's `Connection: keep-alive` — `Connection`
//! is stripped on h2 (illegal there) via a `HeaderMap` swap-remove that would
//! displace `te` from last, so it is omitted rather than corrupt the negotiated
//! h2 path, which is the path against every real origin and is fully correct.

use url::Url;

use super::decode::ACCEPT_ENCODING;
use super::profile::FIREFOX_140_ESR;
use super::{same_origin, Intent};

/// Derive the ordered request header description for one hop.
///
/// - `intent` selects the per-class metadata (`Accept`, Fetch Metadata, `Priority`).
/// - `target` is the URL being fetched *this hop* (recomputed each redirect).
/// - `anchor` is the origin the caller `-H` is scoped to (the original request
///   URL): overrides apply only when `target` is same-origin as it, so
///   credentials and custom headers never leak cross-origin — subsuming the
///   `Authorization` safe-redirect rule.
/// - `referrer_source` drives both `Sec-Fetch-Site` and `Referer`: `None` is a
///   user navigation (site `none`, no referer); `Some(url)` is the document (a
///   subresource) or the pre-redirect hop (a navigation redirect).
pub(crate) fn derive_headers(
    intent: Intent,
    target: &str,
    anchor: &str,
    referrer_source: Option<&str>,
    caller: &[(String, String)],
) -> Vec<(String, String)> {
    let p = &FIREFOX_140_ESR;
    let meta = p.request_meta(intent);
    let mut h: Vec<(String, String)> = Vec::new();
    push(&mut h, "User-Agent", p.user_agent());
    push(&mut h, "Accept", meta.accept.to_string());
    push(&mut h, "Accept-Language", p.accept_language());
    push(&mut h, "Accept-Encoding", ACCEPT_ENCODING.to_string());
    if let Some(referer) = referer(referrer_source, target) {
        push(&mut h, "Referer", referer);
    }
    if meta.uir {
        push(&mut h, "Upgrade-Insecure-Requests", "1".to_string());
    }
    push(&mut h, "Sec-Fetch-Dest", meta.dest.to_string());
    push(&mut h, "Sec-Fetch-Mode", meta.mode.to_string());
    push(
        &mut h,
        "Sec-Fetch-Site",
        site(referrer_source, target).to_string(),
    );
    if let Some(user) = meta.user {
        push(&mut h, "Sec-Fetch-User", user.to_string());
    }
    push(&mut h, "Priority", meta.priority.to_string());
    push(&mut h, "TE", "trailers".to_string());
    apply_caller(&mut h, anchor, target, caller);
    h
}

/// Append one derived header (canonical Title-Cased name kept for the h1
/// serializer; hyper lowercases for h2).
fn push(h: &mut Vec<(String, String)>, name: &str, value: String) {
    h.push((name.to_string(), value));
}

/// `Sec-Fetch-Site` from URL facts (never a caller label): `none` for a user
/// navigation, else the origin relation between the referrer source and target.
fn site(referrer_source: Option<&str>, target: &str) -> &'static str {
    match referrer_source {
        None => "none",
        Some(src) if same_origin(src, target) => "same-origin",
        Some(src) if same_site(src, target) => "same-site",
        Some(_) => "cross-site",
    }
}

/// The `Referer` under Firefox's default `strict-origin-when-cross-origin`: a
/// same-origin request sends the full source URL (fragment and userinfo
/// stripped); a cross-origin request sends only the source origin; an
/// https→http downgrade, and a user navigation, send nothing.
fn referer(referrer_source: Option<&str>, target: &str) -> Option<String> {
    let (src, t) = (Url::parse(referrer_source?).ok()?, Url::parse(target).ok()?);
    if src.scheme() == "https" && t.scheme() == "http" {
        return None;
    }
    if same_origin(src.as_str(), t.as_str()) {
        let mut u = src.clone();
        u.set_fragment(None);
        let _ = u.set_username("");
        let _ = u.set_password(None);
        Some(u.to_string())
    } else {
        Some(format!("{}/", src.origin().ascii_serialization()))
    }
}

/// Schemeful same-site: same scheme and same registrable domain, but not
/// same-origin (that case is decided first). Without a public-suffix list the
/// registrable domain is approximated by the last two labels of a *domain* —
/// exact for the common `cdn.` / `www.` split of one domain; multi-label public
/// suffixes (`co.uk`) are a declared residual, never a credential decision (that
/// is same-origin only).
fn same_site(a: &str, b: &str) -> bool {
    let (Ok(a), Ok(b)) = (Url::parse(a), Url::parse(b)) else {
        return false;
    };
    a.scheme() == b.scheme() && registrable(&a) == registrable(&b)
}

/// The registrable domain: the last two labels of a domain host; an IP literal
/// (or a host with no domain) is its own registrable identity, never split — so
/// two distinct IPs are never mistaken for same-site.
fn registrable(u: &Url) -> String {
    match u.host() {
        Some(url::Host::Domain(d)) => {
            let mut labels: Vec<&str> = d.rsplitn(3, '.').collect();
            labels.truncate(2);
            labels.reverse();
            labels.join(".")
        }
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

/// Layer the caller `-H` as the final case-insensitive override — but only
/// same-origin to the `anchor`, so cross-origin hops carry no caller headers at
/// all. A named header replaces the derived value in place (no duplicate line,
/// canonical name kept); an unnamed one appends with the caller's own name.
fn apply_caller(
    h: &mut Vec<(String, String)>,
    anchor: &str,
    target: &str,
    caller: &[(String, String)],
) {
    if !same_origin(anchor, target) {
        return;
    }
    for (name, value) in caller {
        match h.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(name)) {
            Some(slot) => slot.1 = value.clone(),
            None => h.push((name.clone(), value.clone())),
        }
    }
}

#[cfg(test)]
mod recorder;
#[cfg(test)]
mod tests;
