//! A challenge declared at a status of 400 or above (`docs/design/challenge.md`
//! §4.2, `bl-a7c5`): Cloudflare serves its managed challenge as `403` +
//! `cf-mitigated: challenge`, and a browser runs that page whatever the code.
//! So the declaration, not the status, opens the round trip — every verdict
//! corner is pinned here, beside the guard that a refusal which declares
//! nothing is byte for byte the error it always was.

use super::*;
use mockito::Matcher::Missing;

fn run_capture(args: &[&str]) -> (u8, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let code = run_io(&argv, &mut out, &mut err);
    (code, String::from_utf8(out).unwrap())
}

fn envelope(out: &str) -> Value {
    serde_json::from_str(out.trim()).expect("envelope JSON")
}

/// A Cloudflare-shaped challenge: `403`, the declaring header, and a body that
/// is nothing but the challenge `script`. Served to a request with no cookie.
fn challenge(server: &mut mockito::Server, script: &str) -> mockito::Mock {
    server
        .mock("GET", "/")
        .match_header("cookie", Missing)
        .with_status(403)
        .with_header("cf-mitigated", "challenge")
        .with_body(format!(
            "<body><p>Just a moment</p><script>{script}</script>"
        ))
        .expect(1)
        .create()
}

const MINT: &str = "document.cookie = 'cf_clearance=1; path=/';";

fn block(rounds: u8, passed: bool) -> Value {
    serde_json::json!({"declared": "cf-mitigated", "rounds": rounds, "passed": passed})
}

#[test]
fn without_js_it_needs_js_and_keeps_the_403() {
    let mut server = mockito::Server::new();
    let gate = challenge(&mut server, MINT);
    let (code, out) = run_capture(&[&server.url(), "--out", "text"]);
    gate.assert();
    let v = envelope(&out);
    assert_eq!(code, 0, "needs is not an error");
    assert_eq!(v["status"], "needs");
    assert_eq!(v["needs"], serde_json::json!(["js"]));
    assert_eq!(v["challenge"], block(0, false));
    // The honest http fact rides along: the status and the declaring header.
    assert_eq!(v["http"]["status"], 403);
    assert_eq!(v["http"]["headers"][0]["value"], "challenge", "{v}");
    assert!(v.get("error").is_none() && v.get("out").is_none(), "{v}");
    assert!(v.get("js").is_none(), "{v}");
}

#[test]
fn a_script_that_changes_nothing_is_an_honest_stop_after_one_request() {
    let mut server = mockito::Server::new();
    let gate = challenge(&mut server, "window.widget = 'click me';");
    let (code, out) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    gate.assert();
    let v = envelope(&out);
    assert_eq!(code, 0);
    assert_eq!(v["needs"], serde_json::json!(["human"]));
    assert_eq!(v["challenge"], block(0, false));
    assert_eq!(v["http"]["status"], 403);
    assert_eq!(v["js"]["scripts"], 1, "{v}");
    assert!(v.get("out").is_none(), "no impression of a placeholder");
}

#[test]
fn a_minted_cookie_earns_one_re_issue_and_the_page() {
    let mut server = mockito::Server::new();
    let gate = challenge(&mut server, MINT);
    let real = server
        .mock("GET", "/")
        .match_header("cookie", "cf_clearance=1")
        .with_body("<p>the real page</p>")
        .expect(1)
        .create();
    let (code, out) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    gate.assert();
    real.assert();
    let v = envelope(&out);
    assert_eq!(code, 0);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["out"], "the real page");
    assert_eq!(v["challenge"], block(1, true));
    assert_eq!(v["http"]["status"], 200);
}

#[test]
fn still_challenged_after_the_round_stops_at_two_requests() {
    let mut server = mockito::Server::new();
    let gate = server
        .mock("GET", "/")
        .with_status(403)
        .with_header("cf-mitigated", "challenge")
        .with_body("<script>document.cookie = 'try=' + Math.random();</script>")
        .expect(2)
        .create();
    let (code, out) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    gate.assert();
    let v = envelope(&out);
    assert_eq!(code, 0);
    assert_eq!(v["needs"], serde_json::json!(["human"]));
    assert_eq!(v["challenge"], block(1, false));
    assert_eq!(v["http"]["status"], 403);
    assert_eq!(
        v["js"]["scripts"], 1,
        "the second challenge is not run: {v}"
    );
}

#[test]
fn a_plain_refusal_after_the_round_is_the_error_it_is() {
    // The challenge was passed; then the server said no. A `403` that declares
    // nothing is `http.403`, after a round exactly as before one.
    let mut server = mockito::Server::new();
    let gate = challenge(&mut server, MINT);
    let no = server
        .mock("GET", "/")
        .match_header("cookie", "cf_clearance=1")
        .with_status(403)
        .with_body("<h1>Forbidden</h1>")
        .expect(1)
        .create();
    let (code, out) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    gate.assert();
    no.assert();
    let v = envelope(&out);
    assert_eq!(code, 1);
    assert_eq!(v["error"]["kind"], "http.403");
    assert_eq!(v["challenge"], block(1, true));
}

#[test]
fn a_refusal_that_declares_no_challenge_is_byte_identical() {
    // The guard: `retry-after` beside a 4xx/5xx is a rate limit or an outage,
    // and a vendor action other than `challenge` is a block. Neither runs a
    // script, takes a round or grows a `challenge` key — with or without
    // `--js`, the envelope is byte for byte the error it was.
    let mint = format!("<script>{MINT}</script>");
    for (status, name, value) in [
        (403, "server", "nginx"),
        (403, "cf-mitigated", "block"),
        (429, "retry-after", "30"),
        (503, "retry-after", "5"),
    ] {
        let mut server = mockito::Server::new();
        let page = server
            .mock("GET", "/")
            .with_status(status)
            .with_header(name, value)
            .with_body(&mint)
            .expect(2)
            .create();
        let url = server.url();
        let want = format!(
            "{{\"frot\":\"0\",\"url\":{{\"requested\":\"{url}\",\"final\":\"{url}/\"}},\
             \"view\":\"text\",\"status\":\"error\",\
             \"error\":{{\"kind\":\"http.{status}\",\"message\":\"server returned HTTP {status}\"}},\
             \"http\":{{\"status\":{status},\"headers\":[{{\"name\":\"{name}\",\"value\":\"{value}\"}}]}}}}"
        );
        for recipe in [&["--out", "text"][..], &["--js", "--out", "text"]] {
            let (code, out) = run_capture(&[&[url.as_str()], recipe].concat());
            assert_eq!(code, 1);
            assert_eq!(out.trim(), want);
        }
        page.assert();
    }
}
