//! Injected-clock proofs (bl-e707): the one monotonic clock feeds
//! `performance`/`Date` coherently from a single origin, reduces to the profile
//! precision, advances with real CPU/host/network elapsed, jumps to a virtual
//! timer's due time, and is the *same* clock the §5/§6 deadline bounds the run
//! with. A `manual()` clock the test advances stands in for elapsed time, so
//! every assertion is exact — no real sleeping, no flake.

use std::time::Duration;

use crate::dom::Document;
use crate::fetch::FetchSession;
use crate::js::engine::Clock;

use super::{test_env, Session, StyleSource};

/// A session whose engine reads the injected `clock` — the test keeps its own
/// clone (a manual clock shares state), so advancing it moves the run's one clock.
fn sess_clock(clock: Clock) -> Session {
    Session::with_clock(
        Document::parse("<html><body></body></html>"),
        StyleSource::Bare,
        test_env(),
        &FetchSession::new(Vec::new()),
        Duration::from_secs(1),
        clock,
    )
}

#[test]
fn observable_time_advances_with_real_elapsed_is_coherent_and_never_backward() {
    let clock = Clock::manual();
    let s = sess_clock(clock.clone());
    s.begin();
    // Frozen clock: the observable clock starts at the origin (zero elapsed).
    assert_eq!(s.run_task("performance.now()").unwrap(), "0");
    // 250 ms of real CPU/host/network time passes (a busy script, a blocking
    // fetch): the observable clock advances by exactly that — no perpetual zero.
    clock.advance(Duration::from_millis(250));
    assert_eq!(s.run_task("performance.now()").unwrap(), "250");
    // Date/performance coherence: both derive from ONE origin, so their
    // difference is the origin and Date.now() - timeOrigin == performance.now().
    assert_eq!(
        s.run_task("String(Date.now() - performance.timeOrigin)")
            .unwrap(),
        "250"
    );
    // Precision reduction (identity.md §9 — Firefox's 1 ms clamp): a sub-ms
    // advance is not yet observable, and time never runs backward.
    clock.advance(Duration::from_micros(400));
    assert_eq!(s.run_task("performance.now()").unwrap(), "250");
    // The remaining 600 µs completes the millisecond — now it ticks.
    clock.advance(Duration::from_micros(600));
    assert_eq!(s.run_task("performance.now()").unwrap(), "251");
}

#[test]
fn a_virtual_timer_jump_fires_promptly_but_observes_its_due_time() {
    let clock = Clock::manual();
    let s = sess_clock(clock.clone());
    s.begin();
    // setTimeout(5000) with the clock frozen: no real time will pass, yet the
    // callback must observe time >= 5000 ms (§5 — the virtual clock jumps).
    s.run_task(
        "globalThis.fired=false; \
         setTimeout(function(){ globalThis.fired=true; globalThis.at=performance.now(); }, 5000);",
    )
    .unwrap();
    // It fires immediately in host wall time (the clock never moved), returning 0
    // callback errors, and the reading jumped to its due time.
    assert_eq!(s.run_task("__frot_next_timer()").unwrap(), "0");
    assert_eq!(s.run_task("String(fired)").unwrap(), "true");
    assert_eq!(s.run_task("String(at >= 5000)").unwrap(), "true");
    // Nothing else is due — the loop has settled.
    assert_eq!(s.run_task("__frot_next_timer()").unwrap(), "-1");
    // Real elapsed still stacks ON TOP of the virtual jump: it never goes back.
    clock.advance(Duration::from_millis(10));
    assert_eq!(
        s.run_task("String(performance.now() >= 5010)").unwrap(),
        "true"
    );
}

#[test]
fn the_one_clock_bounds_the_deadline_and_feeds_observable_time_together() {
    // One authority: the same injected clock spends the §5/§6 budget AND is the
    // observable elapsed. Advancing it (as a blocking subfetch's wall time would)
    // moves both — proving fetch duration is captured where the deadline is read.
    let clock = Clock::manual();
    let s = sess_clock(clock.clone()); // 1 s budget, armed below
    s.begin();
    clock.advance(Duration::from_millis(250));
    assert_eq!(s.run_task("performance.now()").unwrap(), "250");
    assert!(!s.deadline_expired());
    // Past the 1 s budget: the deadline is spent by the very same elapsed the
    // observable clock reports (read via `eval`, which re-arms, only afterwards).
    clock.advance(Duration::from_millis(800));
    assert!(s.deadline_expired());
    assert_eq!(s.eval("performance.now()").unwrap(), "1050");
}
