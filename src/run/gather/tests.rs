//! Gather-phase tests: href resolution, concurrency, cascade order, the
//! aggregate budget, and pool/cache behaviour through the shared session. The
//! timing assertions carry wide margins deliberately — they must distinguish
//! concurrent from serial, not measure either.

use super::*;
use crate::fetch::POOL_PER_HOST;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Arc;

/// A stub CSS origin. Every connection sleeps `delay`, then answers with
/// `body` — or, when `body` is `None`, never answers at all (the dead-host
/// case: the connection is accepted, so there is nothing to fail fast on).
fn stub_origin(delay: Duration, body: Option<&'static str>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || serve(stream, delay, body));
        }
    });
    base
}

fn serve(mut stream: TcpStream, delay: Duration, body: Option<&'static str>) {
    let _ = stream.read(&mut [0u8; 1024]);
    std::thread::sleep(delay);
    let Some(body) = body else {
        std::thread::sleep(Duration::from_secs(30));
        return;
    };
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/css\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
}

fn sheets(base: &str, n: usize) -> Vec<String> {
    (0..n).map(|i| format!("{base}/{i}.css")).collect()
}

const GENEROUS: Duration = Duration::from_secs(30);

fn plain() -> FetchSession {
    FetchSession::new(Vec::new())
}

/// A keep-alive CSS origin that counts accepted TCP connections, so a gather's
/// pool reuse is observable: workers on one bounded pool open at most
/// `POOL_PER_HOST` connections, and later same-origin fetches reuse them.
fn counting_origin() -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let conns = Arc::new(AtomicUsize::new(0));
    let c = conns.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            c.fetch_add(1, AtomicOrdering::SeqCst);
            std::thread::spawn(move || keepalive(stream));
        }
    });
    (base, conns)
}

fn keepalive(stream: TcpStream) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = stream;
    loop {
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => return,
                Ok(_) if line == "\r\n" => break,
                Ok(_) => {}
            }
        }
        let resp = "HTTP/1.1 200 OK\r\nContent-Type: text/css\r\nContent-Length: 3\r\n\r\na{}";
        if writer.write_all(resp.as_bytes()).is_err() {
            return;
        }
    }
}

#[test]
fn sheets_are_fetched_concurrently_not_serially() {
    let base = stub_origin(Duration::from_millis(300), Some("a{}"));
    let hrefs = sheets(&base, POOL_PER_HOST);
    let started = Instant::now();
    let got = gather_within(&hrefs, &base, &plain(), GENEROUS);
    let elapsed = started.elapsed();
    assert_eq!(got.len(), POOL_PER_HOST);
    // Serial would be POOL_PER_HOST x 300ms = 1.8s; concurrent is ~300ms.
    assert!(
        elapsed < Duration::from_millis(1500),
        "serial: {:?}",
        elapsed
    );
}

#[test]
fn six_workers_share_one_bounded_pool() {
    let (base, conns) = counting_origin();
    let session = plain();
    // A full wave of six sheets, then three more sequential same-origin fetches.
    // Six isolated agents (the old fetch_within-per-URL) would open nine
    // connections; one bounded pool opens at most POOL_PER_HOST and reuses them.
    let wave = gather_within(&sheets(&base, POOL_PER_HOST), &base, &session, GENEROUS);
    assert_eq!(wave.len(), POOL_PER_HOST);
    for i in 0..3 {
        session
            .subresource(&format!("{base}/x{i}.css"), &base, Intent::Style, GENEROUS)
            .unwrap();
    }
    let n = conns.load(AtomicOrdering::SeqCst);
    assert!(n <= POOL_PER_HOST, "opened {n} connections");
}

#[test]
fn an_unchanged_sheet_is_gathered_once_across_passes() {
    let (base, conns) = counting_origin();
    let session = plain();
    let hrefs = sheets(&base, 1);
    // The `--css --js` shape: the JS-phase gather and the final cascade both ask
    // for the same sheet; the second pass is a session-cache hit — no new
    // connection, no second fetch.
    assert_eq!(
        gather_within(&hrefs, &base, &session, GENEROUS),
        vec!["a{}".to_string()]
    );
    assert_eq!(
        gather_within(&hrefs, &base, &session, GENEROUS),
        vec!["a{}".to_string()]
    );
    assert_eq!(conns.load(AtomicOrdering::SeqCst), 1);
}

#[test]
fn bodies_keep_document_order_when_they_arrive_out_of_order() {
    let slow = stub_origin(Duration::from_millis(400), Some("first{}"));
    let fast = stub_origin(Duration::from_millis(0), Some("second{}"));
    let hrefs = vec![format!("{slow}/a.css"), format!("{fast}/b.css")];
    // The second sheet lands well before the first, but cascade order is
    // source order, so the slow one must still come back first.
    let got = gather_within(&hrefs, &slow, &plain(), GENEROUS);
    assert_eq!(got, vec!["first{}".to_string(), "second{}".to_string()]);
}

#[test]
fn budget_bounds_the_phase_when_a_host_never_answers() {
    let base = stub_origin(Duration::from_millis(0), None);
    // More sheets than workers, so the queued ones meet an expired deadline
    // rather than a free slot.
    let hrefs = sheets(&base, POOL_PER_HOST + 1);
    let started = Instant::now();
    let got = gather_within(&hrefs, &base, &plain(), Duration::from_millis(400));
    let elapsed = started.elapsed();
    // Best-effort: a tripped budget yields whatever arrived (nothing here)
    // and never fails the run.
    assert!(got.is_empty());
    // The old ceiling was 15s per request, serially. The phase now ends at
    // its own budget regardless of how many sheets are outstanding.
    assert!(elapsed < Duration::from_secs(3), "unbounded: {:?}", elapsed);
}

#[test]
fn no_sheets_is_no_work() {
    assert!(gather_within(&[], "http://example.com/", &plain(), GENEROUS).is_empty());
}

#[test]
fn external_hrefs_resolves_only_stylesheet_links() {
    let doc = crate::dom::Document::parse(
        "<link rel='stylesheet' href='/a.css'>\
         <link rel='icon' href='/favicon.ico'>\
         <link rel='stylesheet'>\
         <link rel='stylesheet' href=''>\
         <link rel='Stylesheet preload' href='b.css'>",
    );
    let got = external_hrefs(&doc, "http://example.com/dir/page");
    assert_eq!(
        got,
        vec![
            "http://example.com/a.css".to_string(),
            "http://example.com/dir/b.css".to_string(),
        ]
    );
}

#[test]
fn external_hrefs_empty_when_base_unparseable() {
    let doc = crate::dom::Document::parse("<link rel='stylesheet' href='/a.css'>");
    assert!(external_hrefs(&doc, "not a url").is_empty());
}
