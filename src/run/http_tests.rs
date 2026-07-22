//! HTTP-status surfacing: every network response carries an additive `http`
//! block, and a server error (status >= 400) flips the envelope to `error`
//! (kind `http.<code>`, exit 1) before the body is ever parsed. The
//! bot-defence verdict family that reads the same capture — declared
//! challenges and their negative controls — lives next door in
//! `challenge_tests`.

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
fn accepted_non_200_2xx_stays_ok_and_surfaces_http_status() {
    // Field trial (bl-84ef): amazon.com answered HTTP 202 with a soft-bot-block
    // challenge body. Only status >= 400 flips to `error` (bl-d1c6), so a 2xx
    // that is not 200 must stay `ok`, keep exit 0, and still surface
    // `http.status` — the sole signal that lets a caller tell a soft block from
    // a genuinely empty page. This test fails if 202 ever flips to error or if
    // the `http` block stops being emitted. The >= 400 error path is pinned by
    // `not_found_flips_to_error_with_http_block` and
    // `forbidden_challenge_flips_to_error_with_distinct_kind` below.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(202)
        .with_body("<html><body>challenge</body></html>")
        .create();
    let (code, out, _) = run_capture(&[&server.url(), "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["http"]["status"], 202);
    assert!(
        v.get("error").is_none(),
        "a 2xx response is an impression, not an error"
    );
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

fn headers_of(v: &Value) -> Vec<(String, String)> {
    v["http"]["headers"]
        .as_array()
        .expect("http.headers is always present on a network response")
        .iter()
        .map(|h| {
            (
                h["name"].as_str().unwrap().to_string(),
                h["value"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[test]
fn ok_response_surfaces_allowlisted_headers_and_strips_set_cookie() {
    // The additive `http.headers`: the allowlisted response headers ride every
    // network response, in wire order; `set-cookie` (session material, owned by
    // the cookie jar) and volatile headers (`date`) never appear.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("server", "snooserv")
        .with_header("set-cookie", "sid=secret; HttpOnly")
        .with_header("date", "Mon, 20 Jul 2026 00:00:00 GMT")
        .with_header("content-type", "text/html")
        .with_body("<p>hi</p>")
        .create();
    let (code, out, _) = run_capture(&[&server.url(), "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    let hs = headers_of(&v);
    assert!(hs.contains(&("server".into(), "snooserv".into())), "{hs:?}");
    assert!(
        hs.iter().any(|(n, _)| n == "content-type"),
        "content-type surfaced: {hs:?}"
    );
    assert!(
        hs.iter().all(|(n, _)| n != "set-cookie"),
        "set-cookie must never surface: {hs:?}"
    );
    assert!(
        hs.iter().all(|(n, _)| n != "date"),
        "volatile date must not surface: {hs:?}"
    );
}
