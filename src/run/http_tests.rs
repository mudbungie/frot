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
fn surfaces_exactly_the_allowlist_and_nothing_else() {
    // The allowlist pinned end-to-end, not just at the constructor
    // (`envelope::http::tests`): a response carrying every `SURFACED` entry
    // plus session material and a per-request volatile must yield those six
    // and *only* those six. `set-cookie` is the cookie jar's alone (bl-6dad)
    // and a request id would make golden captures non-deterministic
    // (identity.md §12) — a leak of either is a regression this catches.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Retry-After", "0")
        .with_header("CF-Mitigated", "challenge")
        .with_header("x-amzn-waf-action", "challenge")
        .with_header("Server", "snooserv")
        .with_header("X-DataDome", "protected")
        .with_header("Content-Type", "text/html")
        .with_header("Set-Cookie", "sid=secret; HttpOnly")
        .with_header("X-Request-Id", "e3b0c442")
        .with_body("<html><body><p>hi</p></body></html>")
        .create();
    let (_c, out, _) = run_capture(&[&server.url(), "--out", "text"]);
    let v = parse_envelope(&out);
    // Compared as a multiset, not a sequence: the capture is hyper's
    // `HeaderMap::iter()`, whose order across *distinct* names is documented
    // as arbitrary, so a fixed sequence here would pin a hyper hash detail
    // rather than a frot contract. Order within one name is guaranteed and is
    // pinned by `repeated_header_keeps_wire_order` below; order preservation
    // through the filter itself is pinned in `envelope::http::tests`.
    let mut got = headers_of(&v);
    got.sort();
    let mut want: Vec<(String, String)> = [
        ("retry-after", "0"),
        ("cf-mitigated", "challenge"),
        ("x-amzn-waf-action", "challenge"),
        ("server", "snooserv"),
        ("x-datadome", "protected"),
        ("content-type", "text/html"),
    ]
    .iter()
    .map(|(n, x)| (n.to_string(), x.to_string()))
    .collect();
    want.sort();
    assert_eq!(got, want, "the allowlist, lower-cased, and nothing else");
}

#[test]
fn repeated_header_keeps_wire_order() {
    // One name sent twice: HTTP allows it, so `http.headers` is a list and not
    // a map (bl-acec). Both values survive, in the order the server sent them
    // — a map would silently collapse them to one.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("retry-after", "0")
        .with_header("retry-after", "120")
        .with_body("<html><body><p>hi</p></body></html>")
        .create();
    let (_c, out, _) = run_capture(&[&server.url(), "--out", "text"]);
    let v = parse_envelope(&out);
    let retries: Vec<String> = headers_of(&v)
        .into_iter()
        .filter(|(n, _)| n == "retry-after")
        .map(|(_, x)| x)
        .collect();
    assert_eq!(retries, vec!["0".to_string(), "120".to_string()]);
}
