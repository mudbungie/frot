//! Subfetch cache tests (js.md §6). The network paths use `mockito` (a local
//! server — never the real network, per the repo test rule) and `file://` temp
//! files; every policy branch — resolve, cache hit, the deadline and byte-pool
//! bounds, remote→local block, same/cross-origin headers, 2xx/non-2xx, file
//! read, transport failure — is a real behavior, no defensive dead code.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use mockito::Matcher;

use crate::js::engine::{Deadline, Engine, JS_MEM_LIMIT};

use super::{Outcome, Subfetch};

/// The shipping cache outside a run: no armed window, shipping byte pool.
fn open(base: &str, headers: Vec<(String, String)>) -> Subfetch {
    Subfetch::new(base, headers, Deadline::never())
}

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
    url::Url::from_file_path(dir.join("page.html"))
        .unwrap()
        .to_string()
}

#[test]
fn a_2xx_get_is_frozen_and_ok() {
    let mut server = mockito::Server::new();
    let m = server
        .mock("GET", "/r")
        .with_status(200)
        .with_body("hi")
        .create();
    let mut sf = open(&server.url(), Vec::new());
    let f = got(sf.get("/r"));
    assert!(f.ok);
    assert_eq!(f.status, 200);
    assert_eq!(f.body, "hi");
    m.assert();
}

#[test]
fn a_non_2xx_get_freezes_but_is_not_ok() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/r")
        .with_status(404)
        .with_body("no")
        .create();
    let mut sf = open(&server.url(), Vec::new());
    let f = got(sf.get("/r"));
    assert!(!f.ok);
    assert_eq!(f.status, 404);
}

#[test]
fn a_url_is_fetched_at_most_once_then_frozen() {
    let mut server = mockito::Server::new();
    // Exactly one network hit even across two gets — the second is served frozen.
    let m = server
        .mock("GET", "/r")
        .with_body("once")
        .expect(1)
        .create();
    let mut sf = open(&server.url(), Vec::new());
    assert_eq!(got(sf.get("/r")).body, "once");
    assert_eq!(got(sf.get("/r")).body, "once");
    m.assert();
}

#[test]
fn past_the_byte_pool_a_new_url_is_refused() {
    let mut server = mockito::Server::new();
    let _a = server.mock("GET", "/a").with_body("a").create();
    // Pool of 1 byte: the first response body spends it, the second URL is
    // refused before any fetch (js.md §6 — memory is the bound, not a count).
    let mut sf = Subfetch::with_budget(&server.url(), Vec::new(), Deadline::never(), 1);
    assert_eq!(got(sf.get("/a")).body, "a");
    assert!(failed(sf.get("/b")).contains("byte budget exhausted"));
}

#[test]
fn a_cache_hit_is_served_even_after_the_pool_is_spent() {
    let mut server = mockito::Server::new();
    let _a = server.mock("GET", "/a").with_body("aa").create();
    // The frozen response outlives its pool: a re-get spends nothing new.
    let mut sf = Subfetch::with_budget(&server.url(), Vec::new(), Deadline::never(), 1);
    assert_eq!(got(sf.get("/a")).body, "aa");
    assert_eq!(got(sf.get("/a")).body, "aa");
}

#[test]
fn past_the_deadline_dispatch_is_refused() {
    // The seam consults the engine's own armed window (js.md §6): a zero budget
    // armed is already expired, so no network is ever dispatched.
    let engine = Engine::with_limits(JS_MEM_LIMIT, Duration::ZERO);
    engine.arm();
    let mut sf = Subfetch::new("https://example.com/", Vec::new(), engine.deadline());
    assert!(failed(sf.get("/late.js")).contains("run budget exhausted"));
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
    let mut sf = open(&server.url(), vec![("X-Frot".into(), "1".into())]);
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
    let mut sf = open(&page.url(), vec![("X-Frot".into(), "1".into())]);
    let _keep = &mut page; // the page server must outlive the borrow of its url
    assert!(got(sf.get(&format!("{}/r", other.url()))).ok);
}

#[test]
fn a_file_read_is_statusless_and_ok() {
    let dir = tmpdir("read");
    fs::write(dir.join("mod.js"), "export const x = 1;").unwrap();
    let mut sf = open(&file_base(&dir), Vec::new());
    let f = got(sf.get("mod.js"));
    assert!(f.ok);
    assert_eq!(f.status, 0);
    assert_eq!(f.body, "export const x = 1;");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_transport_failure_is_a_failed_outcome() {
    let dir = tmpdir("miss");
    let mut sf = open(&file_base(&dir), Vec::new());
    // The sibling file does not exist — a transport error, not a refusal.
    assert!(!failed(sf.get("missing.js")).is_empty());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_remote_page_may_not_reach_local_files() {
    let mut sf = open("https://example.com/", Vec::new());
    assert!(failed(sf.get("file:///etc/passwd")).contains("remote"));
}

#[test]
fn an_unresolvable_spec_fails_without_fetching() {
    let mut sf = open("https://example.com/", Vec::new());
    // An unterminated IPv6 literal cannot resolve — refused before any network.
    assert!(!failed(sf.get("https://[invalid")).is_empty());
}

#[test]
fn a_bogus_page_url_fails_every_fetch() {
    // env.url is always a real final URL in production, but the `location` shim
    // tolerates a bogus one; subfetch must too — it fails rather than panics.
    let mut sf = open("not a url", Vec::new());
    assert!(!failed(sf.get("x")).is_empty());
}
