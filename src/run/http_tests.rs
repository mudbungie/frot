//! HTTP-status surfacing: every network response carries an additive `http`
//! block, and a server error (status >= 400) flips the envelope to `error`
//! (kind `http.<code>`, exit 1) before the body is ever parsed.

use super::*;

fn run_capture(args: &[&str]) -> (u8, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let code = run_io(&argv, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn parse_envelope(s: &str) -> Value {
    serde_json::from_str(s.trim()).expect("envelope JSON")
}

#[test]
fn ok_response_carries_http_status_block() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_body("<p>hi</p>")
        .create();
    let (code, out, _) = run_capture(&[&server.url(), "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["http"]["status"], 200);
    assert_eq!(v["out"], "hi");
}

#[test]
fn not_found_flips_to_error_with_http_block() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(404)
        .with_body("<h1>Not Found</h1>")
        .create();
    let (code, out, _) = run_capture(&[&server.url(), "--out", "text"]);
    assert_eq!(code, 1);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["kind"], "http.404");
    assert_eq!(v["error"]["message"], "server returned HTTP 404");
    assert_eq!(v["http"]["status"], 404);
    assert!(
        v.get("out").is_none(),
        "no impression payload on a server error"
    );
}

#[test]
fn forbidden_challenge_flips_to_error_with_distinct_kind() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(403)
        .with_body("<html>Attention Required! | Cloudflare</html>")
        .create();
    let (code, out, _) = run_capture(&[&server.url(), "--out", "text"]);
    assert_eq!(code, 1);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["kind"], "http.403");
    assert_eq!(v["http"]["status"], 403);
}
