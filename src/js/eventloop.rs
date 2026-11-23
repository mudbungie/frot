//! The §5 settle loop, after the script queue drains: the host-driven
//! macrotasks, one `Session::run_task` each.

use super::{Report, Session};
use crate::envelope::JsStop;

/// The settle loop (§5): `DOMContentLoaded`, the image tasks `load` waits for
/// (`prelude/imageload.js`), `load`, then the virtual-clock timers until none is
/// due before the horizon. Each step is one host-driven macrotask (microtasks
/// drain between, in the engine); a budget trip stops the loop, unsettled.
pub(super) fn run(session: &Session, report: &mut Report) {
    let _ = fire(session, "DOMContentLoaded", report)
        && drain(session, "__frot_next_image()", report)
        && fire(session, "load", report)
        && drain(session, "__frot_next_timer()", report);
}

/// Fire one lifecycle event; `false` when the budget tripped.
fn fire(session: &Session, name: &str, report: &mut Report) -> bool {
    let errs = drive(session, &format!("__frot_fire('{name}')"), report);
    errs.map(|n| report.errors += n as u32).is_some()
}

/// Run a driver until it reports nothing left (-1), counting each task's
/// throws; `false` when the budget tripped mid-task.
fn drain(session: &Session, driver: &str, report: &mut Report) -> bool {
    loop {
        match drive(session, driver, report) {
            Some(n) if n < 0 => return true,
            Some(n) => report.errors += n as u32,
            None => return false,
        }
    }
}

/// Run one event-loop driver call, returning its integer protocol value, or
/// `None` when the compute budget tripped (which names `stopped: "budget"`). The
/// drivers catch callback throws in JS and encode them in the return value, so
/// the only non-value outcome is a budget hit.
fn drive(session: &Session, src: &str, report: &mut Report) -> Option<i64> {
    match session.run_task(src) {
        Ok(s) => Some(s.trim().parse::<i64>().unwrap_or(0)),
        Err(_) => {
            report.stopped = Some(JsStop::Budget);
            None
        }
    }
}
