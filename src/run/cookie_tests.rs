//! End-to-end cookie jar (bl-6dad): the one per-invocation jar shared by the
//! transport and `document.cookie`. A redirect-set cookie rides the landing
//! request; a final-response cookie is visible to JS iff non-HttpOnly; a JS write
//! feeds a later same-origin GET; parallel subresource reads are race-safe; a
//! fresh invocation starts empty; a declared challenge exits before any replay.

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

fn text_of(out: &str) -> String {
    parse_envelope(out)["out"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

#[test]
fn a_redirect_set_cookie_rides_the_landing_request() {
    let mut server = mockito::Server::new();
    // The redirect sets a host cookie; the landing page is served only when it
    // comes back — the one navigation must carry it.
    let _r = server
        .mock("GET", "/")
        .with_status(302)
        .with_header("set-cookie", "sid=ok; Path=/")
        .with_header("location", "/landing")
        .create();
    let _l = server
        .mock("GET", "/landing")
        .match_header("cookie", "sid=ok")
        .with_body("<p>welcome</p>")
        .create();
    let (code, out, _) = run_capture(&[&server.url(), "--out", "text"]);
    assert_eq!(code, 0);
    assert!(
        text_of(&out).contains("welcome"),
        "landing not served: {out}"
    );
}

#[test]
fn a_fresh_invocation_starts_with_no_cookies() {
    // No Set-Cookie anywhere: the page is served only when NO cookie arrives,
    // proving the jar is born empty and nothing leaked from another run.
    let mut server = mockito::Server::new();
    let _p = server
        .mock("GET", "/")
        .match_header("cookie", mockito::Matcher::Missing)
        .with_body("<p>anon</p>")
        .create();
    let (code, out, _) = run_capture(&[&server.url(), "--out", "text"]);
    assert_eq!(code, 0);
    assert!(text_of(&out).contains("anon"));
}

const JS_PAGE: &str = "<html><body><div id='c'></div><div id='a'></div><script>\
document.getElementById('c').textContent = 'C:' + document.cookie;\
document.cookie = 'written=3';\
var x = new XMLHttpRequest(); x.open('GET','/api'); x.send();\
document.getElementById('a').textContent = 'A:' + x.responseText;\
</script></body></html>";

#[test]
fn document_cookie_hides_http_only_and_a_js_write_feeds_a_later_get() {
    let mut server = mockito::Server::new();
    let _p = server
        .mock("GET", "/")
        .with_header("set-cookie", "vis=1")
        .with_header("set-cookie", "secret=2; HttpOnly")
        .with_body(JS_PAGE)
        .create();
    // The XHR must carry the JS-written cookie; the mock only answers when it does.
    let _api = server
        .mock("GET", "/api")
        .match_header("cookie", mockito::Matcher::Regex("written=3".into()))
        .with_body("APIOK")
        .create();
    let (code, out, _) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    assert_eq!(code, 0);
    let text = text_of(&out);
    // `document.cookie` saw the normal cookie but never the HttpOnly one.
    assert!(text.contains("C:vis=1"), "cookie not visible to JS: {text}");
    assert!(!text.contains("secret"), "HttpOnly leaked into JS: {text}");
    // The JS write reached the wire on the later same-origin GET.
    assert!(
        text.contains("A:APIOK"),
        "JS-written cookie did not ride: {text}"
    );
}

const CSS_PAGE: &str = "<link rel='stylesheet' href='/s1.css'>\
<link rel='stylesheet' href='/s2.css'>\
<link rel='stylesheet' href='/s3.css'>\
<div class='hide'>secret</div><p>shown</p>";

#[test]
fn parallel_subresource_reads_share_the_jar_race_safe() {
    let mut server = mockito::Server::new();
    let _p = server
        .mock("GET", "/")
        .with_header("set-cookie", "sid=ok; Path=/")
        .with_body(CSS_PAGE)
        .create();
    // Three stylesheets fetched concurrently (up to six gather threads); each is
    // served only when the shared jar hands it the cookie — a race would drop one
    // sheet and leave "secret" visible.
    let mut sheets = Vec::new();
    for n in 1..=3 {
        sheets.push(
            server
                .mock("GET", format!("/s{n}.css").as_str())
                .match_header("cookie", "sid=ok")
                .with_body(".hide{display:none}")
                .create(),
        );
    }
    let (code, out, _) = run_capture(&[&server.url(), "--css", "--out", "text"]);
    assert_eq!(code, 0);
    let text = text_of(&out);
    assert!(
        !text.contains("secret"),
        "a sheet missed the cookie: {text}"
    );
    assert!(text.contains("shown"));
}

#[test]
fn a_challenge_responses_own_cookie_earns_no_second_request() {
    // The challenge response's Set-Cookie reaches the jar, but only the page's
    // own *script* changing the request warrants a second navigation
    // (challenge.md §4.1). The server's cookie alone — with `--js`, and a body
    // that runs nothing — is not that: one request, never a retry.
    let mut server = mockito::Server::new();
    let page = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("retry-after", "0")
        .with_header("set-cookie", "chal=x")
        .with_body("<html><body>please wait</body></html>")
        .expect(1)
        .create();
    let (code, out, _) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    assert_eq!(code, 0);
    let env = parse_envelope(&out);
    assert_eq!(env["status"], "needs");
    assert_eq!(env["needs"], serde_json::json!(["human"]));
    assert_eq!(env["challenge"]["rounds"], 0);
    assert!(env.get("out").is_none(), "challenge produced output: {out}");
    page.assert(); // exactly one request — no retry replayed the challenge cookie
}

/// A declared challenge whose body is `body`, served to a request carrying no
/// cookie — so a second navigation would be a second hit on this mock.
fn gate(server: &mut mockito::Server, body: &str) -> mockito::Mock {
    server
        .mock("GET", "/")
        .match_header("cookie", mockito::Matcher::Missing)
        .with_status(403)
        .with_header("cf-mitigated", "challenge")
        .with_header("set-cookie", "srv=1; Path=/")
        .with_body(body)
        .expect(1)
        .create()
}

#[test]
fn a_cookie_set_by_a_script_src_response_earns_no_round() {
    // challenge.md §12 (bl-cc4c): the response to the page's `<script src>`
    // sets a cookie that rides the re-issue, but the script did nothing — the
    // server changed the request, so no round, and the challenge is asked once.
    let mut server = mockito::Server::new();
    let g = gate(&mut server, "<script src='/c.js'></script>");
    let js = server
        .mock("GET", "/c.js")
        .with_header("set-cookie", "cf=1; Path=/")
        .with_body("var idle = 1;")
        .create();
    let (_, out, _) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    g.assert();
    js.assert();
    let env = parse_envelope(&out);
    assert_eq!(env["needs"], serde_json::json!(["human"]), "{out}");
    assert_eq!(env["challenge"]["rounds"], 0);
}

#[test]
fn a_cookie_set_by_the_scripts_own_fetch_earns_the_round() {
    // The same cookie, set on the response to a GET the script issued, is the
    // script's doing: the request changed because of it, so one round.
    let mut server = mockito::Server::new();
    let g = gate(&mut server, "<script>fetch('/t');</script>");
    let t = server
        .mock("GET", "/t")
        .with_header("set-cookie", "cf=1; Path=/")
        .with_body("{}")
        .create();
    let real = server
        .mock("GET", "/")
        .match_header("cookie", "srv=1; cf=1")
        .with_body("<p>the page</p>")
        .expect(1)
        .create();
    let (_, out, _) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    g.assert();
    t.assert();
    real.assert();
    let env = parse_envelope(&out);
    assert_eq!(env["out"], "the page", "{out}");
    assert_eq!(env["challenge"]["rounds"], 1);
}

#[test]
fn a_script_rewriting_the_servers_cookie_unchanged_earns_no_round() {
    // The script wrote, but the request is identical: never re-sent (§4.1).
    let mut server = mockito::Server::new();
    let g = gate(
        &mut server,
        "<script>document.cookie = 'srv=1; path=/';</script>",
    );
    let (_, out, _) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    g.assert();
    assert_eq!(parse_envelope(&out)["challenge"]["rounds"], 0, "{out}");
}
