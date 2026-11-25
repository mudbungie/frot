//! The script queue (js.md §4): the parser's scripts in document order, then
//! every inserted external or module script, one task each, in the window it
//! was inserted into — a frame's or the top's (`bl-b34e`). Split out of
//! `js.rs` under the 300-line cap.

use super::script::{self, Script};
use super::syscall::Window;
use super::{eventloop, subfetch, EvalError, Report, Session};
use crate::dom::NodeId;
use crate::envelope::JsStop;

/// The parser's scripts, in document order (§4), then every inserted script
/// they queued. A script a script inserts is not one of the parser's: an inline
/// one already ran inside the call that inserted it (`syscall::insert`), and an
/// external or module one waits in the pending queue for [`drain_inserted`]. A
/// budget trip names the `stopped` bound and ends the phase.
pub(super) fn run_script_queue(session: &Session, report: &mut Report) {
    for id in session.parser_scripts() {
        if !report.settled() {
            return;
        }
        if let Some(script) = session.prepare_parsed(id) {
            execute(session, id, script, None, report);
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
        let Some((id, script, window)) = session.next_inserted() else {
            return;
        };
        execute(session, id, script, window.as_ref(), report);
    }
}

/// Run one prepared script in its window (a frame's, or the top's): an
/// external `src` is fetched under §6 (§4.1), an inline module's imports
/// resolve against the document base URL — a frame's `about:blank` has its
/// creator's.
fn execute(
    session: &Session,
    id: NodeId,
    script: Script,
    win: Option<&Window>,
    report: &mut Report,
) {
    let realm = win.map(|w| w.realm);
    match script {
        Script::External { module, src } => run_external(session, id, module, &src, win, report),
        Script::Inline {
            module: false,
            body,
        } => run_script(session, win, id, &body, report),
        Script::Inline { module: true, body } => {
            run_module(session, realm, &script::module_base(session), &body, report)
        }
    }
}

/// Fetch an external `src` under §6 and run its body, then fire `load` at the
/// element — or, when the fetch failed or was non-2xx, count it (§4.2, like a
/// failed stylesheet) and fire `error`, as Firefox does for every external
/// script. A 2xx (or `file://`) body runs as a classic script, or — for
/// `type="module"` — as a module named by its own fetched URL, so its imports
/// resolve against it (§4.1).
fn run_external(
    session: &Session,
    id: NodeId,
    module: bool,
    src: &str,
    win: Option<&Window>,
    report: &mut Report,
) {
    let event = match session.subfetch(src, script::external_intent(module)) {
        subfetch::Outcome::Got(f) if f.ok && module => {
            run_module(session, win.map(|w| w.realm), &f.url, &f.body, report);
            "load"
        }
        subfetch::Outcome::Got(f) if f.ok => {
            run_script(session, win, id, &f.body, report);
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
        eventloop::script_event(session, win.map(|w| w.realm), id, event, report);
    }
}

/// Evaluate one classic script body under the run's bounds, tallying it (§5).
/// `id` is the `<script>` element it came from — inline or external, it is what
/// `document.currentScript` reports for the length of the evaluation (§4.1).
fn run_script(
    session: &Session,
    win: Option<&Window>,
    id: NodeId,
    body: &str,
    report: &mut Report,
) {
    tally(session, report, session.run_script(win, id, body));
}

/// Evaluate one ES module (`name` = its URL, the import base, js.md §4.1) under
/// the run's bounds, tallying it (§5). An unresolvable specifier or failed
/// module fetch throws and is counted here; sibling scripts continue.
fn run_module(
    session: &Session,
    realm: Option<usize>,
    name: &str,
    body: &str,
    report: &mut Report,
) {
    tally(session, report, session.run_module(realm, name, body));
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
