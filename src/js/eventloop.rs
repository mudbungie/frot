//! The §5 settle loop, after the script queue drains: the host-driven
//! macrotasks, one `Session::run_task` each.

use super::{Report, Session};
use crate::envelope::JsStop;

/// The settle loop (§5): the end of parsing (`interactive`) and the timers
/// already due by then (`bl-9f4c`), `DOMContentLoaded`, the image tasks `load`
/// waits for (`prelude/imageload.js`), `load`, then the virtual-clock timers
/// until none is due before the horizon. Each step is one host-driven macrotask
/// (microtasks drain between, in the engine); a budget trip stops the loop,
/// unsettled.
pub(super) fn run(session: &Session, report: &mut Report) {
    let _ = fire(session, "interactive", report)
        && drain(session, "__frot_next_timer(true)", report)
        && fire(session, "DOMContentLoaded", report)
        && drain(session, "__frot_next_image()", report)
        && fire(session, "load", report)
        && drain(session, "__frot_next_timer()", report);
}

/// Fire one lifecycle event; `false` when the budget tripped.
fn fire(session: &Session, name: &str, report: &mut Report) -> bool {
    let errs = drive(session, None, &format!("__frot_fire('{name}')"), report);
    errs.map(|n| report.errors += n as u32).is_some()
}

/// Fire `load` or `error` at the external `<script>` `id` (js.md §4), in the
/// window whose document holds it: a frame's `realm`, or the top's.
pub(super) fn script_event(
    session: &Session,
    realm: Option<usize>,
    id: u32,
    event: &str,
    report: &mut Report,
) {
    let src = format!("__frot_script_event({id},'{event}')");
    if let Some(n) = drive(session, realm, &src, report) {
        report.errors += n as u32;
    }
}

/// Run a driver until it reports nothing left (-1), counting each task's
/// throws; `false` when the budget tripped mid-task.
fn drain(session: &Session, driver: &str, report: &mut Report) -> bool {
    loop {
        match drive(session, None, driver, report) {
            Some(n) if n < 0 => return true,
            Some(n) => report.errors += n as u32,
            None => return false,
        }
    }
}

/// Run one event-loop driver call, returning its integer protocol value, or
/// `None` when the compute budget tripped (which names `stopped: "budget"`). The
/// drivers catch callback throws in JS and encode them in the return value, so
/// the only non-value outcome is a budget hit. Every task is followed by the
/// inserted scripts it queued (js.md §4, `bl-a86b`).
fn drive(session: &Session, realm: Option<usize>, src: &str, report: &mut Report) -> Option<i64> {
    match session.run_task_in(realm, src) {
        Ok(s) => {
            super::queue::drain_inserted(session, report);
            Some(s.trim().parse::<i64>().unwrap_or(0))
        }
        Err(_) => {
            report.stopped = Some(JsStop::Budget);
            None
        }
    }
}
