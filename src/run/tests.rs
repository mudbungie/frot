//! The entry point's own surface: usage, exit codes, and the envelope
//! contract every view shares — `status`, `needs`, and the requested/final URL
//! pair. The per-view `out` payloads live in `view_tests` (bl-a68b).

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
fn hidden_fallback_copy_does_not_report_a_dead_shell_as_ok() {
    // bl-eeb4, field repro 2026-08-11: a shell whose only body text sat in a
    // `<div hidden>` returned `status:"ok"` with that unpainted string as the
    // whole impression. End to end it is a starved shell, in both recipes.
    const SHELL: &str = include_str!("../../tests/fixtures/needs/hidden-fallback-shell.html");
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .expect_at_least(1)
        .with_status(200)
        .with_body(SHELL)
        .create();
    let url = server.url();
    for args in [vec!["--out", "text"], vec!["--css", "--out", "text"]] {
        let mut argv = vec![url.as_str()];
        argv.extend_from_slice(&args);
        let (code, out, _) = run_capture(&argv);
        assert_eq!(code, 0);
        let v = parse_envelope(&out);
        assert_eq!(v["status"], "needs", "{args:?}: {out}");
        assert_eq!(v["needs"], serde_json::json!(["js"]), "{args:?}");
        assert!(v.get("out").is_none(), "{args:?}: {out}");
    }
}
