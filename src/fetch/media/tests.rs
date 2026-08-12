//! The media disposition policy, type by type (bl-0c3e). The browser-relevant
//! classes the ball enumerates are each pinned here; the end-to-end envelope
//! consequence (error kind, no `out`, `http` block intact) is pinned in
//! `run::media_tests`.

use super::*;

fn refused(ct: &str) -> Option<String> {
    non_document(Some(ct))
}

#[test]
fn html_and_plain_text_are_documents() {
    // text/* is the parser's home ground; text/plain is what a browser shows
    // as text and frot reads with the one parser it has (the declared ceiling).
    assert_eq!(refused("text/html"), None);
    assert_eq!(refused("text/html; charset=utf-8"), None);
    assert_eq!(refused("text/plain"), None);
    assert_eq!(refused("TEXT/HTML"), None);
    assert_eq!(refused(" text/html "), None);
}

#[test]
fn xml_family_and_svg_are_documents() {
    // XHTML/XML are markup, and the `+xml` structured suffix generalizes:
    // `image/svg+xml` is markup a browser happens to paint, so an impression
    // of its structure is honest — the top-level `image` type is not the fact.
    assert_eq!(refused("application/xhtml+xml"), None);
    assert_eq!(refused("application/xml"), None);
    assert_eq!(refused("text/xml"), None);
    assert_eq!(refused("image/svg+xml"), None);
}

#[test]
fn json_and_script_are_documents() {
    assert_eq!(refused("application/json"), None);
    assert_eq!(refused("application/ld+json"), None);
    assert_eq!(refused("application/javascript"), None);
    assert_eq!(refused("application/ecmascript"), None);
}

#[test]
fn binary_families_are_not_documents() {
    // Not an extension list and not a two-suffix patch: anything declared
    // outside the textual rule is refused, so a media type nobody enumerated
    // is refused too (`application/wasm` below stands in for that class).
    assert_eq!(refused("image/png"), Some("image/png".to_string()));
    assert_eq!(refused("image/jpeg"), Some("image/jpeg".to_string()));
    assert_eq!(
        refused("application/pdf"),
        Some("application/pdf".to_string())
    );
    assert_eq!(
        refused("application/octet-stream"),
        Some("application/octet-stream".to_string())
    );
    assert_eq!(refused("audio/mpeg"), Some("audio/mpeg".to_string()));
    assert_eq!(refused("video/mp4"), Some("video/mp4".to_string()));
    assert_eq!(refused("font/woff2"), Some("font/woff2".to_string()));
    assert_eq!(
        refused("application/wasm"),
        Some("application/wasm".to_string())
    );
}

#[test]
fn refusal_quotes_the_essence_only() {
    // The parameters (and a charset a server may have attached to a binary
    // type) never reach the diagnostic, and the essence is normalized.
    assert_eq!(
        refused("IMAGE/PNG; charset=binary"),
        Some("image/png".to_string())
    );
    assert_eq!(refused("image / png"), Some("image/png".to_string()));
}

#[test]
fn absent_or_shapeless_declaration_is_a_document() {
    // A `file://` read has no Content-Type at all, and a header that is not
    // `type/subtype` declares nothing — neither may turn a real page into an
    // error. This is the rule that keeps a malformed declaration cheap.
    assert_eq!(non_document(None), None);
    assert_eq!(refused("nonsense"), None);
    assert_eq!(refused(""), None);
    assert_eq!(refused("; charset=utf-8"), None);
}

#[test]
fn diagnostic_is_bounded_and_printable() {
    // A server can send any bytes in a header value. The quoted essence stays
    // short and printable so an error message is never a dumping ground —
    // control bytes and non-ASCII are dropped, and the length is capped.
    let noisy = "image/p\u{7}n\u{fffd}g\t";
    assert_eq!(non_document(Some(noisy)), Some("image/png".to_string()));
    let long = format!("image/{}", "n".repeat(200));
    let got = non_document(Some(&long)).expect("a long binary type is still refused");
    assert_eq!(got.len(), MAX_ESSENCE);
    assert!(got.starts_with("image/n"));
}
