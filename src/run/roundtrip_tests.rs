//! The declared-challenge round trip, end to end (`docs/design/challenge.md`):
//! a synthetic challenge whose **own script** mints the token — by
//! `document.cookie`, or by a POST it chose to issue — earns exactly one
//! re-issue of the caller's URL, and the envelope says so. Every way it can
//! stop short is pinned beside it, as is the ordinary path it must not touch.
//! The verb seam itself is pinned in `js/subfetch/post/tests.rs`.

use super::*;
use mockito::Matcher::{self, Missing};

fn run_capture(args: &[&str]) -> (u8, Value) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let code = run_io(&argv, &mut out, &mut err);
    let out = String::from_utf8(out).unwrap();
    (
        code,
        serde_json::from_str(out.trim()).expect("envelope JSON"),
    )
}

/// The proof-of-work every synthetic challenge here asks for, and its answer.
const WORK: &str = "var n = 7; for (var i = 0; i < 2000; i++) n = (n * 31 + i) % 65521;";
fn solution() -> u32 {
    (0..2000).fold(7, |n, i| (n * 31 + i) % 65521)
}

/// An AWS-WAF-shaped challenge response: `202`, the declaring header, and a
/// body that is nothing but the challenge `script`.
fn challenge(server: &mut mockito::Server, script: &str) -> mockito::Mock {
    server
        .mock("GET", "/")
        .match_header("cookie", Missing)
        .with_status(202)
        .with_header("x-amzn-waf-action", "challenge")
        .with_body(format!(
            "<html><body><script>{script}</script></body></html>"
        ))
        .expect(1)
        .create()
}

/// The real page, served only to a request carrying the minted `cookie`.
fn page(server: &mut mockito::Server, cookie: &str) -> mockito::Mock {
    server
        .mock("GET", "/")
        .match_header("cookie", cookie)
        .match_header("referer", Missing)
        .match_header("sec-fetch-site", "none")
        .with_body("<title>Shop</title><p>the real page</p>")
        .expect(1)
        .create()
}

fn block(rounds: u8, passed: bool) -> Value {
    serde_json::json!({"declared": "x-amzn-waf-action", "rounds": rounds, "passed": passed})
}

#[test]
fn a_script_minted_cookie_earns_one_re_issue_and_the_page() {
    let mut server = mockito::Server::new();
    let script = format!("{WORK} document.cookie = 'waf=' + n + '; path=/'; location.reload();");
    let gate = challenge(&mut server, &script);
    // The re-issue is the caller's own navigation again: no referrer, site
    // `none`, exactly as the first — plus the cookie the script minted.
    let real = page(&mut server, &format!("waf={}", solution()));
    let (code, v) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    gate.assert();
    real.assert();
    assert_eq!(code, 0);
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["out"], "the real page");
    assert_eq!(v["challenge"], block(1, true));
    // `http` and `js` describe the delivered page; `challenge` what it took.
    assert_eq!(v["http"]["status"], 200);
    assert_eq!(v["js"]["scripts"], 0, "{v}");
    assert_eq!(v["url"]["final"], format!("{}/", server.url()));
}

#[test]
fn a_token_minted_by_the_scripts_own_post_earns_the_page() {
    // The measured vendor shape: the script POSTs its solution, the response
    // sets an HttpOnly token no script could write, and the re-issue carries it.
    let mut server = mockito::Server::new();
    let origin = server.url();
    let script = format!(
        "{WORK} fetch('/verify', {{method: 'POST', headers: {{'Content-Type': 'application/json'}}, \
         body: JSON.stringify({{solution: n}})}});"
    );
    let gate = challenge(&mut server, &script);
    let verify = server
        .mock("POST", "/verify")
        .match_body(Matcher::Exact(format!("{{\"solution\":{}}}", solution())))
        .match_header("content-type", "application/json")
        .match_header("origin", origin.as_str())
        .match_header("referer", format!("{origin}/").as_str())
        .match_header("sec-fetch-mode", "cors")
        .with_header("set-cookie", "token=minted; HttpOnly; Path=/")
        .with_body("{}")
        .expect(1)
        .create();
    let real = page(&mut server, "token=minted");
    let (code, v) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    gate.assert();
    verify.assert();
    real.assert();
    assert_eq!(code, 0);
    assert_eq!(v["out"], "the real page", "{v}");
    assert_eq!(v["challenge"], block(1, true));
}

#[test]
fn a_cross_origin_token_post_carries_the_body_and_no_caller_credentials() {
    // The vendor-origin shape (`token.awswaf.com`): the POST goes cross-origin
    // with the script's body and a browser-default content type, never the
    // caller's `-H`; the script writes the returned token itself.
    let mut vendor = mockito::Server::new();
    let mut server = mockito::Server::new();
    let script = format!(
        "{WORK} var x = new XMLHttpRequest(); x.open('POST', '{}/verify'); \
         x.onload = function () {{ document.cookie = 'waf=' + JSON.parse(x.responseText).token; }}; \
         x.send('s=' + n);",
        vendor.url()
    );
    let _gate = server
        .mock("GET", "/")
        .match_header("cookie", Missing)
        .match_header("authorization", "Bearer s3cret")
        .with_status(202)
        .with_header("x-amzn-waf-action", "challenge")
        .with_body(format!("<script>{script}</script>"))
        .create();
    let verify = vendor
        .mock("POST", "/verify")
        .match_body(Matcher::Exact(format!("s={}", solution())))
        .match_header("content-type", "text/plain;charset=UTF-8")
        .match_header("origin", server.url().as_str())
        .match_header("authorization", Missing)
        .match_header("cookie", Missing)
        .with_body("{\"token\":\"abc\"}")
        .expect(1)
        .create();
    let real = page(&mut server, "waf=abc");
    let (code, v) = run_capture(&[
        &server.url(),
        "--js",
        "-H",
        "Authorization: Bearer s3cret",
        "--out",
        "text",
    ]);
    verify.assert();
    real.assert();
    assert_eq!(code, 0);
    assert_eq!(v["out"], "the real page", "{v}");
    assert_eq!(v["challenge"], block(1, true));
}

#[test]
fn a_challenge_script_that_fails_is_an_honest_stop_after_one_request() {
    let mut server = mockito::Server::new();
    let gate = challenge(&mut server, "throw new Error('unsupported browser');");
    let (code, v) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    gate.assert();
    assert_eq!(code, 0);
    assert_eq!(v["status"], "needs");
    assert_eq!(v["needs"], serde_json::json!(["human"]));
    assert_eq!(v["challenge"], block(0, false));
    assert_eq!(v["js"]["errors"], 1, "{v}");
    assert!(v.get("out").is_none());
}

#[test]
fn a_challenge_that_is_still_posed_after_its_round_stops_at_two_requests() {
    // The loop guard: every response is a challenge and every run mints a new
    // cookie. `CHALLENGE_ROUNDS` ends it at two navigations, and the second
    // challenge document is never run.
    let mut server = mockito::Server::new();
    let gate = server
        .mock("GET", "/")
        .with_status(202)
        .with_header("x-amzn-waf-action", "challenge")
        .with_body("<script>document.cookie = 'try=' + Math.random();</script>")
        .expect(2)
        .create();
    let (code, v) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    gate.assert();
    assert_eq!(code, 0);
    assert_eq!(v["needs"], serde_json::json!(["human"]));
    assert_eq!(v["challenge"], block(1, false));
    assert_eq!(v["http"]["status"], 202);
    // The one run that happened is the one reported.
    assert_eq!(v["js"]["scripts"], 1, "{v}");
}

#[test]
fn a_cookie_that_does_not_change_the_request_earns_nothing() {
    // The precondition is the request, not the jar (challenge.md §4.1): a
    // cookie scoped to another path would not ride the re-issue, so there is
    // none. The same holds when the caller's own `-H Cookie` masks the jar.
    let mut server = mockito::Server::new();
    let gate = server
        .mock("GET", "/")
        .with_status(202)
        .with_header("x-amzn-waf-action", "challenge")
        .with_body("<script>document.cookie = 'waf=1; path=/elsewhere';</script>")
        .expect(1)
        .create();
    let (_, v) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    gate.assert();
    assert_eq!(v["challenge"], block(0, false));
    let masked = server
        .mock("GET", "/m")
        .with_status(202)
        .with_header("x-amzn-waf-action", "challenge")
        .with_body("<script>document.cookie = 'waf=1; path=/';</script>")
        .expect(1)
        .create();
    let url = format!("{}/m", server.url());
    let (_, v) = run_capture(&[&url, "--js", "-H", "Cookie: mine=1", "--out", "text"]);
    masked.assert();
    assert_eq!(v["challenge"], block(0, false));
}

#[test]
fn a_captcha_action_is_not_a_declared_challenge_and_takes_no_round() {
    // challenge.md §5.2 row 4: an action other than `challenge` is unchanged —
    // no round, no block, the ordinary pipeline on the one response.
    let mut server = mockito::Server::new();
    let gate = server
        .mock("GET", "/")
        .with_status(202)
        .with_header("x-amzn-waf-action", "captcha")
        .with_body("<p>Type the characters you see</p><script>document.cookie = 'x=1';</script>")
        .expect(1)
        .create();
    let (code, v) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    gate.assert();
    assert_eq!(code, 0);
    assert_eq!(v["out"], "Type the characters you see");
    assert!(v.get("challenge").is_none(), "{v}");
}

#[test]
fn what_the_re_issue_returns_is_reported_as_it_is() {
    let mut server = mockito::Server::new();
    let mint = "document.cookie = 'waf=1; path=/';";
    // Refused after the token: the challenge passed, the server said no.
    let _gate = challenge(&mut server, mint);
    let _no = server
        .mock("GET", "/")
        .match_header("cookie", "waf=1")
        .with_status(403)
        .create();
    let (code, v) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    assert_eq!(code, 1);
    assert_eq!(v["error"]["kind"], "http.403");
    assert_eq!(v["challenge"], block(1, true));
    // No response at all (a redirect loop spends the navigation's hop budget):
    // nothing passed.
    let mut server = mockito::Server::new();
    let _gate = challenge(&mut server, mint);
    let _spin = server
        .mock("GET", "/")
        .match_header("cookie", "waf=1")
        .with_status(302)
        .with_header("location", "/")
        .create();
    let (code, v) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    assert_eq!(code, 1);
    assert_eq!(v["error"]["kind"], "fetch.redirect");
    assert_eq!(v["challenge"], block(1, false));
}
