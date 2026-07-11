//! Subfetch cache tests (js.md §6). The network paths use `mockito` (a local
//! server — never the real network, per the repo test rule) and `file://` temp
//! files; every policy branch — resolve, cache hit, cap, remote→local block,
//! same/cross-origin headers, 2xx/non-2xx, file read, transport failure — is a
//! real behavior, no defensive dead code.

use std::fs;
use std::path::PathBuf;

use mockito::Matcher;

use super::{Outcome, Subfetch};

fn got(o: Outcome) -> super::Frozen {
    match o {
        Outcome::Got(f) => f,
        Outcome::Failed(m) => panic!("expected Got, failed: {m}"),
    }
}

fn failed(o: Outcome) -> String {
    match o {
        Outcome::Failed(m) => m,
        Outcome::Got(_) => panic!("expected Failed, got a response"),
    }
}

/// A unique temp directory for the `file://` cases, cleaned by the caller.
fn tmpdir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("frot-sf-{tag}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn file_base(dir: &std::path::Path) -> String {
    url::Url::from_file_path(dir.join("page.html")).unwrap().to_string()
}

#[test]
fn a_2xx_get_is_frozen_and_ok() {
    let mut server = mockito::Server::new();
    let m = server.mock("GET", "/r").with_status(200).with_body("hi").create();
    let mut sf = Subfetch::new(&server.url(), Vec::new());
    let f = got(sf.get("/r"));
    assert!(f.ok);
    assert_eq!(f.status, 200);
    assert_eq!(f.body, "hi");
    m.assert();
}

#[test]
fn a_non_2xx_get_freezes_but_is_not_ok() {
    let mut server = mockito::Server::new();
    let _m = server.mock("GET", "/r").with_status(404).with_body("no").create();
    let mut sf = Subfetch::new(&server.url(), Vec::new());
    let f = got(sf.get("/r"));
    assert!(!f.ok);
    assert_eq!(f.status, 404);
}

#[test]
fn a_url_is_fetched_at_most_once_then_frozen() {
    let mut server = mockito::Server::new();
    // Exactly one network hit even across two gets — the second is served frozen.
    let m = server.mock("GET", "/r").with_body("once").expect(1).create();
    let mut sf = Subfetch::new(&server.url(), Vec::new());
    assert_eq!(got(sf.get("/r")).body, "once");
    assert_eq!(got(sf.get("/r")).body, "once");
    m.assert();
}

#[test]
fn past_the_cap_a_new_url_is_refused() {
    let mut server = mockito::Server::new();
    let _a = server.mock("GET", "/a").with_body("a").create();
    // Cap of 1: the first URL spends it, the second is refused before any fetch.
    let mut sf = Subfetch::with_cap(&server.url(), Vec::new(), 1);
    assert_eq!(got(sf.get("/a")).body, "a");
    assert!(failed(sf.get("/b")).contains("cap reached"));
}

#[test]
fn same_origin_requests_carry_the_headers() {
    let mut server = mockito::Server::new();
    // The mock matches only when the -H header rode along.
    let _m = server
        .mock("GET", "/r")
        .match_header("x-frot", "1")
        .with_status(200)
        .create();
    let mut sf = Subfetch::new(&server.url(), vec![("X-Frot".into(), "1".into())]);
    assert!(got(sf.get("/r")).ok);
}

#[test]
fn cross_origin_requests_drop_the_headers() {
    let mut page = mockito::Server::new();
    let mut other = mockito::Server::new();
    // The other origin matches only when the credential header is absent.
    let _m = other
        .mock("GET", "/r")
        .match_header("x-frot", Matcher::Missing)
        .with_status(200)
        .create();
    let mut sf = Subfetch::new(&page.url(), vec![("X-Frot".into(), "1".into())]);
    let _keep = &mut page; // the page server must outlive the borrow of its url
    assert!(got(sf.get(&format!("{}/r", other.url()))).ok);
}

#[test]
fn a_file_read_is_statusless_and_ok() {
    let dir = tmpdir("read");
    fs::write(dir.join("mod.js"), "export const x = 1;").unwrap();
    let mut sf = Subfetch::new(&file_base(&dir), Vec::new());
    let f = got(sf.get("mod.js"));
    assert!(f.ok);
    assert_eq!(f.status, 0);
    assert_eq!(f.body, "export const x = 1;");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_transport_failure_is_a_failed_outcome() {
    let dir = tmpdir("miss");
    let mut sf = Subfetch::new(&file_base(&dir), Vec::new());
    // The sibling file does not exist — a transport error, not a refusal.
    assert!(!failed(sf.get("missing.js")).is_empty());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_remote_page_may_not_reach_local_files() {
    let mut sf = Subfetch::new("https://example.com/", Vec::new());
    assert!(failed(sf.get("file:///etc/passwd")).contains("remote"));
}

#[test]
fn an_unresolvable_spec_fails_without_fetching() {
    let mut sf = Subfetch::new("https://example.com/", Vec::new());
    // An unterminated IPv6 literal cannot resolve — refused before any network.
    assert!(!failed(sf.get("https://[invalid")).is_empty());
}

#[test]
fn a_bogus_page_url_fails_every_fetch() {
    // env.url is always a real final URL in production, but the `location` shim
    // tolerates a bogus one; subfetch must too — it fails rather than panics.
    let mut sf = Subfetch::new("not a url", Vec::new());
    assert!(!failed(sf.get("x")).is_empty());
}
