//! Unit tests for the fetch *primitives* — URL validation, the body-size gate,
//! same-origin comparison, header lookup, and the ureq→taxonomy error mapping.
//! The transport itself (the shared pool, redirects, `file://`, header/nav
//! policy) is exercised against a live loopback server in `session/tests.rs`,
//! where the [`super::FetchSession`] seam it flows through lives.

use super::*;
use ureq::Error as UE;

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
fn user_agent_defaults_and_overrides() {
    assert_eq!(user_agent(&[]), USER_AGENT);
    let h = vec![("User-Agent".to_string(), "custom/1".to_string())];
    assert_eq!(user_agent(&h), "custom/1");
}

#[test]
fn map_dns_error() {
    let e = map_ureq_error(UE::HostNotFound);
    assert_eq!(e.kind, kinds::FETCH_DNS);
}

#[test]
fn map_connection_failed() {
    let e = map_ureq_error(UE::ConnectionFailed);
    assert_eq!(e.kind, kinds::FETCH_CONNECT);
}

#[test]
fn map_timeout() {
    let e = map_ureq_error(UE::Timeout(ureq::Timeout::Global));
    assert_eq!(e.kind, kinds::FETCH_TIMEOUT);
}

#[test]
fn map_too_many_redirects() {
    let e = map_ureq_error(UE::TooManyRedirects);
    assert_eq!(e.kind, kinds::FETCH_REDIRECT);
}

#[test]
fn map_redirect_failed() {
    let e = map_ureq_error(UE::RedirectFailed);
    assert_eq!(e.kind, kinds::FETCH_REDIRECT);
}

#[test]
fn map_bad_uri_is_fetch_url() {
    let e = map_ureq_error(UE::BadUri("bad".to_string()));
    assert_eq!(e.kind, kinds::FETCH_URL);
}

#[test]
fn map_require_https_only_is_fetch_url() {
    let e = map_ureq_error(UE::RequireHttpsOnly("http only".to_string()));
    assert_eq!(e.kind, kinds::FETCH_URL);
}

#[test]
fn map_tls_static_str() {
    let e = map_ureq_error(UE::Tls("bad cert"));
    assert_eq!(e.kind, kinds::FETCH_TLS);
}

#[test]
fn map_io_error_is_fetch_body() {
    let e = map_ureq_error(UE::Io(std::io::Error::other("read")));
    assert_eq!(e.kind, kinds::FETCH_BODY);
}

#[test]
fn map_body_exceeds_is_fetch_body() {
    let e = map_ureq_error(UE::BodyExceedsLimit(1));
    assert_eq!(e.kind, kinds::FETCH_BODY);
}

#[test]
fn map_body_stalled_is_fetch_body() {
    let e = map_ureq_error(UE::BodyStalled);
    assert_eq!(e.kind, kinds::FETCH_BODY);
}

#[test]
fn map_decompress_is_fetch_body() {
    let e = map_ureq_error(UE::Decompress("gzip", std::io::Error::other("decompress")));
    assert_eq!(e.kind, kinds::FETCH_BODY);
}

#[test]
fn map_unknown_variant_falls_through_to_internal() {
    let e = map_ureq_error(UE::Other(Box::new(std::io::Error::other("misc"))));
    assert_eq!(e.kind, kinds::INTERNAL);
}
