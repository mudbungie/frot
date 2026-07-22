//! `--js` capability (Phase 4).
//!
//! The one capability permitted to mutate the DOM: page scripts run in an
//! embedded engine against a facade over the *same* arena everything else
//! reads (single source of truth — no mirror DOM). See `docs/design/js.md`.
//!
//! Layering (js.md §3): [`engine`] is the swappable rquickjs seam; [`syscall`]
//! installs the narrow host-function table over a shared [`Document`];
//! [`prelude`] is the JS web-API built on those calls. No `rquickjs` type
//! escapes `src/js/`, mirroring how no `markup5ever` type leaks past `dom.rs`.

pub mod engine;
mod geometry;
mod loader;
mod prelude;
mod probe;
mod script;
mod session;
mod subfetch;
mod syscall;

use std::time::Duration;

use crate::dom::{Document, NodeId};
use crate::fetch::FetchSession;
use engine::EXEC_BUDGET_MS;
use script::{next_script, Script};

pub use engine::EvalError;
pub use geometry::StyleSource;
pub use probe::{measure, Measurement};
pub use session::Session;
pub use syscall::{Env, Log, Message};

/// The `--js` execution outcome (`docs/design/js.md` §10), which the pipeline
/// maps into the envelope `js` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Scripts that executed (§10 `scripts`); a script that threw still ran.
    pub scripts: u32,
    /// Counted failures (§10 `errors`): throws — from scripts *and* from event-
    /// loop timer callbacks / lifecycle listeners (§5) — unhandled promise
    /// rejections (§10, including refused/failed `fetch`), refused navigations
    /// (the §7/§11 counted no-ops the environment shim tallies), unhandled errors
    /// reported via `reportError`/`window.onerror`/a dispatched window `'error'`
    /// event (§10 — the caught-and-reported failures frameworks like React route
    /// here instead of throwing), plus every external `src` script whose §6
    /// subfetch failed or was non-2xx, which §4.2 skips-and-counts like a failed
    /// stylesheet.
    pub errors: u32,
    /// Whether the whole run — script queue *and* settle loop — reached
    /// quiescence within the single wall-clock budget (§5); a budget trip
    /// anywhere clears it.
    pub settled: bool,
    /// The bounded §10 error-message detail behind `errors` (js.md §10): the
    /// first `MESSAGES_MAX` captured `throw` / `report` / `subfetch` messages, in
    /// occurrence order. Captured unconditionally; the pipeline surfaces it only
    /// behind `--js-errors`. Rejections, refused navigations, and timer/
    /// lifecycle-listener throws stay count-only.
    pub messages: Vec<Message>,
}

/// Run a page's JS against `doc` under the bounded virtual-clock event loop
/// (`docs/design/js.md` §4–§5), returning the post-JS document and the [`Report`].
/// `styles`/`env` seed the geometry cache (§8) and environment shims (§7).
///
/// One wall-clock deadline ([`EXEC_BUDGET_MS`]) spans the whole run — the script
/// queue *and* the settle loop — armed once ([`Session::begin`]). The phases run
/// in order (§4.4): the script queue drains, then `DOMContentLoaded`, then
/// `load`, then the virtual-clock settle loop. A budget trip anywhere stops the
/// run and marks it unsettled (§5); a script/callback throw is counted and the
/// run continues.
pub fn run(
    doc: Document,
    styles: StyleSource,
    env: Env,
    fetch: &FetchSession,
) -> (Document, Report) {
    run_with(
        doc,
        styles,
        env,
        fetch,
        Duration::from_millis(EXEC_BUDGET_MS),
    )
}

/// [`run`] with an explicit wall-clock budget; the tests dial it down so the
/// budget-trip paths stay deterministic and fast (the 4.1 spike's pattern).
fn run_with(
    doc: Document,
    styles: StyleSource,
    env: Env,
    fetch: &FetchSession,
    budget: Duration,
) -> (Document, Report) {
    let session = Session::with_budget(doc, styles, env, fetch, budget);
    session.begin();
    let report = run_session(&session);
    (session.into_document(), report)
}

/// Drive an already-armed [`Session`] through the whole run (§4–§5), folding the
/// deferred §10 tallies into one [`Report`]. Shared by the shipping [`run`] path
/// and the [`probe::measure`] instrument (distinct from the one-step [`drive`]).
pub(crate) fn run_session(session: &Session) -> Report {
    let mut report = Report {
        scripts: 0,
        errors: 0,
        settled: true,
        messages: Vec::new(),
    };
    // Preload-scan (bl-08f6): warm the §6 cache concurrently with the initial
    // external scripts discovered after parse, under the run's *one* armed
    // deadline and byte pool, so the source-ordered queue below finds each
    // external `src` already frozen instead of blocking on it serially. It only
    // warms statically present scripts — a script a script inserts is fetched
    // when discovered, never speculatively.
    session.warm_initial_scripts();
    run_script_queue(session, &mut report);
    if report.settled {
        run_event_loop(session, &mut report);
    }
    // Quiescence within budget is `settled`'s one meaning (§5): the loop can
    // conclude *because* the deadline expired (the §6 seam refuses network past
    // it) while the engine interrupt — firing only between JS instructions —
    // never tripped, so an expired deadline here clears `settled` deterministically.
    report.settled = report.settled && !session.deadline_expired();
    // Fold the deferred §10 tallies into `errors`: refused navigations (counted
    // no-ops), unhandled promise rejections (net after the run's microtasks drain
    // — a late `.catch` un-counts), and unhandled errors the shim reported.
    report.errors += session.denials();
    report.errors += session.rejections();
    report.errors += session.reported_errors();
    // The bounded §10 message detail captured across the run (§10).
    report.messages = session.messages();
    report
}

/// Drain the script queue in document order (§4). Scripts a script inserts join
/// the queue (§4.3): each pass re-scans for the next not-yet-run `<script>`, a
/// fixpoint that picks up appended ones. A budget trip clears `settled` and ends
/// the phase; scripts inserted later (by timers/lifecycle) do not re-enter — the
/// script phase is over once this returns (§4.4).
fn run_script_queue(session: &Session, report: &mut Report) {
    let mut done: Vec<NodeId> = Vec::new();
    while report.settled {
        let Some((id, script)) = next_script(session, &done) else {
            break;
        };
        done.push(id);
        match script {
            // External `src`: fetched under the §6 subfetch policy, then run as a
            // classic script or evaluated as a module (§4.1).
            Script::External { module, src } => run_external(session, module, &src, report),
            // Non-JS `type` / `nomodule`: not for us (§4.1), skipped silently.
            Script::Skip => {}
            // Inline classic body, or an inline module whose imports resolve
            // against the page URL (js.md §4.1).
            Script::Inline {
                module: false,
                body,
            } => run_script(session, &body, report),
            Script::Inline { module: true, body } => {
                run_module(session, session.page_url(), &body, report)
            }
        }
    }
}

/// Fetch an external `src` under §6 and run its body. A failed or non-2xx fetch
/// leaves nothing to run, so the script is skipped-and-counted (§4.2) like a
/// failed stylesheet; a 2xx (or `file://`) body runs as a classic script, or —
/// for `type="module"` — as a module named by its own fetched URL, so its
/// imports resolve against it (§4.1).
fn run_external(session: &Session, module: bool, src: &str, report: &mut Report) {
    match session.subfetch(src, script::external_intent(module)) {
        subfetch::Outcome::Got(f) if f.ok && module => run_module(session, &f.url, &f.body, report),
        subfetch::Outcome::Got(f) if f.ok => run_script(session, &f.body, report),
        // A failed or non-2xx external `src` leaves nothing to run: counted (§4.2)
        // and captured by its spec, so `--js-errors` names which bundle went dark.
        _ => {
            report.errors += 1;
            session.capture("subfetch", src);
        }
    }
}

/// Evaluate one classic script body under the run's deadline, tallying it (§5).
fn run_script(session: &Session, body: &str, report: &mut Report) {
    tally(session, report, session.run_task(body));
}

/// Evaluate one ES module (`name` = its URL, the import base, js.md §4.1) under
/// the run's deadline, tallying it (§5). An unresolvable specifier or failed
/// module fetch throws and is counted here; sibling scripts continue.
fn run_module(session: &Session, name: &str, body: &str, report: &mut Report) {
    tally(session, report, session.run_module(name, body));
}

/// Fold one script/module evaluation into the report per §5: it ran (`scripts`);
/// a throw counts an error and its message is captured (§10 `throw`); a budget
/// trip additionally clears `settled` and carries no message (the run stopped,
/// not the script).
fn tally(session: &Session, report: &mut Report, result: Result<String, EvalError>) {
    report.scripts += 1;
    match result {
        Ok(_) => {}
        Err(EvalError::Exception(msg)) => {
            report.errors += 1;
            session.capture("throw", &msg);
        }
        Err(EvalError::Budget) => {
            report.errors += 1;
            report.settled = false;
        }
    }
}

/// The settle loop (§5): fire `DOMContentLoaded` then `load`, then drain the
/// virtual-clock timer queue until nothing is due before the horizon. Each step
/// is one host-driven macrotask (microtasks drain between, in the engine). A
/// budget trip stops the loop and leaves the run unsettled.
fn run_event_loop(session: &Session, report: &mut Report) {
    for name in ["DOMContentLoaded", "load"] {
        match drive(session, &format!("__frot_fire('{name}')"), report) {
            Some(errs) => report.errors += errs as u32,
            None => return,
        }
    }
    loop {
        match drive(session, "__frot_next_timer()", report) {
            // -1: nothing due before the horizon — the loop has settled (§5).
            Some(n) if n < 0 => break,
            // A fired callback: `n` is its error count (0 or 1); loop continues.
            Some(n) => report.errors += n as u32,
            // Budget tripped mid-callback: `settled` already cleared, stop.
            None => break,
        }
    }
}

/// Run one event-loop driver call, returning its integer protocol value, or
/// `None` when the budget tripped (which clears `settled`). The drivers catch
/// callback throws in JS and encode them in the return value, so the only
/// non-value outcome is a deadline hit.
fn drive(session: &Session, src: &str, report: &mut Report) -> Option<i64> {
    match session.run_task(src) {
        Ok(s) => Some(s.trim().parse::<i64>().unwrap_or(0)),
        Err(_) => {
            report.settled = false;
            None
        }
    }
}

#[cfg(test)]
mod tests;
