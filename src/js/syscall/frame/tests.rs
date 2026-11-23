//! Opening a frame under the two bounds a page spends (js.md §14): the heap
//! cap and the compute budget. Every step of [`super::open`] can fail, and each
//! failure must reach the page as a throw — never a half-built window, never a
//! process abort.

use std::time::Duration;

use rquickjs::Value;

use super::open;
use crate::js::engine::{Clock, Deadline, Engine, NetBudget};
use crate::js::syscall::starve_tests::{engine, host};

/// Try to open a frame on `e`'s realm; whether it succeeded.
fn opens(e: &Engine) -> bool {
    e.context().with(|ctx| {
        let agent = Value::new_undefined(ctx.clone());
        open(&ctx, host(e, None), agent).is_ok()
    })
}

/// Sweeping the heap cap walks the first failing allocation through the table
/// and the agent hand-off (the realm's intrinsics and the prelude are built
/// uncapped, `engine::realm`): a starved step fails the open, and the
/// sweep must see both outcomes so the starvation is proven to bite. The band
/// opens just under the measured cap a frame first fits in (~182 KiB), and
/// steps finely so the last allocations — the agent hand-off — starve too.
#[test]
fn a_starved_open_fails_rather_than_half_building_a_window() {
    let mut outcomes = Vec::new();
    for cap in (170_000..400_000).step_by(8) {
        let ok = opens(&engine(cap));
        outcomes.push(ok);
        if ok {
            break;
        }
    }
    assert!(outcomes.contains(&false), "no cap starved the open");
    assert_eq!(
        outcomes.last(),
        Some(&true),
        "no cap in the band opened a frame"
    );
}

/// The frame's prelude is the page's work: a spent compute budget stops it.
#[test]
fn opening_a_frame_spends_the_compute_budget() {
    let clock = Clock::manual();
    let e = Engine::with_bounds(
        crate::js::engine::JS_MEM_LIMIT,
        Deadline::on(clock.clone(), Duration::from_millis(1)),
        NetBudget::network(),
    );
    e.arm();
    clock.advance(Duration::from_millis(2));
    assert!(!opens(&e));
}
