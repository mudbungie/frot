//! One end-to-end pass per `--out` view: the CLI is driven against a live
//! mock origin and the envelope's `out` payload is read back.
//!
//! Split out of `run/tests.rs` (bl-a68b), which keeps the entry point's own
//! surface — usage, exit codes, and the `status`/`needs`/`url` envelope
//! contract. Views with a subject of their own already have their own module:
//! `bboxes_tests` for `--out bboxes`, `ax_tests` for reading order under
//! `--css`, `css_media_tests` for `<link media=…>`.

use super::*;

fn run_capture(args: &[&str]) -> (u8, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let code = run_io(&argv, &mut out, &mut err);
    (code, String::from_utf8(out).unwrap())
}

fn parse_envelope(s: &str) -> Value {
    serde_json::from_str(s.trim()).expect("envelope JSON")
}

#[test]
fn ax_view_emits_ok_envelope_with_tree() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_body("<h1>Hello</h1>")
        .create();
    let url = server.url();
    let (code, out) = run_capture(&[&url, "--out", "ax"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["view"], "ax");
    let arr = v["out"].as_array().expect("out is array");
    let has_heading = arr.iter().any(|n| n["role"] == "heading");
    assert!(has_heading, "expected a heading in {arr:?}");
}

#[test]
fn end_to_end_text_view() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("content-type", "text/html; charset=utf-8")
        .with_body("<html><body><p>Hello, world!</p></body></html>")
        .create();
    let url = server.url();
    let (code, out) = run_capture(&[&url, "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["view"], "text");
    assert_eq!(v["out"], "Hello, world!");
}

#[test]
fn end_to_end_dom_view_is_array() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_body("<html><body><a href='/x'>x</a></body></html>")
        .create();
    let url = server.url();
    let (code, out) = run_capture(&[&url, "--out", "dom"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert!(v["out"].is_array());
}

#[test]
fn end_to_end_links_resolves_against_final_url() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_body("<a href='/landing'>go</a>")
        .create();
    let url = server.url();
    let (code, out) = run_capture(&[&url, "--out", "links"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    let href = v["out"][0]["href"].as_str().unwrap();
    assert!(href.ends_with("/landing"));
    assert!(href.starts_with("http://"));
}

#[test]
fn end_to_end_forms_extracts_action() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_body("<form action='/submit' method='post'><input name='q'></form>")
        .create();
    let url = server.url();
    let (code, out) = run_capture(&[&url, "--out", "forms"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["out"][0]["method"], "post");
    assert!(v["out"][0]["action"].as_str().unwrap().ends_with("/submit"));
}

#[test]
fn end_to_end_meta_extracts_title() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_body("<html><head><title>my page</title></head></html>")
        .create();
    let url = server.url();
    let (code, out) = run_capture(&[&url, "--out", "meta"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["out"]["title"], "my page");
}

#[test]
fn based_links_are_absolute_against_the_final_url() {
    // bl-409e end to end: `<base href="/">` on a page reached through a
    // redirect. The base is the *landing* URL's origin root, and every emitted
    // href is absolute — the README's "links with absolute hrefs" promise, which
    // an unresolved raw base silently broke.
    let mut server = mockito::Server::new();
    let _m1 = server
        .mock("GET", "/start")
        .with_status(301)
        .with_header("location", "/docs/intro")
        .create();
    let _m2 = server
        .mock("GET", "/docs/intro")
        .with_status(200)
        .with_body(
            "<head><base href='/'>\
             <link rel='apple-touch-icon' href='/assets/icon.png'></head>\
             <body><a href='guide'>g</a></body>",
        )
        .create();
    let start = format!("{}/start", server.url());
    let (code, out) = run_capture(&[&start, "--out", "links"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    let hrefs: Vec<String> = v["out"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["href"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        hrefs,
        vec![
            format!("{}/assets/icon.png", server.url()),
            format!("{}/guide", server.url()),
        ]
    );
}

#[test]
fn css_flag_applies_visibility_to_text_view() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_body("<style>.x{display:none}</style><p>shown</p><p class=x>hidden</p>")
        .create();
    let url = server.url();
    let (code, out) = run_capture(&[&url, "--css", "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["out"], "shown");
}

#[test]
fn css_fetches_external_stylesheets_best_effort() {
    let mut server = mockito::Server::new();
    let dead = "http://127.0.0.1:1/dead.css";
    let body = format!(
        "<link rel=stylesheet href=\"/s.css\">\
         <link rel=stylesheet href=\"{dead}\">\
         <p>shown</p><p class=hide>secret</p>"
    );
    let _page = server
        .mock("GET", "/")
        .with_status(200)
        .with_body(body)
        .create();
    let _sheet = server
        .mock("GET", "/s.css")
        .with_status(200)
        .with_body(".hide{display:none}")
        .create();
    let url = server.url();
    let (code, out) = run_capture(&[&url, "--css", "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["out"], "shown");
}
