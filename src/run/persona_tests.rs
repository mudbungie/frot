//! Deterministic persona-contract half of the A/B instrument (`bl-46f5`).
//!
//! The live corpus (`examples/ab_harness.rs`) is dated network evidence; this is
//! the offline, `make cov`-gated oracle (identity.md §12): a local HTTP/1.1
//! origin records the exact request head frot sends and drives the *production*
//! `run_io` pipeline, so a regression in the wire persona fails the build with no
//! network.
//!
//! Two recorded contracts live here:
//! - **`persona-http-head.txt`** — the h1 request head over plain `http://`
//!   (§3.3 header set/order/values). Since `bl-abca` moved the transport to
//!   hyper, the head is **Title-Cased on both schemes** (hyper's
//!   `http1_title_case_headers`), dissolving §3.4's old scheme-dependent
//!   lowercase `http://` tell. This is the CURRENT contract, not a frozen one:
//!   `bl-20ec` (request metadata) changes the header values/order and updates
//!   this one golden file. Each is a single-file edit.
//! - **the declared-challenge negative control** (§3.7): a `Retry-After` 200 is
//!   detected pre-parse, so frot makes EXACTLY ONE request, emits `needs:["human"]`
//!   with no `out`/`js` block, and exits 0. Proven offline by counting the
//!   origin's accepted connections — the count the live harness cannot see.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};

use serde_json::Value;

/// A local HTTP/1.1 origin. It replies with `response` to every connection and
/// sends the captured request head down the channel — one message per accepted
/// connection, so the receiver both reads the head and counts the requests.
fn origin(response: &'static [u8]) -> (u16, Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        // A GET carries no body, and on loopback the head lands in one segment,
        // so a single read captures it (the firefox_tls tests read the same way).
        for mut sock in listener.incoming().flatten() {
            let mut buf = [0u8; 4096];
            let n = sock.read(&mut buf).unwrap();
            let text = String::from_utf8_lossy(&buf[..n]).to_string();
            let head = text.split("\r\n\r\n").next().unwrap().to_string();
            sock.write_all(response).unwrap();
            let _ = sock.flush();
            tx.send(head).unwrap();
        }
    });
    (port, rx)
}

/// Drive the real pipeline against `url`, returning `(exit_code, envelope)`.
fn run(url: &str, view: &str) -> (u8, Value) {
    let argv = vec![url.to_string(), "--out".into(), view.into()];
    let mut out = Vec::new();
    let mut err = Vec::new();
    let exit = super::run_io(&argv, &mut out, &mut err);
    let env = serde_json::from_slice(&out).unwrap();
    (exit, env)
}

const OK_HTML: &[u8] =
    b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: 17\r\n\r\n<p>persona ok</p>";

/// The persona head is byte-stable except the loopback authority; normalize it.
fn normalize(head: &str, port: u16) -> String {
    head.replace(&format!("127.0.0.1:{port}"), "HOST")
}

#[test]
fn h1_request_head_matches_the_recorded_persona_contract() {
    let (port, rx) = origin(OK_HTML);
    let (exit, env) = run(&format!("http://127.0.0.1:{port}/"), "text");
    // The documented exit contract: an `ok` envelope exits 0.
    assert_eq!(exit, 0);
    assert_eq!(env["status"], "ok");
    let head = rx.recv().unwrap();
    let golden = include_str!("testdata/persona-http-head.txt");
    assert_eq!(normalize(&head, port), golden.trim_end());
}

const CHALLENGE_HTML: &[u8] =
    b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nRetry-After: 0\r\nContent-Length: 17\r\n\r\n<p>challenged!!</p>";

#[test]
fn declared_challenge_makes_exactly_one_request_never_executed() {
    let (port, rx) = origin(CHALLENGE_HTML);
    // `--js` on purpose: even asked to execute, a declared challenge stops before
    // parse/JS, so scripts never run (§3.7 "never executed or retried").
    let argv = vec![
        format!("http://127.0.0.1:{port}/"),
        "--js".into(),
        "--out".into(),
        "dom".into(),
    ];
    let mut out = Vec::new();
    let mut err = Vec::new();
    let exit = super::run_io(&argv, &mut out, &mut err);
    let env: Value = serde_json::from_slice(&out).unwrap();
    // `needs:["human"]`, exit 0, and nothing executed or delivered.
    assert_eq!(exit, 0);
    assert_eq!(env["status"], "needs");
    assert_eq!(env["needs"], serde_json::json!(["human"]));
    assert!(
        env.get("out").is_none(),
        "a challenge delivers no impression"
    );
    assert!(env.get("js").is_none(), "a challenge is never executed");
    // Exactly one request: one head arrives, then the channel goes quiet. The
    // `Ok`-then-timeout drains both arms, and a retry would surface a second `Ok`.
    let mut requests = 0;
    while rx
        .recv_timeout(std::time::Duration::from_millis(300))
        .is_ok()
    {
        requests += 1;
    }
    assert_eq!(
        requests, 1,
        "a declared challenge is fetched once, never retried"
    );
}
