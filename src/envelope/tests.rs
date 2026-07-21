use super::*;
use serde_json::json;

#[test]
fn view_parse_and_as_str_round_trip() {
    for (name, v) in View::ALL {
        assert_eq!(View::parse(name), Some(*v));
        assert_eq!(v.as_str(), *name);
    }
}

#[test]
fn view_parse_unknown_returns_none() {
    assert_eq!(View::parse("nope"), None);
    assert_eq!(View::parse(""), None);
}

#[test]
fn view_serde_uses_lowercase_strings() {
    for (name, v) in View::ALL {
        let s = serde_json::to_string(v).unwrap();
        assert_eq!(s, format!("\"{}\"", name));
        let back: View = serde_json::from_str(&s).unwrap();
        assert_eq!(back, *v);
    }
}

#[test]
fn needs_kind_serde() {
    assert_eq!(serde_json::to_string(&NeedsKind::Js).unwrap(), "\"js\"");
    assert_eq!(serde_json::to_string(&NeedsKind::Css).unwrap(), "\"css\"");
    let v: NeedsKind = serde_json::from_str("\"js\"").unwrap();
    assert_eq!(v, NeedsKind::Js);
    let v: NeedsKind = serde_json::from_str("\"css\"").unwrap();
    assert_eq!(v, NeedsKind::Css);
}

#[test]
fn status_kind_serde() {
    for k in [StatusKind::Ok, StatusKind::Needs, StatusKind::Error] {
        let s = serde_json::to_string(&k).unwrap();
        let back: StatusKind = serde_json::from_str(&s).unwrap();
        assert_eq!(back, k);
    }
}

#[test]
fn url_block_requested_only_skips_final() {
    let b = UrlBlock::requested("https://x/");
    let s = serde_json::to_string(&b).unwrap();
    assert!(s.contains("\"requested\":\"https://x/\""));
    assert!(!s.contains("\"final\""));
}

#[test]
fn url_block_resolved_emits_final() {
    let b = UrlBlock::resolved("https://x/", "https://x/y");
    let s = serde_json::to_string(&b).unwrap();
    assert!(s.contains("\"final\":\"https://x/y\""));
    let back: UrlBlock = serde_json::from_str(&s).unwrap();
    assert_eq!(back, b);
}

#[test]
fn url_block_default_final() {
    let b: UrlBlock = serde_json::from_str("{\"requested\":\"x\"}").unwrap();
    assert!(b.final_url.is_none());
}

#[test]
fn error_info_new_constructs() {
    let e = ErrorInfo::new(kinds::FETCH_TIMEOUT, "took too long");
    assert_eq!(e.kind, "fetch.timeout");
    assert_eq!(e.message, "took too long");
}

#[test]
fn envelope_ok_emits_out_no_needs_no_error() {
    let env = Envelope::ok(
        UrlBlock::resolved("https://x/", "https://x/y"),
        View::Text,
        json!("hello"),
    );
    let s = env.to_json_string();
    assert!(s.contains("\"frot\":\"0\""));
    assert!(s.contains("\"status\":\"ok\""));
    assert!(s.contains("\"view\":\"text\""));
    assert!(s.contains("\"out\":\"hello\""));
    assert!(!s.contains("\"needs\""));
    assert!(!s.contains("\"error\""));
}

#[test]
fn envelope_needs_emits_needs_array() {
    let env = Envelope::needs(
        UrlBlock::resolved("https://x/", "https://x/"),
        View::Ax,
        vec![NeedsKind::Js],
        None,
    );
    let s = env.to_json_string();
    assert!(s.contains("\"status\":\"needs\""));
    assert!(s.contains("\"needs\":[\"js\"]"));
    assert!(!s.contains("\"out\""));
}

#[test]
fn envelope_needs_with_partial_out() {
    let env = Envelope::needs(
        UrlBlock::resolved("https://x/", "https://x/"),
        View::Text,
        vec![NeedsKind::Js, NeedsKind::Css],
        Some(json!("partial")),
    );
    let s = env.to_json_string();
    assert!(s.contains("\"out\":\"partial\""));
    assert!(s.contains("\"needs\":[\"js\",\"css\"]"));
}

#[test]
fn envelope_error_emits_error_block() {
    let env = Envelope::error(
        UrlBlock::requested("https://x/"),
        View::Text,
        ErrorInfo::new(kinds::FETCH_DNS, "no such host"),
    );
    let s = env.to_json_string();
    assert!(s.contains("\"status\":\"error\""));
    assert!(s.contains("\"error\":{\"kind\":\"fetch.dns\""));
    assert!(s.contains("\"message\":\"no such host\""));
    assert!(!s.contains("\"out\""));
}

#[test]
fn http_info_new_and_serde_round_trip() {
    let h = HttpInfo::new(404, &[]);
    assert_eq!(h.status, 404);
    let s = serde_json::to_string(&h).unwrap();
    assert_eq!(s, "{\"status\":404,\"headers\":[]}");
    let back: HttpInfo = serde_json::from_str(&s).unwrap();
    assert_eq!(back, h);
}

#[test]
fn envelope_with_http_attaches_and_none_is_omitted() {
    let base = Envelope::ok(
        UrlBlock::resolved("https://x/", "https://x/"),
        View::Text,
        json!("hi"),
    );
    // None leaves the envelope without an http field.
    let s = base.clone().with_http(None).to_json_string();
    assert!(!s.contains("\"http\""));
    // Some attaches the block and survives a round trip.
    let s = base
        .with_http(Some(HttpInfo::new(200, &[])))
        .to_json_string();
    assert!(s.contains("\"http\":{\"status\":200,\"headers\":[]}"));
    let back: Envelope = serde_json::from_str(&s).unwrap();
    assert_eq!(back.http, Some(HttpInfo::new(200, &[])));
}

#[test]
fn envelope_round_trip_deserialize() {
    let env = Envelope::ok(
        UrlBlock::resolved("https://x/", "https://x/y"),
        View::Links,
        json!([{"href":"https://x/", "kind":"a"}]),
    );
    let s = env.to_json_string();
    let back: Envelope = serde_json::from_str(&s).unwrap();
    assert_eq!(back.status, StatusKind::Ok);
    assert_eq!(back.view, View::Links);
    assert_eq!(back.url.requested, "https://x/");
}

#[test]
fn kinds_constants_are_dotted_and_stable() {
    assert_eq!(kinds::USAGE, "usage");
    assert_eq!(kinds::FETCH_URL, "fetch.url");
    assert_eq!(kinds::FETCH_DNS, "fetch.dns");
    assert_eq!(kinds::FETCH_CONNECT, "fetch.connect");
    assert_eq!(kinds::FETCH_TLS, "fetch.tls");
    assert_eq!(kinds::FETCH_TIMEOUT, "fetch.timeout");
    assert_eq!(kinds::FETCH_REDIRECT, "fetch.redirect");
    assert_eq!(kinds::FETCH_BODY, "fetch.body");
    assert_eq!(kinds::FETCH_ENCODING, "fetch.encoding");
    assert_eq!(kinds::PARSE, "parse");
    assert_eq!(kinds::INTERNAL, "internal");
}
