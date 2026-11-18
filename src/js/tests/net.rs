//! Network-layer tests (js.md §6): `fetch`/`XMLHttpRequest` over the once-then-
//! frozen subfetch cache, driven through a local `mockito` server (never the
//! real network) or a refused `file:` target. These exercise the `net` syscall
//! (the result object and its header array) and the prelude `net.js` end to end;
//! the cache's own policy branches live in `super::super::subfetch::tests`.

use mockito::Server;

use super::super::{Env, Session, StyleSource};
use crate::dom::Document;
use crate::fetch::FetchSession;

fn session_at(url: &str) -> Session {
    Session::new(
        Document::parse("<html><body></body></html>"),
        StyleSource::Bare,
        Env {
            url: url.into(),
            user_agent: "frot-test/1".into(),
            accept_language: "en-US,en;q=0.5".into(),
        },
        &FetchSession::new(Vec::new()),
    )
}

#[test]
fn fetch_resolves_with_status_body_and_headers() {
    let mut server = Server::new();
    let _m = server
        .mock("GET", "/r")
        .with_status(200)
        .with_header("x-kind", "note")
        .with_body("hello")
        .create();
    let s = session_at(&server.url());
    s.eval(
        "globalThis.o=''; fetch('/r').then(function(r){ \
           globalThis.k=r.status+':'+r.ok; globalThis.h=r.headers.get('x-kind'); \
           return r.text(); }).then(function(t){ globalThis.o=t; });",
    )
    .unwrap();
    assert_eq!(s.eval("o").unwrap(), "hello");
    assert_eq!(s.eval("k").unwrap(), "200:true");
    assert_eq!(s.eval("h").unwrap(), "note");
}

#[test]
fn a_fetch_appears_as_a_resource_timing_entry() {
    // bl-e707: a real subfetch surfaces through performance.getEntriesByType(
    // 'resource') — the honest resource-timing surface. Only real measured
    // requests appear; unmeasured DNS/TCP/TLS phases are spec-legal 0, and no
    // navigation entry is fabricated for the pre-JS document fetch.
    let mut server = Server::new();
    let _m = server.mock("GET", "/r").with_body("x").create();
    let s = session_at(&server.url());
    s.eval(
        "globalThis.n=0; fetch('/r').then(function(){ \
           globalThis.e = performance.getEntriesByType('resource')[0]; \
           globalThis.n = performance.getEntries().length; });",
    )
    .unwrap();
    assert_eq!(s.eval("String(n)").unwrap(), "1");
    assert_eq!(s.eval("e.entryType").unwrap(), "resource");
    assert_eq!(s.eval("e.name.slice(-2)").unwrap(), "/r");
    assert_eq!(s.eval("String(e.duration >= 0)").unwrap(), "true");
    assert_eq!(
        s.eval("String(e.connectStart === 0 && e.domainLookupStart === 0)")
            .unwrap(),
        "true"
    );
    // A non-resource type omits (no fabricated navigation entry), and lookup by
    // name finds the real one.
    assert_eq!(
        s.eval("String(performance.getEntriesByType('navigation').length)")
            .unwrap(),
        "0"
    );
    assert_eq!(
        s.eval("String(performance.getEntriesByName(e.name).length)")
            .unwrap(),
        "1"
    );
}

#[test]
fn fetch_rejects_a_refused_request() {
    // A remote page reaching a local file is refused (§6): the promise rejects,
    // exercising the syscall's error result. No network happens.
    let s = session_at("https://example.com/");
    s.eval(
        "globalThis.e=''; fetch('file:///etc/passwd') \
         .catch(function(err){ globalThis.e=String(err); });",
    )
    .unwrap();
    assert!(s.eval("e").unwrap().contains("remote"));
}

#[test]
fn fetch_refuses_every_method_but_get_and_post() {
    let s = session_at("https://example.com/");
    s.eval(
        "globalThis.e=''; fetch('/x', {method:'PUT', body:'x'}) \
         .catch(function(err){ globalThis.e=String(err); });",
    )
    .unwrap();
    assert!(s.eval("e").unwrap().contains("only GET and POST"));
}

#[test]
fn a_url_is_frozen_across_fetches() {
    let mut server = Server::new();
    let m = server.mock("GET", "/r").with_body("v1").expect(1).create();
    let s = session_at(&server.url());
    s.eval(
        "globalThis.a=''; globalThis.b=''; \
         fetch('/r').then(function(r){return r.text();}).then(function(t){ globalThis.a=t; \
           return fetch('/r'); }).then(function(r){return r.text();}) \
         .then(function(t){ globalThis.b=t; });",
    )
    .unwrap();
    assert_eq!(s.eval("a").unwrap(), "v1");
    assert_eq!(s.eval("b").unwrap(), "v1");
    m.assert();
}

#[test]
fn xhr_get_reads_status_body_and_headers() {
    let mut server = Server::new();
    let _m = server
        .mock("GET", "/d")
        .with_status(201)
        .with_header("x-tag", "v")
        .with_body("body!")
        .create();
    let s = session_at(&server.url());
    s.eval(
        "var x=new XMLHttpRequest(); x.open('GET','/d'); globalThis.loaded=false; \
         x.onload=function(){ globalThis.loaded=true; }; x.send(); \
         globalThis.xs=x.status; globalThis.xt=x.responseText; globalThis.xh=x.getResponseHeader('x-tag');",
    )
    .unwrap();
    assert_eq!(s.eval("xs").unwrap(), "201");
    assert_eq!(s.eval("xt").unwrap(), "body!");
    assert_eq!(s.eval("xh").unwrap(), "v");
    assert_eq!(s.eval("loaded").unwrap(), "true");
}

#[test]
fn xhr_refuses_every_method_but_get_and_post() {
    let s = session_at("https://example.com/");
    let out = s
        .eval("var x=new XMLHttpRequest(); x.open('DELETE','/x'); try { x.send(); 'sent' } catch(e) { 'threw' }")
        .unwrap();
    assert_eq!(out, "threw");
}

#[test]
fn xhr_failed_get_fires_onerror() {
    let s = session_at("https://example.com/");
    s.eval(
        "var x=new XMLHttpRequest(); x.open('GET','file:///etc/passwd'); globalThis.err=false; \
         x.onerror=function(){ globalThis.err=true; }; x.send();",
    )
    .unwrap();
    assert_eq!(s.eval("err").unwrap(), "true");
}

#[test]
fn a_post_carries_the_scripts_content_type_in_every_shape_or_the_body_default() {
    // The prelude is the one place a verb is named (challenge.md §2). The
    // script's own Content-Type rides in any `init.headers` shape; with none,
    // the body's type decides, as in a browser. A failed POST rejects.
    let mut server = Server::new();
    let mut post = |path: &str, ctype: &str, body: &str| {
        server
            .mock("POST", path)
            .match_header("content-type", ctype)
            .match_body(mockito::Matcher::Exact(body.into()))
            .with_body("ok")
            .expect(1)
            .create()
    };
    let mocks = [
        post("/record", "application/json", "{}"),
        post("/pairs", "a/b", "p"),
        post("/headers", "c/d", "h"),
        post(
            "/form",
            "application/x-www-form-urlencoded;charset=UTF-8",
            "a=1",
        ),
        post("/text", "text/plain;charset=UTF-8", "t"),
        post("/empty", "text/plain;charset=UTF-8", ""),
    ];
    let s = session_at(&server.url());
    s.eval(
        "globalThis.n = 0; globalThis.bad = ''; var ok = function () { n++; }; \
         fetch('/record', {method: 'post', headers: {'content-type': 'application/json'}, body: '{}'}).then(ok); \
         fetch('/pairs', {method: 'POST', headers: [['X-A', '1'], ['Content-Type', 'a/b']], body: 'p'}).then(ok); \
         fetch('/headers', {method: 'POST', headers: new Headers([['Content-Type', 'c/d']]), body: 'h'}).then(ok); \
         fetch('/form', {method: 'POST', body: new URLSearchParams('a=1')}).then(ok); \
         fetch('/text', {method: 'POST', headers: {'x-other': '1'}, body: 't'}).then(ok); \
         fetch('/empty', {method: 'POST'}).then(ok); \
         fetch('file:///etc/passwd', {method: 'POST', body: 'x'}) \
           .catch(function (e) { globalThis.bad = String(e); }); \
         fetch('/bytes', {method: 'POST', body: new Uint8Array([1, 2])}) \
           .catch(function (e) { globalThis.bytes = String(e); });",
    )
    .unwrap();
    assert_eq!(s.eval("n").unwrap(), "6");
    assert!(s.eval("bad").unwrap().contains("remote"));
    // Bytes frot cannot carry faithfully are refused, never sent as text.
    assert!(s
        .eval("bytes")
        .unwrap()
        .contains("only text request bodies"));
    mocks.iter().for_each(|m| m.assert());
}

#[test]
fn xhr_posts_its_body_under_the_content_type_the_script_set() {
    let mut server = Server::new();
    let m = server
        .mock("POST", "/v")
        .match_header("content-type", "application/json")
        .match_header("x-ignored", mockito::Matcher::Missing)
        .match_body(mockito::Matcher::Exact("{\"a\":1}".into()))
        .with_status(201)
        .with_body("made")
        .expect(1)
        .create();
    let s = session_at(&server.url());
    let out = s
        .eval(
            "var x = new XMLHttpRequest(); x.open('POST', '/v'); \
             x.setRequestHeader('X-Ignored', '1'); x.setRequestHeader('Content-Type', 'application/json'); \
             x.send(JSON.stringify({a: 1})); x.status + ':' + x.responseText + ':' + x.send.length",
        )
        .unwrap();
    assert_eq!(out, "201:made:0");
    m.assert();
}
