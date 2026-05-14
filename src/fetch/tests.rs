use super::*;
use ureq::Error as UE;

#[test]
fn extract_charset_finds_value() {
    assert_eq!(
        extract_charset_from_content_type("text/html; charset=utf-8"),
        Some("utf-8".to_string()),
    );
}

#[test]
fn extract_charset_strips_quotes() {
    assert_eq!(
        extract_charset_from_content_type("text/html; charset=\"utf-8\""),
        Some("utf-8".to_string()),
    );
}

#[test]
fn extract_charset_none_when_absent() {
    assert_eq!(extract_charset_from_content_type("text/html"), None);
}

#[test]
fn extract_charset_none_when_empty() {
    assert_eq!(extract_charset_from_content_type("text/html; charset="), None);
}

#[test]
fn decode_body_default_is_utf8() {
    let (s, c) = decode_body(b"hello", None);
    assert_eq!(s, "hello");
    assert_eq!(c, "utf-8");
}

#[test]
fn decode_body_uses_declared_charset() {
    let (s, c) = decode_body(b"hello", Some("text/html; charset=utf-8"));
    assert_eq!(s, "hello");
    assert_eq!(c, "utf-8");
}

#[test]
fn decode_body_iso_8859_1() {
    let (s, _) = decode_body(&[0xe9], Some("text/html; charset=iso-8859-1"));
    assert_eq!(s, "é");
}

#[test]
fn decode_body_unknown_charset_falls_back_to_utf8() {
    let (s, c) = decode_body(b"hello", Some("text/html; charset=fake-9000"));
    assert_eq!(s, "hello");
    assert_eq!(c, "utf-8");
}

#[test]
fn decode_body_sniffs_meta_charset_when_header_missing() {
    let html = b"<meta charset=iso-8859-1>hello";
    let (_, c) = decode_body(html, None);
    assert!(c.contains("8859") || c == "windows-1252");
}

#[test]
fn decode_body_sniffs_truncated_head_for_large_input() {
    let mut data = vec![b'x'; 2000];
    data.extend_from_slice(b"<meta charset=iso-8859-1>");
    let (_, c) = decode_body(&data, None);
    // beyond the sniff window, charset stays default
    assert_eq!(c, "utf-8");
}

#[test]
fn decode_body_sniff_empty_when_charset_eq_then_garbage() {
    let bytes = b"<meta charset=>";
    let (_, c) = decode_body(bytes, None);
    assert_eq!(c, "utf-8");
}

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
    let err = validate_url("file:///etc/passwd").unwrap_err();
    assert_eq!(err.kind, kinds::FETCH_URL);
}

#[test]
fn validate_url_accepts_http_and_https() {
    assert!(validate_url("http://example.com/").is_ok());
    assert!(validate_url("https://example.com/").is_ok());
}

#[test]
fn fetch_invalid_url_returns_url_error() {
    let err = fetch("not a url").unwrap_err();
    assert_eq!(err.kind, kinds::FETCH_URL);
}

#[test]
fn fetch_unsupported_scheme_returns_url_error() {
    let err = fetch("file:///etc/passwd").unwrap_err();
    assert_eq!(err.kind, kinds::FETCH_URL);
}

#[test]
fn fetch_ok_against_mock_server() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("content-type", "text/html; charset=utf-8")
        .with_body("hello")
        .create();
    let url = server.url();
    let r = fetch(&url).unwrap();
    assert_eq!(r.status, 200);
    assert_eq!(r.body, "hello");
    assert_eq!(r.charset, "utf-8");
    assert!(r.headers.iter().any(|(n, _)| n.eq_ignore_ascii_case("content-type")));
}

#[test]
fn fetch_404_returns_envelope_ok() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(404)
        .with_body("not found")
        .create();
    let url = server.url();
    let r = fetch(&url).unwrap();
    assert_eq!(r.status, 404);
    assert_eq!(r.body, "not found");
}

#[test]
fn fetch_follows_redirects_and_records_final_url() {
    let mut server = mockito::Server::new();
    let target_path = "/landed";
    let _m1 = server
        .mock("GET", "/start")
        .with_status(301)
        .with_header("location", target_path)
        .create();
    let _m2 = server
        .mock("GET", target_path)
        .with_status(200)
        .with_body("final")
        .create();
    let url = format!("{}/start", server.url());
    let r = fetch(&url).unwrap();
    assert_eq!(r.status, 200);
    assert_eq!(r.body, "final");
    assert!(r.final_url.ends_with("/landed"));
}

#[test]
fn fetch_to_dead_port_returns_connect_error() {
    // 127.0.0.1:1 should refuse on most systems.
    let err = fetch("http://127.0.0.1:1/").unwrap_err();
    assert!(
        err.kind == kinds::FETCH_CONNECT
            || err.kind == kinds::FETCH_TIMEOUT
            || err.kind == kinds::FETCH_BODY,
        "unexpected error kind: {} ({})",
        err.kind,
        err.message,
    );
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
fn map_rustls_error_is_fetch_tls() {
    let e = map_ureq_error(UE::Rustls(rustls::Error::General("rustls".to_string())));
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
    let e = map_ureq_error(UE::Decompress(
        "gzip",
        std::io::Error::other("decompress"),
    ));
    assert_eq!(e.kind, kinds::FETCH_BODY);
}

#[test]
fn map_unknown_variant_falls_through_to_internal() {
    let e = map_ureq_error(UE::Other(Box::new(std::io::Error::other("misc"))));
    assert_eq!(e.kind, kinds::INTERNAL);
}
