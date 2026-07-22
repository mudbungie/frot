//! End-to-end proofs that `bl-08f6`'s concurrent initial-script warm preserves
//! the impression: external scripts still evaluate in document order even when
//! their fetches finish out of order, and a resource-heavy page is byte-identical
//! run to run despite nondeterministic completion order. Delayed subresources use
//! raw loopback origins (mockito has no per-mock delay), never the real network.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

use serde_json::Value;

use crate::run::run_io;

fn text_out(url: &str) -> Value {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let argv = [url, "--js", "--out", "text"].map(str::to_string);
    let code = run_io(&argv, &mut out, &mut err);
    assert_eq!(code, 0);
    serde_json::from_str(std::str::from_utf8(&out).unwrap().trim()).unwrap()
}

/// A one-shot raw origin serving a fixed JS body after `delay`, then closing —
/// the delayed-subresource stub the concurrency proofs need. Cross-origin
/// scripts carry no credentials, so a bare origin is enough.
fn js_origin(delay: Duration, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || {
                let mut s = stream;
                let _ = s.read(&mut [0u8; 1024]);
                std::thread::sleep(delay);
                let _ = write!(
                    s,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/javascript\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
            });
        }
    });
    base
}

#[test]
fn external_scripts_execute_in_source_order_despite_out_of_order_fetch() {
    // The first script's origin is slow, the second's is immediate — so the
    // second body arrives first. Warming fetches both concurrently, but
    // execution stays document-ordered: the div reads "12", never "21".
    let slow = js_origin(
        Duration::from_millis(300),
        "document.getElementById('x').textContent += '1'",
    );
    let fast = js_origin(
        Duration::from_millis(0),
        "document.getElementById('x').textContent += '2'",
    );
    let html = format!(
        "<html><body><div id='x'></div>\
         <script src='{slow}/s.js'></script>\
         <script src='{fast}/f.js'></script></body></html>"
    );
    let mut server = mockito::Server::new();
    let _m = server.mock("GET", "/").with_body(&html).create();
    let v = text_out(&server.url());
    assert_eq!(v["js"]["scripts"], 2);
    let text = v["out"].as_str().unwrap();
    assert!(text.contains("12"), "not source-ordered: {text:?}");
    assert!(!text.contains("21"), "reordered by completion: {text:?}");
}

#[test]
fn a_resource_heavy_page_yields_identical_output_across_runs() {
    // Control: several external scripts on slow/fast origins, run twice. Their
    // fetches complete in different orders run to run, yet the impression is
    // byte-identical — concurrency changes latency, never the output.
    let a = js_origin(
        Duration::from_millis(120),
        "document.getElementById('x').textContent += 'A'",
    );
    let b = js_origin(
        Duration::from_millis(0),
        "document.getElementById('x').textContent += 'B'",
    );
    let c = js_origin(
        Duration::from_millis(60),
        "document.getElementById('x').textContent += 'C'",
    );
    let html = format!(
        "<html><body><div id='x'></div>\
         <script src='{a}/a.js'></script>\
         <script src='{b}/b.js'></script>\
         <script src='{c}/c.js'></script></body></html>"
    );
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_body(&html)
        .expect_at_least(2)
        .create();
    assert_eq!(
        text_out(&server.url())["out"],
        text_out(&server.url())["out"]
    );
}
