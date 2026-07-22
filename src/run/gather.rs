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
//! - **Concurrency — `POOL_PER_HOST` simultaneous requests** (bl-08f6's shared
//!   [`crate::fetch::fetch_many`]). This bounds sockets and OS threads, which
//!   are real resources; it is deliberately *not* a cap on how many sheets are
//!   fetched. Every sheet is still attempted, in document order, until time runs
//!   out — the count of sheets prices no resource (the argument that retired
//!   `SUBFETCH_MAX`, js.md §6).
//!
//! Tripping the budget is not a run failure. CSS is best-effort here as it
//! always has been: whatever arrived applies, in source order, and the
//! cascade proceeds.

use std::time::{Duration, Instant};

use crate::dom::{Document, NodeKind, WalkEvent};
use crate::fetch::{fetch_many, FetchSession, Intent};
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

/// Fetch every external stylesheet the document links, best-effort: CSS is
/// non-critical, so a failed or unfinished fetch is skipped rather than
/// failing the run. Returns the bodies that arrived, in document order —
/// source order is cascade order, so it must survive concurrency.
///
/// Every sheet rides `session`: the workers share its one connection pool (one
/// bounded pool — not one isolated agent per sheet), and an unchanged sheet
/// already gathered this invocation is re-served from the session cache without
/// a second fetch. The `-H` scoping and credentials rule live in the session.
pub(crate) fn external_css(doc: &Document, base: &str, session: &FetchSession) -> Vec<String> {
    let budget = Duration::from_millis(GATHER_BUDGET_MS);
    gather_within(&external_hrefs(doc, base), base, session, budget)
}

/// [`external_css`]'s gather with the phase budget supplied by the caller —
/// the seam the budget-trip tests dial down, so the bound is exercised
/// without waiting on wall-clock (js.md's `Session::with_budget` pattern).
///
/// The bounded concurrent fetch is the shared [`fetch_many`] primitive (bl-08f6);
/// the cascade is source-ordered, so completion order is discarded and the
/// bodies are restored to href (document) order. CSS is time-bounded only — no
/// byte cap (`usize::MAX`): every sheet is attempted until the budget lapses.
pub(crate) fn gather_within(
    hrefs: &[String],
    base: &str,
    session: &FetchSession,
    budget: Duration,
) -> Vec<String> {
    let reqs: Vec<(String, Intent)> = hrefs.iter().map(|h| (h.clone(), Intent::Style)).collect();
    let mut got = fetch_many(session, &reqs, base, Instant::now() + budget, usize::MAX);
    got.sort_by_key(|(i, _)| *i);
    got.into_iter().map(|(_, r)| r.body).collect()
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
                        r.split_whitespace()
                            .any(|t| t.eq_ignore_ascii_case("stylesheet"))
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
