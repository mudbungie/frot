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
mod prelude;
mod session;
mod subfetch;
mod syscall;

use std::time::Duration;

use crate::dom::{Document, NodeId, NodeKind};
use engine::EXEC_BUDGET_MS;

pub use engine::EvalError;
pub use geometry::StyleSource;
pub use session::Session;
pub use syscall::{Env, Log};

/// The `--js` execution outcome (`docs/design/js.md` §10), which the pipeline
/// maps into the envelope `js` block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    /// Scripts that executed (§10 `scripts`); a script that threw still ran.
    pub scripts: u32,
    /// Counted failures (§10 `errors`): throws — from scripts *and* from event-
    /// loop timer callbacks / lifecycle listeners (§5) — unhandled promise
    /// rejections (§10, including refused/failed `fetch`), refused navigations
    /// (the §7/§11 counted no-ops the environment shim tallies), plus every
    /// external `src` script whose §6 subfetch failed or was non-2xx, which §4.2
    /// skips-and-counts like a failed stylesheet.
    pub errors: u32,
    /// Whether the whole run — script queue *and* settle loop — reached
    /// quiescence within the single wall-clock budget (§5); a budget trip
    /// anywhere clears it.
    pub settled: bool,
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
pub fn run(doc: Document, styles: StyleSource, env: Env) -> (Document, Report) {
    run_with(doc, styles, env, Duration::from_millis(EXEC_BUDGET_MS))
}

/// [`run`] with an explicit wall-clock budget; the tests dial it down so the
/// budget-trip paths stay deterministic and fast (the 4.1 spike's pattern).
fn run_with(doc: Document, styles: StyleSource, env: Env, budget: Duration) -> (Document, Report) {
    let session = Session::with_budget(doc, styles, env, budget);
    session.begin();
    let mut report = Report {
        scripts: 0,
        errors: 0,
        settled: true,
    };
    run_script_queue(&session, &mut report);
    if report.settled {
        run_event_loop(&session, &mut report);
    }
    // Fold the deferred §10 tallies into `errors`: refused navigations (the
    // environment shim's counted no-ops) and unhandled promise rejections (net
    // once the whole run's microtasks have drained — a late `.catch` un-counts).
    report.errors += session.denials();
    report.errors += session.rejections();
    (session.into_document(), report)
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
            // External `src`: fetched under the §6 subfetch policy, then run.
            Script::External(src) => run_external(session, &src, report),
            // Non-JS `type` / `nomodule`: not for us (§4.1), skipped silently.
            Script::Skip => {}
            Script::Inline(body) => run_script(session, &body, report),
        }
    }
}

/// Fetch an external `src` under §6 and run its body. A failed or non-2xx fetch
/// leaves nothing to run, so the script is skipped-and-counted (§4.2) like a
/// failed stylesheet; a 2xx (or `file://`) body runs as a classic script.
fn run_external(session: &Session, src: &str, report: &mut Report) {
    match session.subfetch(src) {
        subfetch::Outcome::Got(f) if f.ok => run_script(session, &f.body, report),
        _ => report.errors += 1,
    }
}

/// Evaluate one script body under the run's deadline, tallying it per §5: a
/// throw counts an error, a budget trip additionally clears `settled`.
fn run_script(session: &Session, body: &str, report: &mut Report) {
    report.scripts += 1;
    match session.run_task(body) {
        Ok(_) => {}
        Err(EvalError::Exception(_)) => report.errors += 1,
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

/// A discovered `<script>`, classified for execution (§4.1).
enum Script {
    /// Inline classic/module body to evaluate.
    Inline(String),
    /// External `src` (the attribute value) — fetched under §6, then run (§4.2).
    External(String),
    /// Non-JS `type` or `nomodule`: intentionally not run, not counted.
    Skip,
}

/// The next `<script>` (document order) not already in `done`, classified.
fn next_script(session: &Session, done: &[NodeId]) -> Option<(NodeId, Script)> {
    let doc = session.document();
    let id = doc
        .find_by_tag("script")
        .into_iter()
        .find(|id| !done.contains(id))?;
    Some((id, classify(&doc, id)))
}

/// Classify a `<script>` per §4.1. `find_by_tag` only yields elements, but a
/// non-element id has nothing to run — a spec-legal-empty [`Script::Skip`],
/// not a panic.
fn classify(doc: &Document, id: NodeId) -> Script {
    let NodeKind::Element(el) = &doc.node(id).kind else {
        return Script::Skip;
    };
    if el.attr("nomodule").is_some() || !is_js_type(el.attr("type")) {
        Script::Skip
    } else if let Some(src) = el.attr("src").filter(|s| !s.is_empty()) {
        Script::External(src.to_string())
    } else {
        Script::Inline(doc.text_content(id))
    }
}

/// Whether a `<script type>` names JavaScript (or a module) and so runs (§4.1);
/// absent/empty is classic JS. Any other MIME is not for us.
fn is_js_type(t: Option<&str>) -> bool {
    match t {
        None => true,
        Some(s) => matches!(
            s.trim().to_ascii_lowercase().as_str(),
            "" | "module"
                | "text/javascript"
                | "application/javascript"
                | "text/ecmascript"
                | "application/ecmascript"
                | "text/jscript"
        ),
    }
}

#[cfg(test)]
mod tests;
