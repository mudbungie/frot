//! The one bounded concurrent subresource fetch (`bl-08f6`) — the preload-scanner
//! primitive both the CSS gather (`run/gather.rs`) and the JS initial-script warm
//! (`js/subfetch.rs`) ride, so browser-like reuse and concurrency has a single
//! home instead of a copy per phase.
//!
//! This is the "subfetch concurrency (parallel dispatch under the same deadline
//! and byte pool)" js.md §6 named as the principled lever and deliberately left
//! unbuilt until now. Two bounds, one per real resource (js.md §6 / css.md §1a):
//! at most [`POOL_PER_HOST`] requests in flight (sockets/threads — the Firefox
//! per-server limit), and at most `byte_budget` response bytes across the wave
//! (memory). Never a request-count cap — a count prices no resource.
//!
//! `std::thread::scope` **joins every worker before returning**: no fetch escapes
//! this call, so the invocation's async ruling holds (identity.md §6.1 item 2 —
//! "no work escapes the call"). Concurrency lives entirely inside the blocking
//! `FetchSession` surface, over its one shared pool (h2 multiplexes the wave onto
//! one connection; h1 opens at most [`POOL_PER_HOST`]).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use super::{FetchResult, FetchSession, Intent, POOL_PER_HOST};

/// Fetch `reqs` (each an **absolute** URL + its request intent) through
/// `session`'s shared pool, at most [`POOL_PER_HOST`] in flight, every request
/// timed out at the `deadline` remainder and the whole wave capped at
/// `byte_budget` response bytes. Returns each success paired with its source
/// index — completion order is irrelevant, the caller restores document/cascade
/// order — while failures and past-budget/-deadline requests are dropped,
/// best-effort like the stylesheet gather. `initiator` is the page URL every
/// subresource references (its origin scopes the `-H` credentials).
pub(crate) fn fetch_many(
    session: &FetchSession,
    reqs: &[(String, Intent)],
    initiator: &str,
    deadline: Instant,
    byte_budget: usize,
) -> Vec<(usize, FetchResult)> {
    // The one real seam: each index becomes a live subresource fetch, timed out
    // at the deadline remainder (`None` past the deadline, so the worker treats a
    // lapsed request as a non-result and moves on). Everything else — the wave
    // bound, the byte pool, the join — is the network-agnostic scheduler.
    fetch_wave(reqs.len(), byte_budget, |i| {
        let (url, intent) = &reqs[i];
        let left = deadline.checked_duration_since(Instant::now())?;
        session.subresource(url, initiator, *intent, left).ok()
    })
}

/// The bounded concurrent scheduler with the network lifted out: run `fetch` over
/// `0..n` across at most [`POOL_PER_HOST`] workers, capping the wave at
/// `byte_budget` response bytes and returning each `Some` result paired with its
/// index. `fetch(i)` is the sole I/O — a live subresource in production, a fixed
/// body in a test — so the byte-budget cap is provable without a real origin's
/// timing. Injecting the fetch is the seam that makes the pool deterministic.
fn fetch_wave(
    n: usize,
    byte_budget: usize,
    fetch: impl Fn(usize) -> Option<FetchResult> + Sync,
) -> Vec<(usize, FetchResult)> {
    let cursor = AtomicUsize::new(0);
    let spent = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..n.min(POOL_PER_HOST))
            .map(|_| scope.spawn(|| drain(n, byte_budget, &fetch, &cursor, &spent)))
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().unwrap())
            .collect()
    })
}

/// One worker: claim the next unclaimed index and run `fetch` on it, until the
/// list is exhausted or the byte budget is reached. A `None` result (a failed or
/// past-deadline fetch) is dropped and the worker moves on; a `Some` spends its
/// body length against the shared pool. Once `spent` crosses `byte_budget` the
/// worker stops — a bounded overshoot of at most the in-flight wave.
fn drain(
    n: usize,
    byte_budget: usize,
    fetch: &(impl Fn(usize) -> Option<FetchResult> + Sync),
    cursor: &AtomicUsize,
    spent: &AtomicUsize,
) -> Vec<(usize, FetchResult)> {
    let mut got = Vec::new();
    while spent.load(Ordering::Relaxed) < byte_budget {
        let i = cursor.fetch_add(1, Ordering::Relaxed);
        if i >= n {
            break;
        }
        if let Some(r) = fetch(i) {
            spent.fetch_add(r.body.len(), Ordering::Relaxed);
            got.push((i, r));
        }
    }
    got
}

#[cfg(test)]
mod tests;
