//! What the declared-challenge round trip must **not** do
//! (`docs/design/challenge.md` §6–§8): cost the ordinary page anything, raise a
//! budget to pass a gate, or originate a request the page's script did not.

use super::golden_tests::capture_bounded;
use super::*;
use crate::js::engine::{Clock, Deadline, NetBudget, EXEC_CPU_MS, NET_BUDGET_MS};
use crate::js::Bounds;
use mockito::Matcher::Missing;
use std::time::Duration;

fn envelope(out: &str) -> Value {
    serde_json::from_str(out.trim()).expect("envelope JSON")
}

fn net() -> NetBudget {
    NetBudget::on(Clock::manual(), Duration::from_millis(NET_BUDGET_MS))
}

#[test]
fn a_page_that_declares_no_challenge_is_byte_identical_and_fetched_once() {
    // The guard for the 99%: no `challenge` key, no second request, and the
    // envelope byte for byte what it was before the round trip existed.
    let mut server = mockito::Server::new();
    let page = server
        .mock("GET", "/")
        .with_body("<p>plain page</p><script>document.cookie = 'seen=1';</script>")
        .expect(2)
        .create();
    let url = server.url();
    let head = format!(
        "{{\"frot\":\"0\",\"url\":{{\"requested\":\"{url}\",\"final\":\"{url}/\"}},\
         \"view\":\"text\",\"status\":\"ok\",\"out\":\"plain page\",\
         \"http\":{{\"status\":200,\"headers\":[]}}"
    );
    let bounds = || Bounds {
        cpu: Deadline::compute(),
        net: net(),
    };
    let (_, plain) = capture_bounded(&[&url, "--out", "text"], bounds());
    assert_eq!(plain.trim(), format!("{head}}}"));
    // Under `--js` the page's script even changes the cookie jar — and that
    // still earns nothing, because nothing declared a challenge.
    let (_, js) = capture_bounded(&[&url, "--js", "--out", "text"], bounds());
    let block = "\"js\":{\"scripts\":1,\"errors\":0,\"settled\":true}";
    assert_eq!(js.trim(), format!("{head},{block}}}"));
    page.assert();
}

#[test]
fn a_proof_of_work_that_outruns_the_budget_is_an_honest_stop() {
    // The compute bound is never raised to pass a gate (challenge.md §7 ii):
    // the run stops on `budget`, nothing was minted, and frot says so.
    let mut server = mockito::Server::new();
    let gate = server
        .mock("GET", "/")
        .with_status(202)
        .with_header("x-amzn-waf-action", "challenge")
        .with_body("<script>while (true) {}</script>")
        .expect(1)
        .create();
    let bounds = Bounds {
        cpu: Deadline::on(Clock::cpu(), Duration::from_millis(20)),
        net: net(),
    };
    let (code, out) = capture_bounded(&[&server.url(), "--js", "--out", "text"], bounds);
    gate.assert();
    let v = envelope(&out);
    assert_eq!(code, 0);
    assert_eq!(v["needs"], serde_json::json!(["human"]));
    assert_eq!(
        v["challenge"],
        serde_json::json!({"declared": "x-amzn-waf-action", "rounds": 0, "passed": false})
    );
    assert_eq!(v["js"]["settled"], false, "{v}");
    assert_eq!(v["js"]["stopped"], "budget", "{v}");
}

#[test]
fn the_challenge_and_the_page_behind_it_spend_one_compute_budget() {
    // Two documents run in this invocation; the page gets what the challenge
    // left, never a fresh allowance. Each `/work` stands for 600 ms of compute
    // on a clock the test owns: together they exceed the one shipping budget, so
    // the *page's* run is the one that stops.
    let cpu = Clock::manual();
    let work = |clock: Clock| {
        move |_: &mockito::Request| {
            clock.advance(Duration::from_millis(600));
            Vec::new()
        }
    };
    // Distinct URLs: a repeated one would be served from the invocation cache.
    let burn =
        |n: u8| format!("var x = new XMLHttpRequest(); x.open('GET', '/work{n}'); x.send();");
    let mut server = mockito::Server::new();
    let both = server
        .mock("GET", mockito::Matcher::Regex("^/work".into()))
        .with_body_from_request(work(cpu.clone()))
        .expect(2)
        .create();
    let _gate = server
        .mock("GET", "/?page")
        .match_header("cookie", Missing)
        .with_status(202)
        .with_header("x-amzn-waf-action", "challenge")
        .with_body(format!(
            "<script>{} document.cookie = 'waf=1';</script>",
            burn(1)
        ))
        .create();
    let _page = server
        .mock("GET", "/?page")
        .match_header("cookie", "waf=1")
        .with_body(format!(
            "<p>the real page</p><script>{} for (var i = 0; i < 1e6; i++) {{}}</script>",
            burn(2)
        ))
        .create();
    let bounds = Bounds {
        cpu: Deadline::on(cpu, Duration::from_millis(EXEC_CPU_MS)),
        net: net(),
    };
    let url = format!("{}/?page", server.url());
    let (_, out) = capture_bounded(&[&url, "--js", "--out", "text"], bounds);
    both.assert();
    let v = envelope(&out);
    assert_eq!(v["challenge"]["passed"], true, "{v}");
    assert_eq!(v["out"], "the real page");
    assert_eq!(v["js"]["stopped"], "budget", "{v}");
}

#[test]
fn frot_originates_no_submission() {
    // Only the page's own `fetch`/XHR can issue a POST. A form is never
    // submitted — by `submit()`, `requestSubmit()`, or a dispatched event — and
    // a challenge page that only *contains* one is an honest stop.
    let mut server = mockito::Server::new();
    let _gate = server
        .mock("GET", "/")
        .with_status(202)
        .with_header("x-amzn-waf-action", "challenge")
        .with_body(
            "<form action='/verify' method='post'><input name='token' value='t'>\
             <button type='submit'>Continue</button></form>\
             <script>var f = document.querySelector('form');\
             try { f.submit(); } catch (e) {}\
             try { f.requestSubmit(); } catch (e) {}\
             f.dispatchEvent(new Event('submit'));\
             f.querySelector('button').dispatchEvent(new Event('click'));</script>",
        )
        .expect(1)
        .create();
    let any = mockito::Matcher::Any;
    let post = server.mock("POST", any.clone()).expect(0).create();
    let get = server.mock("GET", "/verify").expect(0).create();
    let (_, out) = capture_bounded(
        &[&server.url(), "--js", "--out", "text"],
        Bounds {
            cpu: Deadline::compute(),
            net: net(),
        },
    );
    post.assert();
    get.assert();
    assert_eq!(envelope(&out)["challenge"]["rounds"], 0);
}
