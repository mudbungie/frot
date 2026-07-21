//! [`FetchSession`] request-policy tests: the navigation header set and its
//! `-H` layering, the same-origin credential scoping for subresources, and the
//! `file://` read path — the header/`-H`/scheme decisions the session owns.

use super::super::{MAX_BODY_BYTES, USER_AGENT};
use super::*;
use crate::envelope::kinds;

const GENEROUS: Duration = Duration::from_secs(15);

#[test]
fn navigation_sends_the_firefox_set_layered_under_h() {
    let mut server = mockito::Server::new();
    let m = server
        .mock("GET", "/")
        .match_header(
            "accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,\
             image/avif,image/webp,*/*;q=0.8",
        )
        .match_header("accept-language", "en-US,en;q=0.5")
        .match_header("sec-fetch-mode", "navigate")
        .match_header("user-agent", USER_AGENT)
        .with_body("ok")
        .create();
    FetchSession::new(Vec::new())
        .navigate(&server.url())
        .unwrap();
    m.assert();
}

#[test]
fn a_caller_header_overrides_a_navigation_default_without_duplicating() {
    let mut server = mockito::Server::new();
    // The override replaces the default Accept (an exact match would fail on a
    // duplicate) while the untouched Accept-Language default still rides.
    let m = server
        .mock("GET", "/")
        .match_header("accept", "application/json")
        .match_header("accept-language", "en-US,en;q=0.5")
        .match_header("user-agent", "custom/2")
        .with_body("ok")
        .create();
    let headers = vec![
        ("Accept".into(), "application/json".into()),
        ("User-Agent".into(), "custom/2".into()),
    ];
    FetchSession::new(headers).navigate(&server.url()).unwrap();
    m.assert();
}

#[test]
fn a_same_origin_subresource_carries_h_but_cross_origin_does_not() {
    let mut page = mockito::Server::new();
    let mut other = mockito::Server::new();
    let same = page
        .mock("GET", "/s")
        .match_header("x-frot", "1")
        .with_body("y")
        .create();
    let cross = other
        .mock("GET", "/s")
        .match_header("x-frot", mockito::Matcher::Missing)
        .with_body("y")
        .create();
    let s = FetchSession::new(vec![("X-Frot".into(), "1".into())]);
    let page_url = page.url();
    s.subresource(&format!("{page_url}/s"), &page_url, Intent::Style, GENEROUS)
        .unwrap();
    s.subresource(
        &format!("{}/s", other.url()),
        &page_url,
        Intent::Style,
        GENEROUS,
    )
    .unwrap();
    same.assert();
    cross.assert();
}

fn tmp_path(name: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("frot-session-{}-{}", std::process::id(), name));
    p
}

fn file_url(p: &std::path::Path) -> String {
    url::Url::from_file_path(p).unwrap().to_string()
}

fn read_file(url: &str) -> Result<FetchResult, FetchError> {
    FetchSession::new(Vec::new()).subresource(url, url, Intent::Style, GENEROUS)
}

#[test]
fn a_file_read_is_statusless_and_sniffs_charset() {
    let p = tmp_path("ok.html");
    let mut bytes = b"<meta charset=\"windows-1252\"><p>caf".to_vec();
    bytes.push(0xE9);
    bytes.extend_from_slice(b"</p>");
    std::fs::write(&p, &bytes).unwrap();
    let r = read_file(&file_url(&p)).unwrap();
    assert_eq!(r.status, None);
    assert!(r.headers.is_empty());
    assert_eq!(r.charset, "windows-1252");
    assert!(r.body.contains("café"));
    assert!(r.final_url.starts_with("file://"));
    std::fs::remove_file(&p).unwrap();
}

#[test]
fn a_missing_file_is_a_file_error() {
    let e = read_file(&file_url(&tmp_path("nope.html"))).unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_FILE);
}

#[test]
fn a_directory_is_a_file_error() {
    let p = tmp_path("dir");
    std::fs::create_dir_all(&p).unwrap();
    let e = read_file(&file_url(&p)).unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_FILE);
    std::fs::remove_dir(&p).unwrap();
}

#[test]
fn a_file_url_with_a_remote_host_is_a_url_error() {
    let e = read_file("file://remotehost/etc/hosts").unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_URL);
    assert!(e.message.contains("not a local file path"));
}

#[test]
fn an_oversized_file_is_refused_by_the_body_gate() {
    let p = tmp_path("huge.html");
    let f = std::fs::File::create(&p).unwrap();
    f.set_len(MAX_BODY_BYTES + 1).unwrap();
    drop(f);
    let e = read_file(&file_url(&p)).unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_BODY);
    assert!(e.message.contains("limit"));
    std::fs::remove_file(&p).unwrap();
}
