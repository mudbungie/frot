//! [`FetchSession`] tests: the shared connection pool (reuse within an
//! invocation, isolation across invocations), the invocation resource cache
//! (dedup), redirects, header/navigation policy, and `file://` — all against a
//! live loopback server or temp files, never the real network.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::*;
use crate::envelope::kinds;

const GENEROUS: Duration = Duration::from_secs(15);

/// A keep-alive HTTP/1.1 loopback origin that counts accepted TCP connections
/// and serves the same `ok` body for any request on a connection until the
/// client hangs up. The connection count is what proves pool reuse: two
/// requests that reuse a connection produce **one** accept.
fn counting_origin() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let conns = Arc::new(AtomicUsize::new(0));
    let c = conns.clone();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            c.fetch_add(1, Ordering::SeqCst);
            thread::spawn(move || serve_keepalive(stream));
        }
    });
    (base, conns)
}

fn serve_keepalive(stream: TcpStream) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = stream;
    loop {
        // Read one request head; a closed/half-open connection ends the loop,
        // otherwise a blank line terminates the head and we answer.
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => return,
                Ok(_) if line == "\r\n" => break,
                Ok(_) => {}
            }
        }
        let resp = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nok";
        if writer.write_all(resp.as_bytes()).is_err() {
            return;
        }
    }
}

fn get(s: &FetchSession, base: &str, path: &str) -> FetchResult {
    s.subresource(&format!("{base}{path}"), base, Intent::Style, GENEROUS)
        .unwrap()
}

#[test]
fn two_same_origin_requests_reuse_one_connection() {
    let (base, conns) = counting_origin();
    let s = FetchSession::new(Vec::new());
    assert_eq!(get(&s, &base, "/a").body, "ok");
    assert_eq!(get(&s, &base, "/b").body, "ok");
    // One pool, one warm connection: the second request rides the first's socket.
    assert_eq!(conns.load(Ordering::SeqCst), 1);
}

#[test]
fn separate_invocations_cannot_share_a_connection() {
    let (base, conns) = counting_origin();
    // Two independent sessions -> two pools -> two connections. State never
    // crosses an invocation boundary (VISION principle 1).
    get(&FetchSession::new(Vec::new()), &base, "/a");
    get(&FetchSession::new(Vec::new()), &base, "/a");
    assert_eq!(conns.load(Ordering::SeqCst), 2);
}

#[test]
fn a_subresource_is_fetched_once_then_served_from_cache() {
    let mut server = mockito::Server::new();
    // Exactly one network hit across two identical safe-GET subresource
    // requests: the second is a cache hit (`--css --js` fetches an unchanged
    // sheet once across the two gather passes).
    let m = server
        .mock("GET", "/s.css")
        .with_body("a{}")
        .expect(1)
        .create();
    let s = FetchSession::new(Vec::new());
    let u = format!("{}/s.css", server.url());
    assert_eq!(
        s.subresource(&u, &server.url(), Intent::Style, GENEROUS)
            .unwrap()
            .body,
        "a{}"
    );
    assert_eq!(
        s.subresource(&u, &server.url(), Intent::Style, GENEROUS)
            .unwrap()
            .body,
        "a{}"
    );
    m.assert();
}

#[test]
fn a_navigation_is_never_cached() {
    let mut server = mockito::Server::new();
    // Two navigations hit the network twice — the top-level document is the
    // impression, not a reusable resource.
    let m = server.mock("GET", "/").with_body("doc").expect(2).create();
    let s = FetchSession::new(Vec::new());
    assert_eq!(s.navigate(&server.url()).unwrap().body, "doc");
    assert_eq!(s.navigate(&server.url()).unwrap().body, "doc");
    m.assert();
}

#[test]
fn fetch_ok_reports_status_body_and_charset() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("content-type", "text/html; charset=utf-8")
        .with_body("hello")
        .create();
    let r = FetchSession::new(Vec::new())
        .navigate(&server.url())
        .unwrap();
    assert_eq!(r.status, Some(200));
    assert_eq!(r.body, "hello");
    assert_eq!(r.charset, "utf-8");
    assert!(r
        .headers
        .iter()
        .any(|(n, _)| n.eq_ignore_ascii_case("content-type")));
}

#[test]
fn a_404_is_read_not_errored() {
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(404)
        .with_body("not found")
        .create();
    let r = FetchSession::new(Vec::new())
        .navigate(&server.url())
        .unwrap();
    assert_eq!(r.status, Some(404));
    assert_eq!(r.body, "not found");
}

#[test]
fn redirects_are_followed_through_the_session() {
    let mut server = mockito::Server::new();
    let _m1 = server
        .mock("GET", "/start")
        .with_status(301)
        .with_header("location", "/landed")
        .create();
    let _m2 = server
        .mock("GET", "/landed")
        .with_status(200)
        .with_body("final")
        .create();
    let r = FetchSession::new(Vec::new())
        .navigate(&format!("{}/start", server.url()))
        .unwrap();
    assert_eq!(r.status, Some(200));
    assert_eq!(r.body, "final");
    assert!(r.final_url.ends_with("/landed"));
}

#[test]
fn a_dead_port_is_a_connect_or_timeout_error() {
    let e = FetchSession::new(Vec::new())
        .navigate("http://127.0.0.1:1/")
        .unwrap_err();
    assert!(
        e.kind == kinds::FETCH_CONNECT
            || e.kind == kinds::FETCH_TIMEOUT
            || e.kind == kinds::FETCH_BODY,
        "unexpected kind: {} ({})",
        e.kind,
        e.message,
    );
}

#[test]
fn a_truncated_body_is_a_fetch_error() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        for mut stream in listener.incoming().flatten() {
            let _ = stream.read(&mut [0u8; 1024]);
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 4096\r\n\r\nshort",
            );
        }
    });
    let e = FetchSession::new(Vec::new())
        .navigate(&format!("http://{addr}/"))
        .unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_BODY);
}

#[test]
fn an_invalid_url_is_a_url_error() {
    let e = FetchSession::new(Vec::new())
        .navigate("not a url")
        .unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_URL);
}
