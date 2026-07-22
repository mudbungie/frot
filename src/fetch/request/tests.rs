//! Unit tests for the request derivation: the per-intent ordered header set, the
//! URL-fact `Sec-Fetch-Site`/`Referer` policy, and the same-origin caller scope.
//! The wire serialization (h1 and h2 byte order) is the recorder's (`recorder.rs`).

use super::*;

/// The names of a derived set, in order — the axis the serializers preserve.
fn names(h: &[(String, String)]) -> Vec<String> {
    h.iter().map(|(n, _)| n.clone()).collect()
}

fn value<'a>(h: &'a [(String, String)], name: &str) -> Option<&'a str> {
    h.iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

const PAGE: &str = "https://site.example/page";

#[test]
fn a_navigation_carries_the_document_set_in_firefox_order() {
    let h = derive_headers(Intent::Navigation, PAGE, PAGE, None, &[], None);
    assert_eq!(
        names(&h),
        [
            "User-Agent",
            "Accept",
            "Accept-Language",
            "Accept-Encoding",
            "Upgrade-Insecure-Requests",
            "Sec-Fetch-Dest",
            "Sec-Fetch-Mode",
            "Sec-Fetch-Site",
            "Sec-Fetch-User",
            "Priority",
            "TE",
        ]
    );
    assert_eq!(
        value(&h, "Accept"),
        Some("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
    );
    assert_eq!(value(&h, "Accept-Encoding"), Some("gzip, br"));
    assert_eq!(value(&h, "Sec-Fetch-Dest"), Some("document"));
    assert_eq!(value(&h, "Sec-Fetch-Mode"), Some("navigate"));
    assert_eq!(value(&h, "Sec-Fetch-Site"), Some("none"));
    assert_eq!(value(&h, "Sec-Fetch-User"), Some("?1"));
    assert_eq!(value(&h, "Priority"), Some("u=0, i"));
    assert_eq!(value(&h, "TE"), Some("trailers"));
    // A navigation has no referrer source, so no Referer line.
    assert_eq!(value(&h, "Referer"), None);
}

#[test]
fn each_subresource_intent_gets_its_own_dest_mode_accept_priority() {
    let table = [
        (
            Intent::Style,
            "style",
            "no-cors",
            "text/css,*/*;q=0.1",
            "u=2",
        ),
        (Intent::ClassicScript, "script", "no-cors", "*/*", "u=2"),
        (Intent::Module, "script", "cors", "*/*", "u=2"),
        (Intent::FetchXhr, "empty", "cors", "*/*", "u=4"),
    ];
    for (intent, dest, mode, accept, prio) in table {
        let h = derive_headers(intent, PAGE, PAGE, Some(PAGE), &[], None);
        assert_eq!(value(&h, "Sec-Fetch-Dest"), Some(dest));
        assert_eq!(value(&h, "Sec-Fetch-Mode"), Some(mode));
        assert_eq!(value(&h, "Accept"), Some(accept));
        assert_eq!(value(&h, "Priority"), Some(prio));
        // No navigation-only headers ride a subresource.
        assert_eq!(value(&h, "Upgrade-Insecure-Requests"), None);
        assert_eq!(value(&h, "Sec-Fetch-User"), None);
    }
}

#[test]
fn sec_fetch_site_is_a_fact_of_the_two_urls() {
    // Same registrable domain, different subdomain → same-site (not same-origin).
    let sub = derive_headers(
        Intent::Style,
        "https://cdn.site.example/a.css",
        "https://cdn.site.example/a.css",
        Some(PAGE),
        &[],
        None,
    );
    assert_eq!(value(&sub, "Sec-Fetch-Site"), Some("same-site"));
    // Same origin exactly.
    let same = derive_headers(
        Intent::Style,
        "https://site.example/a.css",
        "https://site.example/a.css",
        Some(PAGE),
        &[],
        None,
    );
    assert_eq!(value(&same, "Sec-Fetch-Site"), Some("same-origin"));
    // Different registrable domain → cross-site.
    let cross = derive_headers(
        Intent::Style,
        "https://other.test/a.css",
        "https://other.test/a.css",
        Some(PAGE),
        &[],
        None,
    );
    assert_eq!(value(&cross, "Sec-Fetch-Site"), Some("cross-site"));
}

#[test]
fn referer_follows_strict_origin_when_cross_origin() {
    // Same-origin: the full source URL, fragment and userinfo stripped.
    assert_eq!(
        referer(
            Some("https://u:p@site.example/page#frag"),
            "https://site.example/a"
        ),
        Some("https://site.example/page".to_string())
    );
    // Cross-origin (same-site or cross-site): source origin only, trailing slash.
    assert_eq!(
        referer(Some(PAGE), "https://cdn.site.example/a.css"),
        Some("https://site.example/".to_string())
    );
    // https → http downgrade: no referer at all.
    assert_eq!(referer(Some(PAGE), "http://site.example/a"), None);
    // No referrer source (a user navigation): no referer.
    assert_eq!(referer(None, PAGE), None);
    // Unparseable source or target yields none rather than a bad header.
    assert_eq!(referer(Some("::nope"), PAGE), None);
    assert_eq!(referer(Some(PAGE), "::nope"), None);
}

#[test]
fn same_site_needs_matching_scheme_and_keeps_ips_whole() {
    // Scheme mismatch is not same-site even at the same host.
    assert!(!same_site(
        "https://site.example/",
        "http://cdn.site.example/"
    ));
    // Two distinct IP literals are never same-site (no label splitting).
    assert!(!same_site("http://127.0.0.1/", "http://10.0.0.1/"));
    // Unparseable input is never same-site.
    assert!(!same_site("::bad", "https://site.example/"));
}

#[test]
fn registrable_reduces_a_domain_and_keeps_an_ip_or_hostless_url_whole() {
    let dom = Url::parse("https://a.b.site.example/").unwrap();
    assert_eq!(registrable(&dom), "site.example");
    let ip = Url::parse("https://127.0.0.1/").unwrap();
    assert_eq!(registrable(&ip), "127.0.0.1");
    // A scheme with no host (e.g. `data:`) has no registrable domain.
    let hostless = Url::parse("data:text/plain,hi").unwrap();
    assert_eq!(registrable(&hostless), "");
}

#[test]
fn a_caller_override_is_final_case_insensitive_and_never_duplicates() {
    let caller = vec![
        ("accept".to_string(), "application/json".to_string()),
        ("X-Frot".to_string(), "1".to_string()),
    ];
    let h = derive_headers(Intent::Navigation, PAGE, PAGE, None, &caller, None);
    // The derived Accept is replaced in place (one line, canonical name kept).
    assert_eq!(value(&h, "Accept"), Some("application/json"));
    assert_eq!(
        h.iter()
            .filter(|(n, _)| n.eq_ignore_ascii_case("accept"))
            .count(),
        1
    );
    // A brand-new caller header is appended.
    assert_eq!(value(&h, "X-Frot"), Some("1"));
}

#[test]
fn caller_headers_do_not_leak_cross_origin() {
    // Anchor is the page; a cross-origin target (a redirect hop) carries none of
    // the caller set — Authorization included.
    let caller = vec![("Authorization".to_string(), "secret".to_string())];
    let same = derive_headers(Intent::Navigation, PAGE, PAGE, None, &caller, None);
    assert_eq!(value(&same, "Authorization"), Some("secret"));
    let cross = derive_headers(
        Intent::Navigation,
        "https://evil.test/",
        PAGE,
        None,
        &caller,
        None,
    );
    assert_eq!(value(&cross, "Authorization"), None);
}
