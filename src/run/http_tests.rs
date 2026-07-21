//! HTTP-status surfacing: every network response carries an additive `http`
//! block, and a server error (status >= 400) flips the envelope to `error`
//! (kind `http.<code>`, exit 1) before the body is ever parsed. A
//! server-declared challenge at a success status (needs.md §3) likewise
//! flips pre-parse, to `needs:["human"]` (exit 0).

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

#[test]
fn retry_after_on_200_flips_to_needs_human() {
    // Field trial (bl-6d75): reddit's bot-challenge interstitial answers
    // HTTP 200 + `retry-after: 0` with a body that has a title and text — a
    // stand-in the server itself declared (needs.md §3). It must never be
    // reported as an `ok` impression of the page.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Retry-After", "0")
        .with_body("<html><head><title>Please wait</title></head><body><main>verifying</main></body></html>")
        .create();
    let (code, out, _) = run_capture(&[&server.url(), "--out", "text"]);
    assert_eq!(code, 0, "needs is not an error");
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "needs");
    assert_eq!(v["needs"], serde_json::json!(["human"]));
    assert_eq!(v["http"]["status"], 200);
    assert!(v.get("out").is_none(), "no impression of a placeholder");
    assert!(v.get("error").is_none(), "nothing failed");
}

#[test]
fn cf_mitigated_challenge_flips_to_needs_human() {
    // Cloudflare declares a challenge response with `cf-mitigated:
    // challenge` (needs.md §3); at a status < 400 that declaration — not the
    // body — flips the envelope.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("CF-Mitigated", "challenge")
        .with_body("<html><body>Checking your browser</body></html>")
        .create();
    let (code, out, _) = run_capture(&[&server.url(), "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "needs");
    assert_eq!(v["needs"], serde_json::json!(["human"]));
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

#[test]
fn challenge_header_that_fired_needs_is_the_one_surfaced() {
    // One capture, two consumers (needs.md §3): the exact header that made
    // `needs::challenge` fire (`retry-after`) is the header `http.headers`
    // surfaces — proof they derive from the same capture and cannot disagree.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("retry-after", "0")
        .with_header("server", "snooserv")
        .with_body("<html><body><main>verifying</main></body></html>")
        .create();
    let (_c, out, _) = run_capture(&[&server.url(), "--out", "text"]);
    let v = parse_envelope(&out);
    assert_eq!(v["needs"], serde_json::json!(["human"]));
    let hs = headers_of(&v);
    assert!(
        hs.contains(&("retry-after".into(), "0".into())),
        "the declaring header is surfaced: {hs:?}"
    );
}

#[test]
fn bot_defence_403_is_distinguishable_from_a_genuine_403() {
    // Field corpus (identity.md §3.7): g2's bot-defence 403 carried
    // `x-datadome: protected` + `server: cloudflare`; a genuine origin 403 does
    // not. Both flip to `error{http.403}`, but `http.headers` now carries the
    // vendor markers, so a caller tells a refusal from a real failure.
    let mut defence = mockito::Server::new();
    let _d = defence
        .mock("GET", "/")
        .with_status(403)
        .with_header("x-datadome", "protected")
        .with_header("server", "cloudflare")
        .with_body("<html>blocked</html>")
        .create();
    let (_c, out, _) = run_capture(&[&defence.url(), "--out", "text"]);
    let v = parse_envelope(&out);
    assert_eq!(v["error"]["kind"], "http.403");
    let hs = headers_of(&v);
    assert!(
        hs.contains(&("x-datadome".into(), "protected".into())),
        "vendor marker distinguishes the refusal: {hs:?}"
    );

    let mut genuine = mockito::Server::new();
    let _g = genuine
        .mock("GET", "/")
        .with_status(403)
        .with_header("server", "nginx")
        .with_body("<h1>Forbidden</h1>")
        .create();
    let (_c, out, _) = run_capture(&[&genuine.url(), "--out", "text"]);
    let v = parse_envelope(&out);
    assert_eq!(v["error"]["kind"], "http.403");
    assert!(
        headers_of(&v).iter().all(|(n, _)| n != "x-datadome"),
        "a genuine 403 carries no bot-defence marker"
    );
}

#[test]
fn declared_challenge_flips_every_view_and_skips_js() {
    // The flip is pre-parse and view-independent, mirroring the >= 400 flip:
    // a `--out dom --js` of an interstitial still reports `needs:["human"]`,
    // with no `js` block — a declared challenge's scripts never run.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("retry-after", "5")
        .with_body("<html><body><script>window.solved=1;</script></body></html>")
        .create();
    let (code, out, _) = run_capture(&[&server.url(), "--js", "--out", "dom"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "needs");
    assert_eq!(v["needs"], serde_json::json!(["human"]));
    assert!(
        v.get("js").is_none(),
        "challenge scripts are never executed"
    );
}
