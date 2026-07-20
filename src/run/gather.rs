//! The CSS-gather phase: collect external `<link rel=stylesheet>` sheets,
//! concurrently and under one aggregate wall-clock budget.
//!
//! Two bounds, one per real resource (`docs/design/css.md` §1a, mirroring
//! `docs/design/js.md` §6's "one bound per resource, none per request count"):
//!
//! - **Time — [`GATHER_BUDGET_MS`], for the whole phase.** Not per request:
//!   a per-request ceiling multiplied by an attacker/host-controlled sheet
//!   count is not a bound. The deadline is the single authority, and each
//!   request's timeout is *derived* from what the budget has left
//!   ([`fetch::fetch_within`]), so a host that never answers costs the phase
//!   its remaining budget and nothing more — there is no in-flight overshoot.
//! - **Concurrency — [`MAX_IN_FLIGHT`] simultaneous requests.** This bounds
//!   sockets and OS threads, which are real resources; it is deliberately
//!   *not* a cap on how many sheets are fetched. Every sheet is still
//!   attempted, in document order, until time runs out — the count of sheets
//!   prices no resource (the argument that retired `SUBFETCH_MAX`, js.md §6).
//!
//! Tripping the budget is not a run failure. CSS is best-effort here as it
//! always has been: whatever arrived applies, in source order, and the
//! cascade proceeds.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crate::dom::{Document, NodeKind, WalkEvent};
use crate::fetch;
use url::Url;

/// Aggregate wall-clock ceiling for the entire gather phase.
///
/// A backstop, not the mechanism: concurrency is what makes the common case
/// fast (the pathological real page, linear.app's 68 Next.js chunks, gathers
/// well inside this once six requests fly at once), and this bound exists so
/// that one slow or dead host cannot set the floor for the run. It is sized
/// from both ends — comfortably above a full wave of legitimately slow sheets
/// (a cold cross-origin CDN handshake plus transfer, ~1–2 s), and far below
/// the per-request `fetch::TIMEOUT_SECS` ceiling it replaces as the phase's
/// effective limit, so `--css` can no longer cost 15 s × N. A constant, not a
/// flag, on the same severability posture as the 1280 px viewport and
/// `EXEC_BUDGET_MS`: nothing about a caller's page makes a different number
/// right, and a flag here would be a tunable for a behaviour that should just
/// be correct.
const GATHER_BUDGET_MS: u64 = 5_000;

/// Simultaneous stylesheet requests. Six is what Firefox allows per server
/// (`network.http.max-persistent-connections-per-server`), and frot already
/// presents as Firefox — issuing 68 parallel connections would be both a
/// fingerprint tell and rude to the origin.
const MAX_IN_FLIGHT: usize = 6;

/// Fetch every external stylesheet the document links, best-effort: CSS is
/// non-critical, so a failed or unfinished fetch is skipped rather than
/// failing the run. Returns the bodies that arrived, in document order —
/// source order is cascade order, so it must survive concurrency.
///
/// The caller's `-H` headers ride along only when the sheet shares the page's
/// origin — credentials never leak cross-origin.
pub(crate) fn external_css(
    doc: &Document,
    base: &str,
    headers: &[(String, String)],
) -> Vec<String> {
    let budget = Duration::from_millis(GATHER_BUDGET_MS);
    gather_within(&external_hrefs(doc, base), base, headers, budget)
}

/// [`external_css`]'s gather with the phase budget supplied by the caller —
/// the seam the budget-trip tests dial down, so the bound is exercised
/// without waiting on wall-clock (js.md's `Session::with_budget` pattern).
pub(crate) fn gather_within(
    hrefs: &[String],
    base: &str,
    headers: &[(String, String)],
    budget: Duration,
) -> Vec<String> {
    let deadline = Instant::now() + budget;
    // The only shared state is the work cursor; each worker owns its results
    // and hands them back at join, so nothing needs a lock.
    let cursor = AtomicUsize::new(0);
    let mut got: Vec<(usize, String)> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..hrefs.len().min(MAX_IN_FLIGHT))
            .map(|_| scope.spawn(|| drain(hrefs, base, headers, deadline, &cursor)))
            .collect();
        workers.into_iter().flat_map(|w| w.join().unwrap()).collect()
    });
    got.sort_by_key(|(i, _)| *i);
    got.into_iter().map(|(_, body)| body).collect()
}

/// One worker: take the next unclaimed href and fetch it, until the list is
/// exhausted or the phase deadline passes. Each fetch is given exactly the
/// budget's remainder, so the last request cannot outlive the phase.
fn drain(
    hrefs: &[String],
    base: &str,
    headers: &[(String, String)],
    deadline: Instant,
    cursor: &AtomicUsize,
) -> Vec<(usize, String)> {
    let mut got = Vec::new();
    loop {
        let i = cursor.fetch_add(1, Ordering::Relaxed);
        let (Some(href), Some(left)) =
            (hrefs.get(i), deadline.checked_duration_since(Instant::now()))
        else {
            return got;
        };
        let scoped: &[(String, String)] =
            if fetch::same_origin(href, base) { headers } else { &[] };
        if let Ok(r) = fetch::fetch_within(href, scoped, left) {
            got.push((i, r.body));
        }
    }
}

/// Absolute URLs of `<link rel="stylesheet">` hrefs, resolved against the
/// page's final URL. Non-stylesheet links, empty hrefs, and hrefs that fail
/// to resolve are dropped. A `file:` sheet is kept only when the page itself
/// is `file:` — remote content must never cause local reads.
pub(crate) fn external_hrefs(doc: &Document, base: &str) -> Vec<String> {
    let base_url = Url::parse(base).ok();
    let base_is_file = base_url.as_ref().is_some_and(|b| b.scheme() == "file");
    let mut out = Vec::new();
    doc.walk(None, &mut |ev, e| {
        if let WalkEvent::Enter(_) = ev {
            if let NodeKind::Element(el) = &e.kind {
                let is_sheet = el.name == "link"
                    && el.attr("rel").is_some_and(|r| {
                        r.split_whitespace().any(|t| t.eq_ignore_ascii_case("stylesheet"))
                    });
                if is_sheet {
                    if let Some(h) = el.attr("href").filter(|s| !s.is_empty()) {
                        if let Some(u) = base_url.as_ref().and_then(|b| b.join(h).ok()) {
                            if u.scheme() != "file" || base_is_file {
                                out.push(u.to_string());
                            }
                        }
                    }
                }
            }
        }
    });
    out
}

#[cfg(test)]
mod tests;
