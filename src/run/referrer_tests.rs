//! `document.referrer` end to end (bl-dc63): the Referer the document's
//! navigation put on the wire, read back by the page — the refreshing document
//! after a followed meta refresh, nothing after a typed navigation — through a
//! redirect too, where Firefox keeps the request's own referrer (bl-b298).

use super::*;

fn run_capture(args: &[&str]) -> Value {
    let mut out = Vec::new();
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    run_io(&argv, &mut out, &mut Vec::new());
    serde_json::from_str(String::from_utf8(out).unwrap().trim()).expect("envelope JSON")
}

#[test]
fn the_landing_document_reads_the_referrer_its_navigation_sent() {
    // bl-dc63: `document.referrer` is the Referer the wire carried — the
    // refreshing document here — and the typed navigation's own is empty.
    let mut server = mockito::Server::new();
    let root = format!("{}/", server.url());
    let page = "<title> Got \n it </title><p id=o></p><script>\
                document.getElementById('o').textContent=\
                JSON.stringify([document.referrer, document.title])</script>";
    let _r = server
        .mock("GET", "/")
        .with_body("<meta http-equiv=refresh content=\"0;url=/land\">")
        .create();
    let _l = server.mock("GET", "/land").with_body(page).create();
    let _t = server.mock("GET", "/typed").with_body(page).create();
    let v = run_capture(&[&root, "--js", "--out", "text"]);
    assert_eq!(v["out"], format!("[\"{root}\",\"Got it\"]"));
    let v = run_capture(&[&format!("{root}typed"), "--js", "--out", "text"]);
    assert_eq!(v["out"], "[\"\",\"Got it\"]");
}

#[test]
fn a_redirect_never_becomes_the_referrer() {
    // bl-b298, Firefox 153: typed → 302 → landing sends no Referer (and the
    // page reads ''); refresh → 302 → landing sends the refreshing document.
    use mockito::Matcher;
    let mut server = mockito::Server::new();
    let root = server.url();
    let page = "<p id=o></p><script>document.getElementById('o')\
                .textContent=JSON.stringify([document.referrer])</script>";
    let hop = |from: &str, to: &str| (from.to_string(), format!("{root}{to}"));
    let mut mocks = Vec::new();
    for (from, to) in [hop("/r", "/land"), hop("/r2", "/land2")] {
        let m = server.mock("GET", from.as_str()).with_status(302);
        mocks.push(m.with_header("location", &to).create());
    }
    let refresh = "<meta http-equiv=refresh content=\"0;url=/r2\">";
    mocks.push(server.mock("GET", "/refresh").with_body(refresh).create());
    let land = server
        .mock("GET", "/land")
        .match_header("referer", Matcher::Missing);
    let land = land.match_header("sec-fetch-site", "none");
    mocks.push(land.with_body(page).create());
    let land2 = server
        .mock("GET", "/land2")
        .match_header("referer", Matcher::Exact(format!("{root}/refresh")));
    let land2 = land2.match_header("sec-fetch-user", Matcher::Missing);
    mocks.push(land2.with_body(page).create());
    let v = run_capture(&[&format!("{root}/r"), "--js", "--out", "text"]);
    assert_eq!(v["out"], "[\"\"]");
    let v = run_capture(&[&format!("{root}/refresh"), "--js", "--out", "text"]);
    assert_eq!(v["out"], format!("[\"{root}/refresh\"]"));
}
