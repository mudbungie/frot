//! End-to-end zero-delay meta refresh (bl-08d8, decided bl-c262): the refresh
//! is followed as part of the one navigation — same session, same jar, same hop
//! budget as an HTTP redirect — and the envelope reports the landing exactly as
//! it does after a 3xx. Grammar and target selection are pinned in
//! `src/refresh/tests.rs`; this file pins the navigation.

use super::*;
use mockito::Matcher::Missing;

fn run_capture(args: &[&str]) -> (u8, Value) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let code = run_io(&argv, &mut out, &mut err);
    let out = String::from_utf8(out).unwrap();
    (
        code,
        serde_json::from_str(out.trim()).expect("envelope JSON"),
    )
}

fn refresh(content: &str) -> String {
    format!("<html><head><meta http-equiv=\"refresh\" content=\"{content}\"></head></html>")
}

#[test]
fn an_immediate_refresh_lands_on_its_target_like_a_redirect() {
    // The docs.astro.build shape: the root is nothing but the refresh.
    let mut server = mockito::Server::new();
    let root = format!("{}/", server.url());
    let _r = server
        .mock("GET", "/")
        .match_header("referer", Missing)
        .match_header("sec-fetch-site", "none")
        .with_body(refresh("0;url=/en/getting-started/"))
        .create();
    // The hop is a navigation the root document caused: it carries the root as
    // referrer and reads as same-origin, as the hop after a 302 does.
    let landing = server
        .mock("GET", "/en/getting-started/")
        .match_header("referer", root.as_str())
        .match_header("sec-fetch-site", "same-origin")
        .match_header("sec-fetch-dest", "document")
        .with_body("<title>Getting started</title><p>real docs</p>")
        .create();
    let (code, v) = run_capture(&[&server.url(), "--out", "text"]);
    landing.assert();
    assert_eq!(code, 0);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["out"], "real docs");
    assert_eq!(v["url"]["requested"], server.url());
    assert_eq!(v["url"]["final"], format!("{root}en/getting-started/"));
    assert_eq!(v["http"]["status"], 200);
}

#[test]
fn a_refresh_response_header_is_followed_like_the_meta() {
    // bl-e52a: the same rule in its HTTP spelling. The header claims the
    // document before its markup, so the meta's target is never requested and
    // the interim page's script never runs.
    let mut server = mockito::Server::new();
    let root = format!("{}/", server.url());
    let _r = server
        .mock("GET", "/")
        .with_header("refresh", "0; url=/real")
        .with_header("set-cookie", "seen=1; Path=/")
        .with_body(format!(
            "{}<script>new Image().src = '/beacon'</script>",
            refresh("0;url=/meta")
        ))
        .create();
    let meta = server.mock("GET", "/meta").expect(0).create();
    let landing = server
        .mock("GET", "/real")
        .match_header("referer", root.as_str())
        .match_header("cookie", "seen=1")
        .with_body("<p>header landing</p>")
        .create();
    let (code, v) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    landing.assert();
    meta.assert();
    assert_eq!(code, 0);
    assert_eq!(v["out"], "header landing");
    assert_eq!(v["url"]["final"], format!("{root}real"));
    assert_eq!(v["js"]["scripts"], 0);
}

#[test]
fn a_delayed_refresh_header_is_not_followed() {
    let mut server = mockito::Server::new();
    let _r = server
        .mock("GET", "/")
        .with_header("refresh", "5; url=/later")
        .with_body("<p>read me first</p>")
        .create();
    let later = server.mock("GET", "/later").expect(0).create();
    let (code, v) = run_capture(&[&server.url(), "--out", "text"]);
    later.assert();
    assert_eq!(code, 0);
    assert_eq!(v["out"], "read me first");
}

#[test]
fn a_delayed_refresh_is_not_followed() {
    let mut server = mockito::Server::new();
    let _r = server
        .mock("GET", "/")
        .with_body("<meta http-equiv=refresh content='3;url=/later'><p>read me first</p>")
        .create();
    let later = server.mock("GET", "/later").expect(0).create();
    let (code, v) = run_capture(&[&server.url(), "--out", "text"]);
    later.assert();
    assert_eq!(code, 0);
    assert_eq!(v["out"], "read me first");
    assert_eq!(v["url"]["final"], format!("{}/", server.url()));
}

#[test]
fn a_cookie_set_by_the_refreshing_page_rides_the_hop() {
    // The cookie-test idiom: set a cookie, reload (no URL), serve the page once
    // the cookie comes back. One jar across the whole navigation.
    let mut server = mockito::Server::new();
    let _gate = server
        .mock("GET", "/")
        .match_header("cookie", Missing)
        .with_header("set-cookie", "seen=1; Path=/")
        .with_body(refresh("0"))
        .create();
    let _page = server
        .mock("GET", "/")
        .match_header("cookie", "seen=1")
        .with_body("<p>welcome back</p>")
        .create();
    let (code, v) = run_capture(&[&server.url(), "--out", "text"]);
    assert_eq!(code, 0);
    assert_eq!(v["out"], "welcome back");
}

#[test]
fn a_cross_origin_refresh_carries_no_caller_headers_and_only_the_origin() {
    let mut other = mockito::Server::new();
    let mut server = mockito::Server::new();
    let _r = server
        .mock("GET", "/start")
        .match_header("authorization", "Bearer s3cret")
        .with_body(refresh(&format!("0; URL='{}/landed'", other.url())))
        .create();
    let landed = other
        .mock("GET", "/landed")
        .match_header("authorization", Missing)
        .match_header("referer", format!("{}/", server.url()).as_str())
        .with_body("<p>elsewhere</p>")
        .create();
    let start = format!("{}/start", server.url());
    let (code, v) = run_capture(&[
        &start,
        "-H",
        "Authorization: Bearer s3cret",
        "--out",
        "text",
    ]);
    landed.assert();
    assert_eq!(code, 0);
    assert_eq!(v["url"]["final"], format!("{}/landed", other.url()));
}

#[test]
fn a_refresh_loop_exhausts_the_redirect_budget() {
    let mut server = mockito::Server::new();
    // Ten requests, the budget an HTTP redirect loop gets — then the same error.
    let m = server
        .mock("GET", "/")
        .with_body(refresh("0"))
        .expect(10)
        .create();
    let (code, v) = run_capture(&[&server.url(), "--out", "text"]);
    m.assert();
    assert_eq!(code, 1);
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["kind"], "fetch.redirect");
    assert!(
        v["url"].get("final").is_none(),
        "nothing was landed on: {v}"
    );
}

#[test]
fn redirects_and_refreshes_spend_one_budget() {
    // `/` refreshes to `/r`, which 302s back: two requests a lap, so the shared
    // ten is five laps — not ten refreshes of ten redirects each.
    let mut server = mockito::Server::new();
    let page = server
        .mock("GET", "/")
        .with_body(refresh("0;url=/r"))
        .expect(5)
        .create();
    let hop = server
        .mock("GET", "/r")
        .with_status(302)
        .with_header("location", "/")
        .expect(5)
        .create();
    let (code, v) = run_capture(&[&server.url(), "--out", "text"]);
    page.assert();
    hop.assert();
    assert_eq!(code, 1);
    assert_eq!(v["error"]["kind"], "fetch.redirect");
}

#[test]
fn a_refresh_to_an_error_page_reports_that_response() {
    let mut server = mockito::Server::new();
    let _r = server
        .mock("GET", "/")
        .with_body(refresh("0;url=/gone"))
        .create();
    let _g = server.mock("GET", "/gone").with_status(404).create();
    let (code, v) = run_capture(&[&server.url(), "--out", "text"]);
    assert_eq!(code, 1);
    assert_eq!(v["error"]["kind"], "http.404");
    assert_eq!(v["url"]["final"], format!("{}/gone", server.url()));
}

#[test]
fn under_js_the_refreshing_page_is_never_run() {
    // The refresh is honoured for the document as fetched, before scripts: the
    // interim page's script (which would fetch `/beacon`) does not execute, and
    // the `js` block describes the landed page alone.
    let mut server = mockito::Server::new();
    let _r = server
        .mock("GET", "/")
        .with_body(
            "<meta http-equiv=refresh content='0;url=/app'>\
             <script>var x = new XMLHttpRequest(); x.open('GET', '/beacon'); x.send();</script>",
        )
        .create();
    let beacon = server.mock("GET", "/beacon").expect(0).create();
    let _app = server
        .mock("GET", "/app")
        .with_body("<div id=root></div><script>document.getElementById('root').textContent = location.pathname;</script>")
        .create();
    let (code, v) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    beacon.assert();
    assert_eq!(code, 0);
    assert_eq!(v["out"], "/app", "{v}");
    assert_eq!(v["js"]["scripts"], 1);
}

#[test]
fn a_script_inserted_refresh_stays_a_no_op() {
    // Script-chosen navigation is still refused (js.md §11): only the markup
    // frot fetched can direct the navigation.
    let mut server = mockito::Server::new();
    let _r = server
        .mock("GET", "/")
        .with_body(
            "<p>here</p><script>var m = document.createElement('meta');\
             m.setAttribute('http-equiv', 'refresh');\
             m.setAttribute('content', '0;url=/away'); document.head.appendChild(m);</script>",
        )
        .create();
    let away = server.mock("GET", "/away").expect(0).create();
    let (code, v) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    away.assert();
    assert_eq!(code, 0);
    assert_eq!(v["out"], "here");
}

#[test]
fn a_local_document_refreshes_to_its_sibling_and_a_local_loop_is_bounded() {
    let mut dir = std::env::temp_dir();
    dir.push(format!("frot-run-refresh-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file_url = |name: &str| {
        url::Url::from_file_path(dir.join(name))
            .unwrap()
            .to_string()
    };
    std::fs::write(dir.join("index.html"), refresh("0;url=real.html")).unwrap();
    std::fs::write(dir.join("real.html"), "<p>local page</p>").unwrap();
    std::fs::write(dir.join("loop.html"), refresh("0")).unwrap();
    let (code, v) = run_capture(&[&file_url("index.html"), "--out", "text"]);
    assert_eq!(code, 0);
    assert_eq!(v["out"], "local page");
    assert_eq!(v["url"]["final"], file_url("real.html"));
    let (code, v) = run_capture(&[&file_url("loop.html"), "--out", "text"]);
    assert_eq!(code, 1);
    assert_eq!(v["error"]["kind"], "fetch.redirect");
    std::fs::remove_dir_all(&dir).unwrap();
}
