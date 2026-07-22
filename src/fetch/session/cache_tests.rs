//! Invocation resource-cache semantics (`bl-08f6`): one URL dedupes across the
//! request intents that share it, and a content-negotiated (`Vary`) response is
//! conservatively refused reuse rather than risk the wrong body. All against a
//! local `mockito` origin, never the real network.

use std::time::Duration;

use super::{FetchSession, Intent};

const GENEROUS: Duration = Duration::from_secs(15);

/// Fetch `path` as a subresource of `base` under `intent`, returning the body.
fn body(s: &FetchSession, base: &str, path: &str, intent: Intent) -> String {
    s.subresource(&format!("{base}{path}"), base, intent, GENEROUS)
        .unwrap()
        .body
}

#[test]
fn the_same_url_dedupes_across_two_request_intents() {
    // The `--css --js` overlap: a URL wanted as a stylesheet before JS and as a
    // module during it is one network hit — the one session cache is keyed by
    // URL, so the second intent re-serves the frozen body.
    let mut server = mockito::Server::new();
    let m = server
        .mock("GET", "/x")
        .with_body("shared")
        .expect(1)
        .create();
    let s = FetchSession::new(Vec::new());
    assert_eq!(body(&s, &server.url(), "/x", Intent::Style), "shared");
    assert_eq!(body(&s, &server.url(), "/x", Intent::Module), "shared");
    m.assert();
}

#[test]
fn a_vary_cookie_response_is_refetched_not_reused() {
    // A content-negotiated response (Vary on a header our requests differ on)
    // must never be re-served for the next request — the cached body could be
    // the wrong one. It is refused caching, so both requests hit the origin.
    let mut server = mockito::Server::new();
    let m = server
        .mock("GET", "/n")
        .with_header("vary", "Cookie")
        .with_body("body")
        .expect(2)
        .create();
    let s = FetchSession::new(Vec::new());
    for _ in 0..2 {
        assert_eq!(body(&s, &server.url(), "/n", Intent::Style), "body");
    }
    m.assert();
}

#[test]
fn a_vary_star_response_is_never_reused() {
    let mut server = mockito::Server::new();
    let m = server
        .mock("GET", "/n")
        .with_header("vary", "*")
        .with_body("body")
        .expect(2)
        .create();
    let s = FetchSession::new(Vec::new());
    for _ in 0..2 {
        body(&s, &server.url(), "/n", Intent::Style);
    }
    m.assert();
}

#[test]
fn a_vary_only_on_accept_encoding_is_still_cached_once() {
    // Accept-Encoding is the one request header frot sends identically on every
    // request, so a Vary on it (even with an empty trailing token) cannot change
    // the body: the response stays reusable and the second request is a hit.
    let mut server = mockito::Server::new();
    let m = server
        .mock("GET", "/s.css")
        .with_header("vary", "Accept-Encoding, ")
        .with_body("a{}")
        .expect(1)
        .create();
    let s = FetchSession::new(Vec::new());
    for _ in 0..2 {
        body(&s, &server.url(), "/s.css", Intent::Style);
    }
    m.assert();
}
