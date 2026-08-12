//! Unit tests for the fetch *primitives* — URL validation, the body-size gate,
//! same-origin comparison, header lookup, and redirect-target resolution. The
//! transport itself (h2/h1 handshake and the error taxonomy) is in
//! `transport/tests.rs`; the redirect *loop*, `file://`, and header/nav policy
//! are exercised through the [`super::FetchSession`] seam in `session/tests.rs`.

use super::*;

#[test]
fn header_value_is_case_insensitive() {
    let h = vec![
        ("Content-Type".to_string(), "text/html".to_string()),
        ("Content-Length".to_string(), "42".to_string()),
    ];
    assert_eq!(header_value(&h, "content-type"), Some("text/html".into()));
    assert_eq!(header_value(&h, "CONTENT-LENGTH"), Some("42".into()));
    assert_eq!(header_value(&h, "missing"), None);
}

#[test]
fn validate_url_rejects_garbage() {
    let err = validate_url("not a url").unwrap_err();
    assert_eq!(err.kind, kinds::FETCH_URL);
}

#[test]
fn validate_url_rejects_unsupported_scheme() {
    let err = validate_url("ftp://example.com/x").unwrap_err();
    assert_eq!(err.kind, kinds::FETCH_URL);
}

#[test]
fn validate_url_accepts_http_https_and_file() {
    assert!(validate_url("http://example.com/").is_ok());
    assert!(validate_url("https://example.com/").is_ok());
    assert!(validate_url("file:///tmp/x.html").is_ok());
}

#[test]
fn same_origin_compares_scheme_host_and_port() {
    assert!(same_origin("http://a.example/x", "http://a.example/y?z"));
    assert!(same_origin("https://a.example:443/", "https://a.example/p"));
    assert!(!same_origin("http://a.example/", "https://a.example/"));
    assert!(!same_origin("http://a.example/", "http://b.example/"));
    assert!(!same_origin("http://a.example:81/", "http://a.example/"));
    assert!(!same_origin("not a url", "http://a.example/"));
}

#[test]
fn body_len_gate_accepts_at_limit_and_rejects_over() {
    assert!(check_body_len(MAX_BODY_BYTES).is_ok());
    let e = check_body_len(MAX_BODY_BYTES + 1).unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_BODY);
    assert!(e.message.contains("limit"));
}

#[test]
fn user_agent_defaults_to_the_persona_and_a_caller_overrides() {
    // The default is derived from the pinned persona (Firefox 153esr), not a
    // stale literal; a `-H` override wins verbatim.
    assert_eq!(user_agent(&[]), FIREFOX_153_ESR.user_agent());
    assert_eq!(
        user_agent(&[]),
        "Mozilla/5.0 (X11; Linux x86_64; rv:153.0) Gecko/20100101 Firefox/153.0"
    );
    let h = vec![("User-Agent".to_string(), "custom/1".to_string())];
    assert_eq!(user_agent(&h), "custom/1");
}

#[test]
fn accept_language_defaults_to_the_persona_and_a_caller_overrides() {
    // The JS `navigator.language(s)` derive from this same string (bl-3972), so
    // the default matches the profile's derived header and a `-H` override wins.
    assert_eq!(accept_language(&[]), FIREFOX_153_ESR.accept_language());
    assert_eq!(accept_language(&[]), "en-US,en;q=0.9");
    let h = vec![("Accept-Language".to_string(), "fr-CA,fr;q=0.8".to_string())];
    assert_eq!(accept_language(&h), "fr-CA,fr;q=0.8");
}

#[test]
fn a_non_redirect_status_has_no_target() {
    assert_eq!(
        redirect_target(200, &[], "http://a.example/").unwrap(),
        None
    );
}

#[test]
fn a_redirect_resolves_its_location_against_the_current_url() {
    let h = vec![("Location".to_string(), "/landed".to_string())];
    assert_eq!(
        redirect_target(301, &h, "http://a.example/start").unwrap(),
        Some("http://a.example/landed".to_string())
    );
}

#[test]
fn a_redirect_without_a_location_is_a_redirect_error() {
    let e = redirect_target(302, &[], "http://a.example/").unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_REDIRECT);
}

#[test]
fn a_redirect_to_an_unparseable_location_is_a_redirect_error() {
    let h = vec![("location".to_string(), "http://".to_string())];
    let e = redirect_target(307, &h, "http://a.example/").unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_REDIRECT);
}

#[test]
fn the_validated_url_is_the_canonical_spelling_every_hop_uses() {
    // bl-2832: `dispatch` starts `current` — and therefore `final`, the request
    // target, and the cookie/referrer decisions — from this serialization, so
    // these are the rules the envelope's `url.final` reports. They are the
    // browser's: Chrome's `location.href` for each input below is the value on
    // the right.
    let canon = |u: &str| validate_url(u).expect("a valid URL").to_string();
    // A host with no path *has* the root path.
    assert_eq!(canon("https://example.com"), "https://example.com/");
    // A default port is not part of the URL; a non-default one is.
    assert_eq!(canon("https://example.com:443/x"), "https://example.com/x");
    assert_eq!(canon("http://example.com:80/x"), "http://example.com/x");
    assert_eq!(
        canon("https://example.com:8443/x"),
        "https://example.com:8443/x"
    );
    // Scheme and host case-fold; the path does not (it is server-significant).
    assert_eq!(
        canon("HTTPS://EXAMPLE.COM/Path"),
        "https://example.com/Path"
    );
    // An IDNA host serializes as the punycode actually sent on the wire.
    assert_eq!(
        canon("https://bücher.example/"),
        "https://xn--bcher-kva.example/"
    );
    // Escapes are preserved, never decoded (`%2F` is not `/`); an unescaped
    // space is escaped rather than passed through.
    assert_eq!(
        canon("https://example.com/a%2Fb"),
        "https://example.com/a%2Fb"
    );
    assert_eq!(
        canon("https://example.com/a b"),
        "https://example.com/a%20b"
    );
    // Query and fragment ride through — `location.href` keeps both.
    assert_eq!(
        canon("https://example.com?q=1#frag"),
        "https://example.com/?q=1#frag"
    );
}
