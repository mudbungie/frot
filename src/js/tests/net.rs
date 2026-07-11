//! Network-layer tests (js.md §6): `fetch`/`XMLHttpRequest` over the once-then-
//! frozen subfetch cache, driven through a local `mockito` server (never the
//! real network) or a refused `file:` target. These exercise the `net` syscall
//! (the result object and its header array) and the prelude `net.js` end to end;
//! the cache's own policy branches live in `super::super::subfetch::tests`.

use mockito::Server;

use super::super::{Env, Session, StyleSource};
use crate::dom::Document;

fn session_at(url: &str) -> Session {
    Session::new(
        Document::parse("<html><body></body></html>"),
        StyleSource::Bare,
        Env {
            url: url.into(),
            user_agent: "frot-test/1".into(),
            headers: Vec::new(),
        },
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
fn fetch_refuses_a_non_get_method() {
    let s = session_at("https://example.com/");
    s.eval(
        "globalThis.e=''; fetch('/x', {method:'POST'}) \
         .catch(function(err){ globalThis.e=String(err); });",
    )
    .unwrap();
    assert!(s.eval("e").unwrap().contains("only GET"));
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
fn xhr_refuses_a_non_get_method() {
    let s = session_at("https://example.com/");
    let out = s
        .eval("var x=new XMLHttpRequest(); x.open('POST','/x'); try { x.send(); 'sent' } catch(e) { 'threw' }")
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
