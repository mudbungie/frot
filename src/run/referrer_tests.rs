//! `document.referrer` end to end (bl-dc63): the Referer the document's
//! navigation put on the wire, read back by the page — the refreshing document
//! after a followed meta refresh, nothing after a typed navigation.

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
