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
mod eventloop;
mod geometry;
mod loader;
mod prelude;
mod probe;
mod script;
mod session;
mod subfetch;
mod syscall;

use crate::dom::{Document, NodeId};
use crate::envelope::JsStop;
use crate::fetch::FetchSession;
use engine::{Deadline, NetBudget};
use script::Script;

pub use engine::EvalError;
pub use geometry::StyleSource;
pub use probe::{measure, Measurement};
pub use session::Session;
pub(crate) use subfetch::ScriptBody;
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
    /// Which §5 bound ended the run (§10 `stopped`), or `None` when it reached
    /// quiescence within both — script queue *and* settle loop. The run's one
    /// termination fact: [`settled`](Report::settled) is exactly its absence, so
    /// there is nothing to keep in step.
    pub stopped: Option<JsStop>,
    /// Whether the page's script asked to reload its own document — a refused
    /// navigation `run.rs` may honour as a re-issue (`challenge.md` §12).
    pub reload: bool,
    /// The bounded §10 error-message detail behind `errors` (js.md §10): the
    /// first `MESSAGES_MAX` captured `throw` / `report` / `subfetch` messages, in
    /// occurrence order. Captured unconditionally; the pipeline surfaces it only
    /// behind `--js-errors`. Rejections, refused navigations, and timer/
    /// lifecycle-listener throws stay count-only.
    pub messages: Vec<Message>,
}

impl Report {
    /// Whether the run reached §5 quiescence *within* its bounds — i.e. no bound
    /// ended it (§10). Derived, never stored: one fact, one home.
    pub fn settled(&self) -> bool {
        self.stopped.is_none()
    }
}

/// The two §5 bounds one run is spent against, carried as one value because
/// they are one policy: [`EXEC_CPU_MS`](engine::EXEC_CPU_MS) of *CPU* time for
/// frot's own compute, and [`NET_BUDGET_MS`](engine::NET_BUDGET_MS) of *wire*
/// time for §6 network.
///
/// It is a parameter rather than a constant read inside, so a caller that
/// cannot afford a host clock can supply one it controls: the golden fixture
/// suite serves from an in-process mock and takes the network window on a
/// frozen [`Clock::manual`](engine::Clock::manual) (bl-c81a). Explicit through
/// the call signature is the opposite of hidden test-only state — the same
/// posture `run::deliver` already takes with the SIGPIPE disposition.
///
/// One `Bounds` is one **invocation's** allowance. A clone shares both meters,
/// and [`run`] banks what a finished run left, so an invocation that runs two
/// documents — a declared challenge, then the page behind it (`challenge.md`
/// §7) — spends one budget across both, never two.
#[derive(Clone)]
pub struct Bounds {
    /// Compute, spent on a CPU clock.
    pub cpu: Deadline,
    /// Network, metered on the clock the page also observes as `Date.now` and
    /// spendable only by real dispatch (`bl-79dc`).
    pub net: NetBudget,
}

impl Bounds {
    /// What every shipping invocation runs under: the two host clocks.
    pub fn shipping() -> Self {
        Bounds {
            cpu: Deadline::compute(),
            net: NetBudget::network(),
        }
    }
}

/// Run a page's JS against `doc` under the bounded virtual-clock event loop
/// (`docs/design/js.md` §4–§5), returning the post-JS document and the [`Report`].
/// `styles`/`env` seed the geometry cache (§8) and environment shims (§7).
///
/// `bounds` spans the whole run — the script queue *and* the settle loop — the
/// compute window armed once ([`Session::begin`]) and the network budget spent
/// only by real §6 dispatch. The phases run in order (§4.4): the script
/// queue drains, then `DOMContentLoaded`, then `load`, then the virtual-clock
/// settle loop. A budget trip anywhere stops the run and marks it unsettled
/// (§5); a script/callback throw is counted and the run continues.
pub fn run(
    doc: Document,
    styles: StyleSource,
    env: Env,
    fetch: &FetchSession,
    bounds: Bounds,
) -> (Document, Report) {
    let cpu = bounds.cpu.clone();
    let session = Session::with_bounds(doc, styles, env, fetch, bounds.cpu, bounds.net);
    session.begin();
    let report = run_session(&session);
    cpu.bank();
    (session.into_document(), report)
}

/// Drive an already-armed [`Session`] through the whole run (§4–§5), folding the
/// deferred §10 tallies into one [`Report`]. Shared by the shipping [`run`] path
/// and the [`probe::measure`] instrument (distinct from the one-step [`drive`]).
pub(crate) fn run_session(session: &Session) -> Report {
    let mut report = Report {
        scripts: 0,
        errors: 0,
        stopped: None,
        reload: false,
        messages: Vec::new(),
    };
    // Preload-scan (bl-08f6): warm the §6 cache concurrently with the initial
    // external scripts discovered after parse, under the run's armed network
    // deadline and byte pool, so the source-ordered queue below finds each
    // external `src` already frozen instead of blocking on it serially. It only
    // warms statically present scripts — a script a script inserts is fetched
    // when discovered, never speculatively.
    session.warm_initial_scripts();
    run_script_queue(session, &mut report);
    if report.settled() {
        eventloop::run(session, &mut report);
    }
    // Quiescence within bounds is `settled`'s one meaning (§5): the loop can
    // conclude *because* the §6 seam refused dispatch once the run had spent its
    // whole network budget on the wire, while the engine interrupt — firing only
    // between JS instructions, and spending CPU a blocked socket never burns —
    // never tripped. That refusal is a recorded fact, so the driver asks for it
    // rather than measuring at conclusion; and since only real dispatch can spend
    // that budget, `Network` here always names network (bl-79dc).
    report.stopped = report
        .stopped
        .or_else(|| session.refused_network().then_some(JsStop::Network));
    // Fold the deferred §10 tallies into `errors`: refused navigations (counted
    // no-ops), unhandled promise rejections (net after the run's microtasks drain
    // — a late `.catch` un-counts), and unhandled errors the shim reported.
    report.errors += session.denials();
    report.reload = session.reload_asked();
    report.errors += session.rejections();
    report.errors += session.reported_errors();
    let (ran, threw) = session.inserted_tally();
    report.scripts += ran;
    report.errors += threw;
    // The bounded §10 message detail captured across the run (§10).
    report.messages = session.messages();
    report
}

/// The parser's scripts, in document order (§4), then every inserted script
/// they queued. A script a script inserts is not one of the parser's: an inline
/// one already ran inside the call that inserted it (`syscall::insert`), and an
/// external or module one waits in the pending queue for [`drain_inserted`]. A
/// budget trip names the `stopped` bound and ends the phase.
fn run_script_queue(session: &Session, report: &mut Report) {
    for id in session.parser_scripts() {
        if !report.settled() {
            return;
        }
        if let Some(script) = session.prepare_parsed(id) {
            execute(session, id, script, report);
        }
    }
    drain_inserted(session, report);
}

/// Run the inserted external and module scripts waiting for a task, oldest
/// first, one task each (§4, `bl-a86b`) — after the parser's scripts and after
/// every event-loop task, so a script a timer inserts runs before the next
/// timer, as a fetch that completes at once would in a browser.
pub(crate) fn drain_inserted(session: &Session, report: &mut Report) {
    while report.settled() {
        let Some((id, script)) = session.next_inserted() else {
            return;
        };
        execute(session, id, script, report);
    }
}

/// Run one prepared script: an external `src` is fetched under §6 (§4.1), an
/// inline module's imports resolve against the document base URL.
fn execute(session: &Session, id: NodeId, script: Script, report: &mut Report) {
    match script {
        Script::External { module, src } => run_external(session, id, module, &src, report),
        Script::Inline {
            module: false,
            body,
        } => run_script(session, id, &body, report),
        Script::Inline { module: true, body } => {
            run_module(session, &script::module_base(session), &body, report)
        }
    }
}

/// Fetch an external `src` under §6 and run its body, then fire `load` at the
/// element — or, when the fetch failed or was non-2xx, count it (§4.2, like a
/// failed stylesheet) and fire `error`, as Firefox does for every external
/// script. A 2xx (or `file://`) body runs as a classic script, or — for
/// `type="module"` — as a module named by its own fetched URL, so its imports
/// resolve against it (§4.1).
fn run_external(session: &Session, id: NodeId, module: bool, src: &str, report: &mut Report) {
    let event = match session.subfetch(src, script::external_intent(module)) {
        subfetch::Outcome::Got(f) if f.ok && module => {
            run_module(session, &f.url, &f.body, report);
            "load"
        }
        subfetch::Outcome::Got(f) if f.ok => {
            run_script(session, id, &f.body, report);
            "load"
        }
        // Captured by its spec, so `--js-errors` names which bundle went dark.
        _ => {
            report.errors += 1;
            session.capture("subfetch", src);
            "error"
        }
    };
    if report.settled() {
        eventloop::script_event(session, id, event, report);
    }
}

/// Evaluate one classic script body under the run's bounds, tallying it (§5).
/// `id` is the `<script>` element it came from — inline or external, it is what
/// `document.currentScript` reports for the length of the evaluation (§4.1).
fn run_script(session: &Session, id: NodeId, body: &str, report: &mut Report) {
    tally(session, report, session.run_script(id, body));
}

/// Evaluate one ES module (`name` = its URL, the import base, js.md §4.1) under
/// the run's bounds, tallying it (§5). An unresolvable specifier or failed
/// module fetch throws and is counted here; sibling scripts continue.
fn run_module(session: &Session, name: &str, body: &str, report: &mut Report) {
    tally(session, report, session.run_module(name, body));
}

/// Fold one script/module evaluation into the report per §5: it ran (`scripts`);
/// a throw counts an error and its message is captured (§10 `throw`); a budget
/// trip additionally names `stopped: "budget"` and carries no message (the run
/// stopped, not the script).
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
            report.stopped = Some(JsStop::Budget);
        }
    }
}

#[cfg(test)]
mod tests;
