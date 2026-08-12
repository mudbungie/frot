//! Media-type disposition end-to-end (bl-0c3e): a response the one parser
//! cannot read is refused before the parse, for every view, with the `http`
//! block intact and no fabricated `out`. The policy per media type is unit-
//! pinned in `fetch::media::tests`; this module pins the envelope consequence
//! and the two field cases the bug report named — real PNG magic under a real
//! `image/png`, and binary bytes that happen to contain angle brackets.

use super::*;

fn run_capture(args: &[&str]) -> (u8, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let code = run_io(&argv, &mut out, &mut err);
    (code, String::from_utf8(out).unwrap())
}

fn parse_envelope(s: &str) -> Value {
    serde_json::from_str(s.trim()).expect("envelope JSON")
}

/// The first bytes of the Google logo the bug report fetched: the 8-byte PNG
/// signature followed by an IHDR chunk header.
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR\x00\x00\x01\x10\x00\x00\x00\x5c";

#[test]
fn png_response_is_refused_instead_of_parsed_as_html() {
    // The bug: `frot <google logo>.png --out text` returned `status: ok` with
    // 5,536 characters of replacement glyphs and chunk data as the impression.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "image/png")
        .with_body(PNG)
        .create();
    let (code, out) = run_capture(&[&server.url(), "--out", "text"]);
    assert_eq!(code, 1);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["kind"], "parse");
    assert_eq!(
        v["error"]["message"],
        "response media type image/png is not a document"
    );
    assert!(v.get("out").is_none(), "no fabricated impression payload");
    // The transport fact is still surfaced — the refusal's own evidence.
    assert_eq!(v["http"]["status"], 200);
    let ct = v["http"]["headers"]
        .as_array()
        .expect("http.headers")
        .iter()
        .any(|h| h["name"] == "content-type" && h["value"] == "image/png");
    assert!(ct, "the declaring header is surfaced: {}", v["http"]);
}

#[test]
fn refusal_never_quotes_the_body() {
    // Diagnostics are bounded to the media type: no chunk name, no replacement
    // glyph, no byte of the body may ride out in the error text.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "image/png")
        .with_body([PNG, b"tEXtComment\x00secret\xff\xfe"].concat())
        .create();
    let (_code, out) = run_capture(&[&server.url(), "--out", "dom"]);
    let message = parse_envelope(&out)["error"]["message"]
        .as_str()
        .expect("message")
        .to_string();
    assert_eq!(message, "response media type image/png is not a document");
    assert!(!message.contains("IHDR") && !message.contains("secret"));
}

#[test]
fn binary_body_with_accidental_angle_brackets_is_still_refused() {
    // Compressed bytes routinely contain `<` and `>`. The declaration decides,
    // not the bytes — frot never sniffs for markup (the `nosniff` ceiling), so
    // an accidental tag-looking run cannot buy a parse.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "application/octet-stream")
        .with_body(b"\x00\x01<html><body>not markup</body></html>\xff\x00")
        .create();
    let (code, out) = run_capture(&[&server.url(), "--out", "dom"]);
    assert_eq!(code, 1);
    let v = parse_envelope(&out);
    assert_eq!(v["error"]["kind"], "parse");
    assert_eq!(
        v["error"]["message"],
        "response media type application/octet-stream is not a document"
    );
}

#[test]
fn refusal_is_view_independent() {
    // Like the >= 400 flip beside it, the disposition is decided pre-parse, so
    // no view can produce a payload out of a body no view could read.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "application/pdf")
        .with_body(b"%PDF-1.7\n1 0 obj\n")
        .expect_at_least(1)
        .create();
    for view in ["dom", "text", "ax", "links", "forms", "bboxes", "meta"] {
        let (code, out) = run_capture(&[&server.url(), "--out", view]);
        assert_eq!(code, 1, "{view}");
        let v = parse_envelope(&out);
        assert_eq!(v["status"], "error", "{view}");
        assert_eq!(v["error"]["kind"], "parse", "{view}");
        assert!(v.get("out").is_none(), "{view}");
    }
}

#[test]
fn declared_html_is_parsed_whatever_the_url_says() {
    // The URL extension is never consulted: a page served as `text/html` from
    // a `.png` path is a page. (The inverse — a real PNG under `image/png` —
    // is refused above; the server's word decides both.)
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/logo.png")
        .with_status(200)
        .with_header("Content-Type", "text/html; charset=utf-8")
        .with_body("<p>a page at a png path</p>")
        .create();
    let (code, out) = run_capture(&[&format!("{}/logo.png", server.url()), "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["out"], "a page at a png path");
}

#[test]
fn textual_types_still_produce_impressions() {
    // The refusal is narrow: text stays a document, whatever its subtype.
    for (ct, body, want) in [
        ("text/plain", "just words", "just words"),
        ("application/json", "{\"a\": 1}", "{\"a\": 1}"),
    ] {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("Content-Type", ct)
            .with_body(body)
            .create();
        let (code, out) = run_capture(&[&server.url(), "--out", "text"]);
        assert_eq!(code, 0, "{ct}");
        let v = parse_envelope(&out);
        assert_eq!(v["status"], "ok", "{ct}");
        assert_eq!(v["out"], want, "{ct}");
    }
}

#[test]
fn svg_is_markup_and_keeps_its_structure() {
    // SVG proves the rule is the media type's *shape*, not its top-level
    // family: `image/svg+xml` is an `image/*` that is markup, so it is
    // impressed rather than refused. (`--out text` skips `<svg>` subtrees by
    // its own denoising policy, so the structural view is what shows it.)
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "image/svg+xml")
        .with_body("<svg><circle r='4'></circle></svg>")
        .create();
    let (code, out) = run_capture(&[&server.url(), "--out", "dom"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "ok");
    assert!(
        out.contains("\"circle\""),
        "the SVG structure survives: {}",
        v["out"]
    );
}

#[test]
fn a_response_that_declares_nothing_is_a_document() {
    // No `Content-Type` at all (a header-less response, and every `file://`
    // read) declares nothing, so the body is parsed as before: a malformed or
    // absent declaration must never turn a real page into an error.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Content-Type", "not-a-media-type")
        .with_body("<p>still a page</p>")
        .create();
    let (code, out) = run_capture(&[&server.url(), "--out", "text"]);
    assert_eq!(code, 0);
    assert_eq!(parse_envelope(&out)["out"], "still a page");
}
