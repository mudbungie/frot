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
fn help_writes_usage_to_stdout_and_exits_zero() {
    let (code, out, err) = run_capture(&["--help"]);
    assert_eq!(code, 0);
    assert!(out.contains("usage: frot"));
    assert!(err.is_empty());
}

#[test]
fn version_writes_to_stdout_and_exits_zero() {
    let (code, out, err) = run_capture(&["--version"]);
    assert_eq!(code, 0);
    assert!(out.contains("frot "));
    assert!(err.is_empty());
}

#[test]
fn no_args_writes_to_stderr_and_exits_two() {
    let (code, out, err) = run_capture(&[]);
    assert_eq!(code, 2);
    assert!(out.is_empty());
    assert!(err.contains("no arguments"));
}

#[test]
fn usage_error_writes_to_stderr_and_exits_two() {
    let (code, out, err) = run_capture(&["--nope"]);
    assert_eq!(code, 2);
    assert!(out.is_empty());
    assert!(err.contains("unknown flag"));
}

#[test]
fn invalid_url_emits_error_envelope_and_exits_one() {
    let (code, out, _) = run_capture(&["not a url", "--out", "text"]);
    assert_eq!(code, 1);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "error");
    assert_eq!(v["view"], "text");
    assert_eq!(v["error"]["kind"], "fetch.url");
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
    let (code, out, _) = run_capture(&[&url, "--out", "ax"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["view"], "ax");
    let arr = v["out"].as_array().expect("out is array");
    let has_heading = arr.iter().any(|n| n["role"] == "heading");
    assert!(has_heading, "expected a heading in {:?}", arr);
}

// `--out bboxes` end-to-end tests live in the sibling `bboxes_tests` module
// (keeps this file under the 300-line source cap).

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
    let (code, out, _) = run_capture(&[&url, "--out", "text"]);
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
    let (code, out, _) = run_capture(&[&url, "--out", "dom"]);
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
    let (code, out, _) = run_capture(&[&url, "--out", "links"]);
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
    let (code, out, _) = run_capture(&[&url, "--out", "forms"]);
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
    let (code, out, _) = run_capture(&[&url, "--out", "meta"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["out"]["title"], "my page");
}

#[test]
fn empty_spa_shell_yields_needs_envelope_with_js() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_body(
            "<html><head><script src='app.js'></script></head><body><div id='root'></div></body></html>",
        )
        .create();
    let url = server.url();
    let (code, out, _) = run_capture(&[&url, "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "needs");
    assert_eq!(v["needs"], serde_json::json!(["js"]));
    assert!(v.get("out").map(|x| x.is_null()).unwrap_or(true));
}

#[test]
fn envelope_records_both_requested_and_final_urls() {
    let mut server = mockito::Server::new();
    let _m1 = server
        .mock("GET", "/start")
        .with_status(301)
        .with_header("location", "/landed")
        .create();
    let _m2 = server
        .mock("GET", "/landed")
        .with_status(200)
        .with_body("done")
        .create();
    let start = format!("{}/start", server.url());
    let (code, out, _) = run_capture(&[&start, "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["url"]["requested"], start);
    assert!(v["url"]["final"].as_str().unwrap().ends_with("/landed"));
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
    let (code, out, _) = run_capture(&[&start, "--out", "links"]);
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
    let (code, out, _) = run_capture(&[&url, "--css", "--out", "text"]);
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
    let (code, out, _) = run_capture(&[&url, "--css", "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["out"], "shown");
}
