//! The jar contract: domain/path/secure/samesite/credential applicability, the
//! `Set-Cookie` store/replace/delete, the `Cookie` serialization order, and the
//! `document.cookie` read (HttpOnly and non-applicable omitted). Set-Cookie
//! parsing detail lives in `parse::tests`; these drive the jar's public surface.

use super::*;

/// Store one `Set-Cookie` `line` as received from `url`.
fn set(j: &mut CookieJar, line: &str, url: &str) {
    j.store(&[("Set-Cookie".to_string(), line.to_string())], url);
}

/// A jar seeded from a list of `(Set-Cookie, request-url)`.
fn jar(sets: &[(&str, &str)]) -> CookieJar {
    let mut j = CookieJar::new();
    for (line, url) in sets {
        set(&mut j, line, url);
    }
    j
}

/// The nav `Cookie` for `target` from `ctx` (`None` = first-hop navigation).
fn nav(j: &CookieJar, target: &str, ctx: Option<&str>) -> Option<String> {
    j.header_for(target, ctx, Intent::Navigation)
}

#[test]
fn host_only_cookie_scopes_to_the_exact_host() {
    let j = jar(&[("a=1", "https://example.com/")]);
    assert_eq!(
        nav(&j, "https://example.com/", None).as_deref(),
        Some("a=1")
    );
    // A host-only cookie reaches neither a subdomain nor a foreign host.
    assert_eq!(nav(&j, "https://sub.example.com/", None), None);
    assert_eq!(nav(&j, "https://other.com/", None), None);
}

#[test]
fn a_domain_cookie_matches_the_host_and_its_subdomains() {
    let j = jar(&[("a=1; Domain=example.com", "https://www.example.com/")]);
    assert_eq!(
        nav(&j, "https://www.example.com/", None).as_deref(),
        Some("a=1")
    );
    assert_eq!(
        nav(&j, "https://example.com/", None).as_deref(),
        Some("a=1")
    );
    assert_eq!(nav(&j, "https://evil.com/", None), None);
}

#[test]
fn a_domain_the_host_cannot_set_is_rejected() {
    // Set-Cookie with a Domain the request host does not domain-match is dropped.
    let j = jar(&[("a=1; Domain=other.com", "https://www.example.com/")]);
    assert_eq!(nav(&j, "https://other.com/", None), None);
    assert_eq!(nav(&j, "https://www.example.com/", None), None);
}

#[test]
fn path_match_respects_the_slash_boundary() {
    let j = jar(&[
        ("a=1; Path=/app", "https://x.com/app/p"),
        ("b=2; Path=/app/", "https://x.com/app/p"),
    ]);
    // Exact, and a prefix that stops at a '/' boundary, both match `a`.
    assert_eq!(nav(&j, "https://x.com/app", None).as_deref(), Some("a=1"));
    assert!(nav(&j, "https://x.com/app/deep", None)
        .unwrap()
        .contains("a=1"));
    // `/application` shares the prefix but not on a boundary → `a` withheld.
    assert_eq!(nav(&j, "https://x.com/application", None), None);
    // `b` (Path=/app/) needs a trailing segment: withheld at `/app`, sent at
    // `/app/x` alongside `a`, and longest-path-first orders it first.
    assert_eq!(
        nav(&j, "https://x.com/app/x", None).as_deref(),
        Some("b=2; a=1")
    );
    // A shallower path than the cookie's does not match.
    assert_eq!(nav(&j, "https://x.com/", None), None);
}

#[test]
fn secure_cookies_ride_only_https() {
    let j = jar(&[("a=1; Secure", "https://x.com/"), ("b=2", "http://x.com/")]);
    assert!(nav(&j, "https://x.com/", None).unwrap().contains("a=1"));
    // Over http the Secure cookie is withheld; the plain one still rides.
    assert_eq!(nav(&j, "http://x.com/", None).as_deref(), Some("b=2"));
}

#[test]
fn samesite_gates_the_cross_site_context() {
    let j = jar(&[
        ("s=1; SameSite=Strict", "https://example.com/"),
        ("l=1; SameSite=Lax", "https://example.com/"),
        ("n=1; SameSite=None; Secure", "https://example.com/"),
    ]);
    // First-party subresource (same-site, not a nav): all three ride.
    assert_eq!(
        j.header_for(
            "https://example.com/api",
            Some("https://example.com/p"),
            Intent::Style
        )
        .as_deref(),
        Some("s=1; l=1; n=1")
    );
    // Third-party subresource (cross-site, not a nav): only SameSite=None.
    assert_eq!(
        j.header_for(
            "https://example.com/px",
            Some("https://other.com/"),
            Intent::Style
        )
        .as_deref(),
        Some("n=1")
    );
    // Cross-site top-level navigation: Lax (safe GET) and None, not Strict.
    assert_eq!(
        nav(&j, "https://example.com/", Some("https://other.com/")).as_deref(),
        Some("l=1; n=1")
    );
}

#[test]
fn an_unparseable_context_is_treated_cross_site() {
    // A bad referrer source is not same-site, so a Lax (non-nav) subresource
    // withholds the cookie rather than guessing.
    let j = jar(&[("l=1; SameSite=Lax", "https://example.com/")]);
    assert_eq!(
        j.header_for("https://example.com/api", Some("::bad"), Intent::Style),
        None
    );
}

#[test]
fn fetch_xhr_sends_cookies_only_same_origin() {
    let j = jar(&[("a=1", "https://example.com/")]);
    // Same-origin fetch: credentialed.
    assert_eq!(
        j.header_for(
            "https://example.com/api",
            Some("https://example.com/p"),
            Intent::FetchXhr
        )
        .as_deref(),
        Some("a=1")
    );
    // A same-site but cross-origin fetch (subdomain page) sends nothing.
    assert_eq!(
        j.header_for(
            "https://example.com/api",
            Some("https://sub.example.com/p"),
            Intent::FetchXhr
        ),
        None
    );
}

#[test]
fn a_first_hop_navigation_is_same_site_with_itself() {
    // No referrer → the target is its own context, so a Strict cookie rides a
    // direct navigation (typing the URL), as a browser sends it.
    let j = jar(&[("s=1; SameSite=Strict", "https://example.com/")]);
    assert_eq!(
        nav(&j, "https://example.com/", None).as_deref(),
        Some("s=1")
    );
}

#[test]
fn unparseable_or_hostless_targets_yield_no_cookie() {
    let j = jar(&[("a=1", "https://example.com/")]);
    assert_eq!(nav(&j, "::bad", None), None);
    // A parseable but hostless URL (no host to match) sends nothing.
    assert_eq!(nav(&j, "file:///x", None), None);
    // And a bad request URL on store is ignored, not panicked.
    let mut j2 = CookieJar::new();
    set(&mut j2, "a=1", "::bad");
    assert_eq!(nav(&j2, "https://example.com/", None), None);
}

#[test]
fn cookies_serialize_longest_path_first_then_by_age() {
    let j = jar(&[
        ("a=1; Path=/", "https://x.com/app"),
        ("b=2; Path=/app", "https://x.com/app"),
        ("c=3; Path=/app", "https://x.com/app"),
    ]);
    // /app (len 4) before / (len 1); the two /app cookies keep set order.
    assert_eq!(
        nav(&j, "https://x.com/app", None).as_deref(),
        Some("b=2; c=3; a=1")
    );
}

#[test]
fn document_cookie_omits_http_only_but_the_wire_carries_it() {
    let j = jar(&[
        ("a=1", "https://example.com/"),
        ("h=secret; HttpOnly", "https://example.com/"),
    ]);
    // JS sees only the non-HttpOnly cookie.
    assert_eq!(j.document_cookie("https://example.com/"), "a=1");
    // The wire request carries both — HttpOnly is a JS-visibility rule, not a
    // send rule.
    assert_eq!(
        nav(&j, "https://example.com/", None).as_deref(),
        Some("a=1; h=secret")
    );
    // A bad page URL and an empty jar both read as the empty string.
    assert_eq!(j.document_cookie("::bad"), "");
    assert_eq!(CookieJar::new().document_cookie("https://example.com/"), "");
}

#[test]
fn a_reset_replaces_and_a_past_expiry_deletes() {
    let mut j = CookieJar::new();
    set(&mut j, "a=1", "https://x.com/");
    set(&mut j, "a=2", "https://x.com/"); // same (name,domain,path): replaces
    assert_eq!(nav(&j, "https://x.com/", None).as_deref(), Some("a=2"));
    // A future Max-Age keeps it; Max-Age=0 deletes it.
    set(&mut j, "b=1; Max-Age=3600", "https://x.com/");
    assert!(nav(&j, "https://x.com/", None).unwrap().contains("b=1"));
    set(&mut j, "a=; Max-Age=0", "https://x.com/");
    assert_eq!(nav(&j, "https://x.com/", None).as_deref(), Some("b=1"));
}

#[test]
fn store_skips_non_set_cookie_headers() {
    let mut j = CookieJar::new();
    j.store(
        &[
            ("Content-Type".to_string(), "text/html".to_string()),
            ("Set-Cookie".to_string(), "a=1".to_string()),
        ],
        "https://x.com/",
    );
    assert_eq!(nav(&j, "https://x.com/", None).as_deref(), Some("a=1"));
}

#[test]
fn write_script_ignores_a_nameless_write_and_a_bad_page_url() {
    let mut j = CookieJar::new();
    j.write_script("=novalue", "https://x.com/");
    j.write_script("a=1", "::bad");
    // A hostless page URL parses but has no host, so the write is dropped.
    j.write_script("a=1", "file:///x");
    assert_eq!(nav(&j, "https://x.com/", None), None);
}
