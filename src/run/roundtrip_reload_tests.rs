//! A script's request to reload its own document earns the round a declared
//! challenge does (`docs/design/challenge.md` §12): asked — by the server or
//! by the page's script — and earned — the script changed the request. The
//! measured shape is Akamai's undeclared interstitial (§11.6); every way it
//! stops short, and the ordinary pages it must not touch, are pinned beside it.

use super::*;
use mockito::Matcher::Missing;

fn run_capture(args: &[&str]) -> Value {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    run_io(&argv, &mut out, &mut err);
    serde_json::from_str(String::from_utf8(out).unwrap().trim()).expect("envelope JSON")
}

/// An undeclared 200 whose body is `script`, served to a request carrying the
/// server's first cookie only — so a second navigation is a second hit.
fn interstitial(server: &mut mockito::Server, script: &str) -> mockito::Mock {
    server
        .mock("GET", "/")
        .match_header("cookie", Missing)
        .with_header("set-cookie", "ak=1; Path=/")
        .with_body(format!(
            "<meta http-equiv=refresh content=\"5; URL='/?bm-verify=t'\"><script>{script}</script>"
        ))
        .expect(1)
        .create()
}

/// Akamai's verify step, as captured: a synchronous XHR POST whose `loadend`
/// listener reads `{"reload": true}` and replaces the location with its own.
const VERIFY: &str = "var x = new XMLHttpRequest(); x.open('POST', '/_sec/verify', false); \
    x.setRequestHeader('Content-Type', 'application/json'); \
    x.addEventListener('loadend', function () { var r = JSON.parse(x.responseText); \
      if (r.reload) window.location.replace(window.location.href.replace(/[&?]bm-verify=[^#]*/, '')); \
      else location.reload(); }); \
    x.send(JSON.stringify({'bm-verify': 't', pow: 1 + 2}));";

/// The second navigation that must never happen: any cookie-carrying GET.
fn no_second(server: &mut mockito::Server) -> mockito::Mock {
    let any = mockito::Matcher::Regex(String::new());
    server
        .mock("GET", "/")
        .match_header("cookie", any)
        .expect(0)
        .create()
}

fn verify(server: &mut mockito::Server, cookie: &str) -> mockito::Mock {
    server
        .mock("POST", "/_sec/verify")
        .with_header("set-cookie", cookie)
        .with_body("{\"reload\": true}")
        .expect(1)
        .create()
}

#[test]
fn an_interstitial_that_asks_to_reload_after_its_verify_gets_the_page() {
    let mut server = mockito::Server::new();
    let gate = interstitial(&mut server, VERIFY);
    let v = verify(&mut server, "ak=2; Path=/");
    let real = server
        .mock("GET", "/")
        .match_header("cookie", "ak=2")
        .match_header("referer", Missing)
        .with_body("<p>the real page</p>")
        .expect(1)
        .create();
    let env = run_capture(&[&server.url(), "--js", "--out", "text"]);
    gate.assert();
    v.assert();
    real.assert();
    assert_eq!(env["status"], "ok", "{env}");
    assert_eq!(env["out"], "the real page");
    // Nothing declared a challenge, so there is no `challenge` block to say
    // what it took (`bl-14f9`): `http` and `js` are the delivered page's.
    assert!(env.get("challenge").is_none(), "{env}");
    assert_eq!(env["http"]["status"], 200);
    assert_eq!(env["js"]["scripts"], 0);
}

#[test]
fn a_reload_that_changes_nothing_is_the_interstitial_itself() {
    // The verify answer sets no cookie: the request is unchanged, so a reload
    // would re-ask the same question — the page is what it is, one request.
    let mut server = mockito::Server::new();
    let gate = interstitial(&mut server, VERIFY);
    let v = server
        .mock("POST", "/_sec/verify")
        .with_body("{\"reload\": true}")
        .create();
    let none = no_second(&mut server);
    let env = run_capture(&[&server.url(), "--js", "--out", "text"]);
    gate.assert();
    v.assert();
    none.assert();
    assert_eq!(env["needs"], serde_json::json!(["js"]), "{env}");
    assert_eq!(
        env["js"]["errors"], 1,
        "the refused navigation still counts"
    );
}

#[test]
fn a_changed_request_without_a_reload_or_with_a_navigation_elsewhere_takes_no_round() {
    // The ordinary page writes cookies all the time: without the ask, nothing.
    // A script that navigates anywhere but its own document is not followed.
    for script in [
        "document.cookie = 'ak=2; path=/'; document.body.textContent = 'here';",
        "document.cookie = 'ak=2; path=/'; location.assign('/elsewhere');",
        "document.cookie = 'ak=2; path=/'; location.replace(location.href + '#f');",
    ] {
        let mut server = mockito::Server::new();
        let gate = interstitial(&mut server, script);
        let none = no_second(&mut server);
        run_capture(&[&server.url(), "--js", "--out", "text"]);
        gate.assert();
        none.assert();
    }
}

#[test]
fn a_subresource_cookie_and_a_reload_take_no_round() {
    // The ask alone is not the earning: a `<script src>`'s cookie is the
    // server's (§12), so `location.reload()` beside it changes nothing.
    let mut server = mockito::Server::new();
    let gate = server
        .mock("GET", "/")
        .with_body("<script src='/a.js'></script><p>stand-in</p>")
        .expect(1)
        .create();
    let js = server
        .mock("GET", "/a.js")
        .with_header("set-cookie", "ak=2; Path=/")
        .with_body("location.reload();")
        .create();
    let env = run_capture(&[&server.url(), "--js", "--out", "text"]);
    gate.assert();
    js.assert();
    assert_eq!(env["out"], "stand-in", "{env}");
}

#[test]
fn a_reload_spends_the_one_round_whatever_answers_it() {
    // The re-issue is answered by a declared challenge: the invocation's one
    // round is spent, so it is reported and stopped — never a third request.
    let mut server = mockito::Server::new();
    let gate = interstitial(&mut server, VERIFY);
    let v = verify(&mut server, "ak=2; Path=/");
    let declared = server
        .mock("GET", "/")
        .match_header("cookie", "ak=2")
        .with_status(202)
        .with_header("x-amzn-waf-action", "challenge")
        .with_body("<script>document.cookie = 'waf=1; path=/';</script>")
        .expect(1)
        .create();
    let env = run_capture(&[&server.url(), "--js", "--out", "text"]);
    gate.assert();
    v.assert();
    declared.assert();
    assert_eq!(env["needs"], serde_json::json!(["human"]), "{env}");
    let block = serde_json::json!({"declared": "x-amzn-waf-action", "rounds": 1, "passed": false});
    assert_eq!(env["challenge"], block);
    // And a reload asked by the page a round delivered is not honoured again.
    let mut server = mockito::Server::new();
    let gate = interstitial(&mut server, VERIFY);
    let v = verify(&mut server, "ak=2; Path=/");
    let again = server
        .mock("GET", "/")
        .match_header("cookie", "ak=2")
        .with_body("<script>document.cookie = 'ak=3; path=/'; location.reload();</script>")
        .expect(1)
        .create();
    run_capture(&[&server.url(), "--js", "--out", "text"]);
    gate.assert();
    v.assert();
    again.assert();
}

#[test]
fn without_js_the_interstitial_is_never_run_or_reloaded() {
    let mut server = mockito::Server::new();
    let gate = interstitial(&mut server, VERIFY);
    let env = run_capture(&[&server.url(), "--out", "text"]);
    gate.assert();
    assert!(env.get("js").is_none(), "{env}");
}
