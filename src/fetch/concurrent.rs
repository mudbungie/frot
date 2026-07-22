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
    let cursor = AtomicUsize::new(0);
    let spent = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..reqs.len().min(POOL_PER_HOST))
            .map(|_| {
                scope.spawn(|| {
                    drain(
                        session,
                        reqs,
                        initiator,
                        deadline,
                        byte_budget,
                        &cursor,
                        &spent,
                    )
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().unwrap())
            .collect()
    })
}

/// One worker: claim the next unclaimed request and fetch it through the shared
/// `session`, until the list is exhausted, the byte budget is reached, or the
/// deadline passes. Each fetch gets exactly the budget's remainder, so no
/// in-flight request can outlive the phase; the session owns dedup and `-H`
/// scoping. Once `spent` crosses `byte_budget` (or the deadline lapses) the
/// worker stops — a bounded overshoot of at most the in-flight wave.
#[allow(clippy::too_many_arguments)]
fn drain(
    session: &FetchSession,
    reqs: &[(String, Intent)],
    initiator: &str,
    deadline: Instant,
    byte_budget: usize,
    cursor: &AtomicUsize,
    spent: &AtomicUsize,
) -> Vec<(usize, FetchResult)> {
    let mut got = Vec::new();
    loop {
        if spent.load(Ordering::Relaxed) >= byte_budget {
            return got;
        }
        let i = cursor.fetch_add(1, Ordering::Relaxed);
        let (Some((url, intent)), Some(left)) =
            (reqs.get(i), deadline.checked_duration_since(Instant::now()))
        else {
            return got;
        };
        if let Ok(r) = session.subresource(url, initiator, *intent, left) {
            spent.fetch_add(r.body.len(), Ordering::Relaxed);
            got.push((i, r));
        }
    }
}

#[cfg(test)]
mod tests;
