//! Bounded virtual-clock event-loop tests (js.md §5): timers/rAF against virtual
//! time, the horizon, lifecycle events, microtask ordering, and the single
//! wall-clock deadline. Virtual time keeps every timing assertion deterministic
//! with no real sleeping; budget trips use a tight bound (the 4.1 spike pattern).

use super::{drive, drive_bounded};
use crate::dom::NodeKind;

/// The single `data-*` attribute a test writes to record loop progress.
fn attr(doc: &crate::dom::Document, name: &str) -> Option<String> {
    let body = doc.find_by_tag("body")[0];
    let NodeKind::Element(el) = &doc.node(body).kind else {
        unreachable!("body is an element")
    };
    el.attr(name).map(str::to_string)
}

#[test]
fn a_budget_trip_stops_the_queue_and_marks_unsettled() {
    // The first script spins out the wall-clock budget; the loop breaks, so the
    // second never runs and the run is unsettled (§5). A tight budget keeps the
    // test fast and deterministic.
    let (doc, report) = drive_bounded(
        "<body><script>while(true){}</script>\
         <script>document.body.appendChild(document.createElement('hr'))</script></body>",
        20,
    );
    assert_eq!((report.scripts, report.errors, report.settled), (1, 1, false));
    assert_eq!(doc.find_by_tag("hr").len(), 0);
}

#[test]
fn a_settimeout_callback_fires_against_virtual_time() {
    // §5 virtual clock: a 500 ms timer fires with no real sleep, mutating the
    // shared arena; the run settles.
    let (doc, report) = drive(
        "<body><script>\
         setTimeout(function () { document.body.appendChild(document.createElement('hr')); }, 500);\
         </script></body>",
    );
    assert_eq!((report.scripts, report.errors, report.settled), (1, 0, true));
    assert_eq!(doc.find_by_tag("hr").len(), 1);
}

#[test]
fn a_timer_past_the_horizon_never_fires() {
    // §5 horizon: a task due past VIRTUAL_HORIZON_MS (10 s) is dropped —
    // "setTimeout(f, 30_000) never fires" — yet the run still settles.
    let (doc, report) = drive(
        "<body><script>\
         setTimeout(function () { document.body.appendChild(document.createElement('hr')); }, 30000);\
         </script></body>",
    );
    assert!(report.settled);
    assert_eq!(doc.find_by_tag("hr").len(), 0);
}

#[test]
fn setinterval_self_terminates_at_the_horizon() {
    // §5: an interval poller fires at 4 s and 8 s; the 12 s tick is past the
    // horizon, so it self-terminates with no per-API cap.
    let (doc, report) = drive(
        "<body><script>\
         var n = 0;\
         setInterval(function () { n++; document.body.setAttribute('data-n', String(n)); }, 4000);\
         </script></body>",
    );
    assert!(report.settled);
    assert_eq!(attr(&doc, "data-n").as_deref(), Some("2"));
}

#[test]
fn requestanimationframe_chains_then_stops() {
    // §5: rAF is a 16 ms virtual timer; a bounded chain re-requests twice, so
    // three frames run (at 16/32/48 ms virtual). The callback gets a timestamp.
    let (doc, report) = drive(
        "<body><script>\
         var c = 0;\
         function frame(ts) { c++; document.body.setAttribute('data-c', String(c)); if (c < 3) requestAnimationFrame(frame); }\
         requestAnimationFrame(frame);\
         </script></body>",
    );
    assert!(report.settled);
    assert_eq!(attr(&doc, "data-c").as_deref(), Some("3"));
}

#[test]
fn cleartimeout_cancels_a_pending_timer() {
    // clearTimeout/clearInterval remove a scheduled task before it is due (§5).
    let (doc, report) = drive(
        "<body><script>\
         var id = setTimeout(function () { document.body.appendChild(document.createElement('hr')); }, 100);\
         clearTimeout(id);\
         </script></body>",
    );
    assert!(report.settled);
    assert_eq!(doc.find_by_tag("hr").len(), 0);
}

#[test]
fn lifecycle_events_fire_in_order_after_scripts() {
    // §4.4: after the script queue drains, DOMContentLoaded then load fire on
    // document and window; their handlers mutate the arena.
    let (doc, report) = drive(
        "<body><script>\
         document.addEventListener('DOMContentLoaded', function () { document.body.appendChild(document.createElement('main')); });\
         window.addEventListener('load', function () { document.body.appendChild(document.createElement('footer')); });\
         </script></body>",
    );
    assert_eq!((report.errors, report.settled), (0, true));
    assert_eq!(doc.find_by_tag("main").len(), 1);
    assert_eq!(doc.find_by_tag("footer").len(), 1);
}

#[test]
fn a_throwing_lifecycle_handler_is_counted_and_the_run_continues() {
    // §5: a load handler (registered via the onload property) that throws is
    // counted; the loop still settles.
    let (_doc, report) = drive(
        "<body><script>window.onload = function () { throw new Error('boom'); };</script></body>",
    );
    assert_eq!((report.errors, report.settled), (1, true));
}

#[test]
fn a_throwing_timer_callback_is_counted_and_the_run_continues() {
    // §5: an unhandled throw in a timer callback aborts that task, is counted,
    // and the loop continues to quiescence.
    let (_doc, report) = drive(
        "<body><script>setTimeout(function () { throw new Error('x'); }, 10);</script></body>",
    );
    assert_eq!((report.errors, report.settled), (1, true));
}

#[test]
fn a_budget_trip_inside_a_timer_marks_the_run_unsettled() {
    // §5: the single deadline spans the settle loop, so a timer callback that
    // spins trips the budget and marks the whole run unsettled.
    let (_doc, report) = drive_bounded(
        "<body><script>setTimeout(function () { while (true) {} }, 10);</script></body>",
        20,
    );
    assert!(!report.settled);
}

#[test]
fn a_budget_trip_inside_a_lifecycle_handler_stops_the_loop() {
    // §5: a DOMContentLoaded handler that spins trips the deadline during the
    // fire phase; the loop stops before `load` and the run is unsettled.
    let (_doc, report) = drive_bounded(
        "<body><script>\
         document.addEventListener('DOMContentLoaded', function () { while (true) {} });\
         </script></body>",
        20,
    );
    assert!(!report.settled);
}

#[test]
fn an_unhandled_promise_rejection_is_counted() {
    // §10: a promise that rejects with no handler is a counted error — the run
    // still settles (rejections are weather, not a budget trip).
    let (_doc, report) = drive("<body><script>Promise.reject(new Error('nope'));</script></body>");
    assert_eq!((report.errors, report.settled), (1, true));
}

#[test]
fn a_late_handled_rejection_is_not_counted() {
    // §10: a rejection reported unhandled but caught later (here in a timer) is
    // un-counted by the tracker's matching handle report — net zero.
    let (_doc, report) = drive(
        "<body><script>\
         var p = Promise.reject(new Error('x'));\
         setTimeout(function () { p.catch(function () {}); }, 10);\
         </script></body>",
    );
    assert_eq!((report.errors, report.settled), (0, true));
}

#[test]
fn microtasks_drain_between_macrotasks() {
    // §5: a promise continuation scheduled inside a timer callback runs before
    // the next timer — the host drains the job queue between macrotasks.
    let (doc, report) = drive(
        "<body><script>\
         setTimeout(function () {\
           Promise.resolve().then(function () { document.body.setAttribute('data-p', 'ok'); });\
         }, 10);\
         setTimeout(function () {\
           document.body.setAttribute('data-seen', document.body.getAttribute('data-p') || 'no');\
         }, 20);\
         </script></body>",
    );
    assert!(report.settled);
    assert_eq!(attr(&doc, "data-seen").as_deref(), Some("ok"));
}
