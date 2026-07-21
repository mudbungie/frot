//! End-to-end `-H` scoping: the page request carries the caller's headers;
//! `--css` stylesheet subfetches carry them only when same-origin.

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

const HIDING_PAGE: &str =
    r#"<link rel="stylesheet" href="{sheet}"><div class="hide">secret</div><p>visible</p>"#;

#[test]
fn same_origin_sheet_receives_the_headers() {
    let mut server = mockito::Server::new();
    let _page = server
        .mock("GET", "/")
        .match_header("cookie", "auth=1")
        .with_body(HIDING_PAGE.replace("{sheet}", "/hide.css"))
        .create();
    // The sheet mock only matches when the cookie arrives; without it the
    // fetch 501s and the sheet is skipped, leaving "secret" visible.
    let _sheet = server
        .mock("GET", "/hide.css")
        .match_header("cookie", "auth=1")
        .with_body(".hide{display:none}")
        .create();
    let (code, out, _) = run_capture(&[
        &server.url(),
        "-H",
        "Cookie: auth=1",
        "--css",
        "--out",
        "text",
    ]);
    assert_eq!(code, 0);
    let text = parse_envelope(&out)["out"].as_str().unwrap().to_string();
    assert!(
        !text.contains("secret"),
        "cookie not sent to sheet: {}",
        text
    );
    assert!(text.contains("visible"));
}

#[test]
fn cross_origin_sheet_does_not_receive_the_headers() {
    let mut sheets = mockito::Server::new();
    let sheet_url = format!("{}/hide.css", sheets.url());
    // Matches only when NO cookie header arrives — proving the credential
    // stayed home while the sheet still applied.
    let _sheet = sheets
        .mock("GET", "/hide.css")
        .match_header("cookie", mockito::Matcher::Missing)
        .with_body(".hide{display:none}")
        .create();
    let mut server = mockito::Server::new();
    let _page = server
        .mock("GET", "/")
        .match_header("cookie", "auth=1")
        .with_body(HIDING_PAGE.replace("{sheet}", &sheet_url))
        .create();
    let (code, out, _) = run_capture(&[
        &server.url(),
        "-H",
        "Cookie: auth=1",
        "--css",
        "--out",
        "text",
    ]);
    assert_eq!(code, 0);
    let text = parse_envelope(&out)["out"].as_str().unwrap().to_string();
    assert!(
        !text.contains("secret"),
        "cross-origin sheet was not fetched anonymously: {}",
        text
    );
}

#[test]
fn header_usage_errors_exit_two_without_envelope() {
    let (code, out, err) = run_capture(&["https://x/", "-H", "nope", "--out", "text"]);
    assert_eq!(code, 2);
    assert!(out.is_empty());
    assert!(err.contains("bad header"));

    let (code, _, err) = run_capture(&["file:///x.html", "-H", "A: 1", "--out", "text"]);
    assert_eq!(code, 2);
    assert!(err.contains("file://"));
}
