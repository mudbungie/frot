//! The verb seam (`docs/design/challenge.md` §2/§3): a page script's POST is
//! bounded in size, never cached, reaches only `http(s)`, and rides the same
//! budgets as a GET. That no *other* code can issue one is not tested here —
//! it is a property of [`ScriptBody`]'s constructor, enforced by the compiler.

use std::time::Duration;

use mockito::Matcher;

use super::{ScriptBody, REQ_BODY_BYTES};
use crate::fetch::{FetchSession, Intent};
use crate::js::engine::{Clock, NetBudget};
use crate::js::subfetch::{Outcome, Subfetch};

const TEXT: &str = "text/plain;charset=UTF-8";

fn open(base: &str) -> Subfetch {
    Subfetch::new(FetchSession::new(Vec::new()), base, NetBudget::never())
}

fn failed(o: Outcome) -> String {
    match o {
        Outcome::Failed(m) => m,
        Outcome::Got(f) => panic!("expected Failed, got {}", f.status),
    }
}

fn got(o: Outcome) -> crate::js::subfetch::Frozen {
    match o {
        Outcome::Got(f) => f,
        Outcome::Failed(m) => panic!("expected Got, failed: {m}"),
    }
}

#[test]
fn a_post_carries_the_scripts_body_and_is_never_frozen() {
    let mut server = mockito::Server::new();
    let post = server
        .mock("POST", "/verify")
        .match_body(Matcher::Exact("solution=42".into()))
        .match_header("content-type", "application/x-www-form-urlencoded")
        .match_header("content-length", "11")
        .with_status(201)
        .with_body("minted")
        .expect(2)
        .create();
    // A later GET of the same URL is its own request: the POST's response was
    // never put in the once-then-frozen cache.
    let get = server
        .mock("GET", "/verify")
        .with_body("got")
        .expect(1)
        .create();
    let mut sf = open(&server.url());
    for _ in 0..2 {
        let f = got(sf.post(
            "/verify",
            "solution=42",
            "application/x-www-form-urlencoded",
        ));
        assert_eq!((f.status, f.body.as_str()), (201, "minted"));
    }
    assert_eq!(got(sf.get("/verify", Intent::FetchXhr)).body, "got");
    post.assert();
    get.assert();
    // Each real dispatch is a resource timing, a POST like a GET.
    assert_eq!(sf.timings().len(), 3);
}

#[test]
fn a_body_over_the_cap_never_becomes_a_request() {
    let mut server = mockito::Server::new();
    let post = server.mock("POST", "/up").expect(1).create();
    let mut sf = open(&server.url());
    got(sf.post("/up", &"x".repeat(REQ_BODY_BYTES), TEXT));
    let big = "x".repeat(REQ_BODY_BYTES + 1);
    let why = failed(sf.post("/up", &big, TEXT));
    assert_eq!(why, "request body is 65537 bytes; limit is 65536");
    post.assert();
    assert!(ScriptBody::from_script(&big, TEXT).is_err());
}

#[test]
fn only_an_http_target_can_receive_a_post() {
    // A `data:` or local target is refused; so is the remote→local reach every
    // subfetch refuses, and a page URL nothing resolves against.
    let mut sf = open("https://example.com/");
    assert!(failed(sf.post("data:text/plain,x", "b", TEXT)).contains("non-http POST"));
    assert!(failed(sf.post("file:///etc/passwd", "b", TEXT)).contains("remote"));
    let mut local = open("file:///tmp/page.html");
    assert!(failed(local.post("form.cgi", "b", TEXT)).contains("non-http POST"));
    assert!(!failed(open("not a url").post("/x", "b", TEXT)).is_empty());
}

#[test]
fn a_post_spends_the_same_network_budget_a_get_does() {
    let net = NetBudget::on(Clock::wall(), Duration::ZERO);
    let mut sf = Subfetch::new(FetchSession::new(Vec::new()), "https://example.com/", net);
    assert!(failed(sf.post("/verify", "b", TEXT)).contains("network budget"));
    assert!(sf.refused());
}

#[test]
fn a_redirected_post_keeps_its_body_only_across_307_and_308() {
    let mut server = mockito::Server::new();
    let _a = server
        .mock("POST", "/keep")
        .with_status(307)
        .with_header("location", "/kept")
        .create();
    let kept = server
        .mock("POST", "/kept")
        .match_body(Matcher::Exact("b".into()))
        .with_body("kept")
        .expect(1)
        .create();
    let _b = server
        .mock("POST", "/see")
        .with_status(303)
        .with_header("location", "/seen")
        .create();
    // A 303 (as a 301/302) continues as a GET: no body, no POST headers.
    let seen = server
        .mock("GET", "/seen")
        .match_header("content-type", Matcher::Missing)
        .match_header("origin", Matcher::Missing)
        .with_body("seen")
        .expect(1)
        .create();
    let mut sf = open(&server.url());
    assert_eq!(got(sf.post("/keep", "b", TEXT)).body, "kept");
    assert_eq!(got(sf.post("/see", "b", TEXT)).body, "seen");
    kept.assert();
    seen.assert();
}

#[test]
fn a_post_that_cannot_connect_is_a_failed_outcome() {
    let mut sf = open("http://127.0.0.1:1/");
    assert!(!failed(sf.post("/verify", "b", TEXT)).is_empty());
}
