//! Gather-phase tests: href resolution, concurrency, cascade order, and the
//! aggregate budget. The timing assertions carry wide margins deliberately —
//! they must distinguish concurrent from serial, not measure either.

use super::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

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

#[test]
fn sheets_are_fetched_concurrently_not_serially() {
    let base = stub_origin(Duration::from_millis(300), Some("a{}"));
    let hrefs = sheets(&base, MAX_IN_FLIGHT);
    let started = Instant::now();
    let got = gather_within(&hrefs, &base, &[], GENEROUS);
    let elapsed = started.elapsed();
    assert_eq!(got.len(), MAX_IN_FLIGHT);
    // Serial would be MAX_IN_FLIGHT x 300ms = 1.8s; concurrent is ~300ms.
    assert!(elapsed < Duration::from_millis(1500), "serial: {:?}", elapsed);
}

#[test]
fn bodies_keep_document_order_when_they_arrive_out_of_order() {
    let slow = stub_origin(Duration::from_millis(400), Some("first{}"));
    let fast = stub_origin(Duration::from_millis(0), Some("second{}"));
    let hrefs = vec![format!("{slow}/a.css"), format!("{fast}/b.css")];
    // The second sheet lands well before the first, but cascade order is
    // source order, so the slow one must still come back first.
    let got = gather_within(&hrefs, &slow, &[], GENEROUS);
    assert_eq!(got, vec!["first{}".to_string(), "second{}".to_string()]);
}

#[test]
fn budget_bounds_the_phase_when_a_host_never_answers() {
    let base = stub_origin(Duration::from_millis(0), None);
    // More sheets than workers, so the queued ones meet an expired deadline
    // rather than a free slot.
    let hrefs = sheets(&base, MAX_IN_FLIGHT + 1);
    let started = Instant::now();
    let got = gather_within(&hrefs, &base, &[], Duration::from_millis(400));
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
    assert!(gather_within(&[], "http://example.com/", &[], GENEROUS).is_empty());
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
