//! [`fetch_many`] tests: the barrier-controlled overlap proof (real concurrency,
//! not a sleep threshold), the byte-pool bound on a wave, and the deterministic
//! teardown that leaves no worker behind past the deadline (identity.md §6.1
//! "no work escapes the call"). Every origin is a loopback stub, never the real
//! network.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

use super::*;

fn reqs(base: &str, n: usize) -> Vec<(String, Intent)> {
    (0..n)
        .map(|i| (format!("{base}/r{i}"), Intent::ClassicScript))
        .collect()
}

fn plain() -> FetchSession {
    FetchSession::new(Vec::new())
}

/// An origin whose every connection blocks on a shared [`Barrier`] before it
/// answers: the barrier can only trip when `parties` requests are in flight at
/// the *same time*, so a body coming back from all of them is proof of genuine
/// overlap — a serial fetch would leave the first request wedged at the barrier
/// forever. No timing threshold is involved.
fn barrier_origin(parties: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let barrier = Arc::new(Barrier::new(parties));
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let b = barrier.clone();
            std::thread::spawn(move || {
                let mut s = stream;
                let _ = s.read(&mut [0u8; 1024]);
                b.wait();
                let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok");
            });
        }
    });
    base
}

/// A keep-alive origin returning a fixed body immediately, counting accepted
/// connections — for the byte-budget wave.
fn fast_origin() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || {
                let mut s: TcpStream = stream;
                let _ = s.read(&mut [0u8; 1024]);
                let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok");
            });
        }
    });
    base
}

/// A dead origin: it accepts the connection but never answers, so a request can
/// only end at its own deadline — the case that proves no worker outlives the
/// budget.
fn dead_origin() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || {
                let mut s: TcpStream = stream;
                let _ = s.read(&mut [0u8; 1024]);
                std::thread::sleep(Duration::from_secs(30));
            });
        }
    });
    base
}

fn far() -> Instant {
    Instant::now() + Duration::from_secs(30)
}

#[test]
fn a_full_wave_overlaps_proven_by_a_barrier_not_a_clock() {
    // POOL_PER_HOST requests each wedge on a barrier that needs all of them
    // present at once. They come back only because the wave is genuinely
    // concurrent — a serial dispatch would deadlock at the first.
    let base = barrier_origin(POOL_PER_HOST);
    let got = fetch_many(
        &plain(),
        &reqs(&base, POOL_PER_HOST),
        &base,
        far(),
        usize::MAX,
    );
    assert_eq!(got.len(), POOL_PER_HOST);
    assert!(got.iter().all(|(_, r)| r.body == "ok"));
}

#[test]
fn the_byte_budget_caps_a_wave_but_a_large_one_does_not() {
    let base = fast_origin();
    let many = reqs(&base, POOL_PER_HOST * 2);
    // A one-byte budget curtails dispatch after the first wave commits (each
    // body is two bytes): at most POOL_PER_HOST land, never all twelve.
    let capped = fetch_many(&plain(), &many, &base, far(), 1);
    assert!(capped.len() <= POOL_PER_HOST, "got {}", capped.len());
    // With the budget lifted the same wave fetches every request — the count is
    // never itself a bound.
    let full = fetch_many(&plain(), &many, &base, far(), usize::MAX);
    assert_eq!(full.len(), POOL_PER_HOST * 2);
}

#[test]
fn no_worker_outlives_the_deadline_against_a_dead_host() {
    let base = dead_origin();
    // More requests than workers, a host that never answers, and a short
    // deadline: `thread::scope` joins every worker at the deadline, so the call
    // returns promptly with nothing — no request escapes into the background.
    let started = Instant::now();
    let deadline = started + Duration::from_millis(300);
    let got = fetch_many(
        &plain(),
        &reqs(&base, POOL_PER_HOST + 2),
        &base,
        deadline,
        usize::MAX,
    );
    assert!(got.is_empty());
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "leaked: {:?}",
        started.elapsed()
    );
}

#[test]
fn an_empty_request_list_spawns_no_work() {
    assert!(fetch_many(&plain(), &[], "http://example.com/", far(), usize::MAX).is_empty());
}
