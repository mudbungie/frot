//! Charset selection: declared, sniffed, and fallback.

use super::*;

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
    assert_eq!(
        extract_charset_from_content_type("text/html; charset="),
        None
    );
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
fn inflate_gunzips_a_gzip_body() {
    use std::io::Write as _;
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(b"hello gzip").unwrap();
    let gz = enc.finish().unwrap();
    let h = vec![("Content-Encoding".to_string(), "gzip".to_string())];
    assert_eq!(inflate(&h, gz).unwrap(), b"hello gzip");
}

#[test]
fn inflate_unbrotlis_a_br_body() {
    use std::io::Write as _;
    let mut out = Vec::new();
    {
        let mut w = brotli::CompressorWriter::new(&mut out, 4096, 5, 22);
        w.write_all(b"hello brotli").unwrap();
    }
    let h = vec![("content-encoding".to_string(), "br".to_string())];
    assert_eq!(inflate(&h, out).unwrap(), b"hello brotli");
}

#[test]
fn inflate_passes_identity_and_unadvertised_encodings_through() {
    assert_eq!(inflate(&[], b"raw".to_vec()).unwrap(), b"raw");
    let h = vec![("content-encoding".to_string(), "zstd".to_string())];
    assert_eq!(inflate(&h, b"raw".to_vec()).unwrap(), b"raw");
}

#[test]
fn inflate_reports_corrupt_gzip_as_a_body_error() {
    let h = vec![("content-encoding".to_string(), "gzip".to_string())];
    let e = inflate(&h, b"this is not gzip".to_vec()).unwrap_err();
    assert_eq!(e.kind, crate::envelope::kinds::FETCH_BODY);
}
