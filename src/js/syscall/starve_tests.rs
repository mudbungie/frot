//! Installing the syscall table under a starved engine heap.
//!
//! `install` binds seven groups of host functions and then evaluates the
//! prelude, each of which allocates. Registering the table is *host setup*, not
//! page script, so a failure is not a recoverable page error: `install`
//! deliberately `.expect()`s. The guarantee under test is that every one of
//! those allocation points is fatal in exactly that way — a loud panic — and
//! never a silent half-installed table, which would leave page scripts running
//! against a realm missing some syscalls.

use super::*;
use crate::dom::Document;
use crate::js::engine::{Engine, EXEC_BUDGET_MS};
use crate::js::geometry::{self, StyleSource};
use crate::js::subfetch;
use std::time::Duration;

fn env() -> Env {
    Env {
        url: "https://example.com/".into(),
        user_agent: "frot-test/1".into(),
        headers: Vec::new(),
    }
}

/// Install the table on an engine capped at `mem_limit` bytes, reporting which
/// phase failed. `Ok(())` is a complete install; `Err(msg)` is the panic the
/// starved phase raised, caught here so the sweep can continue.
fn install_capped(mem_limit: usize) -> Result<(), String> {
    std::panic::catch_unwind(|| {
        let engine = Engine::with_limits(mem_limit, Duration::from_millis(EXEC_BUDGET_MS));
        let doc = Rc::new(RefCell::new(Document::parse("<html><body></body></html>")));
        let geo = Rc::new(RefCell::new(geometry::Geometry::new(StyleSource::Bare)));
        let counters = Counters {
            denials: Rc::new(RefCell::new(0)),
            reported: Rc::new(RefCell::new(0)),
            messages: Rc::new(RefCell::new(Vec::new())),
        };
        let sf = Rc::new(RefCell::new(subfetch::Subfetch::new(
            "https://example.com/",
            Vec::new(),
            engine.deadline(),
        )));
        install(
            &engine,
            doc,
            Rc::new(RefCell::new(Vec::new())),
            geo,
            env(),
            counters,
            sf,
        );
    })
    .map_err(|p| {
        p.downcast_ref::<String>()
            .cloned()
            .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default()
    })
}

/// Whether `mem_limit` was enough to get the whole syscall table bound (the
/// run may still have died later, in the prelude).
fn table_phase_completed(mem_limit: usize) -> bool {
    match install_capped(mem_limit) {
        Ok(()) => true,
        Err(m) => !m.contains("syscall table"),
    }
}

/// A cap inside the zone where the table stops fitting. Located at run time
/// rather than hardcoded: it tracks the engine's own allocation sizes, which
/// are a QuickJS build detail and not ours to pin. The predicate is not
/// perfectly monotone (GC timing shifts the exact byte at which a phase runs
/// out), so this lands *somewhere* in the transition zone rather than on an
/// exact edge — hence the wide fine sweep around it below.
fn table_phase_boundary() -> usize {
    let (mut lo, mut hi) = (110_000usize, 400_000usize);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if table_phase_completed(mid) {
            hi = mid
        } else {
            lo = mid + 1
        }
    }
    lo
}

/// Sweeping the heap cap moves the first failing allocation through each of the
/// seven binding groups in turn, then through the prelude. Every cap must
/// either install fully or panic. The sweep asserts all three outcomes appear:
/// a table-binding failure, a prelude failure, and a clean install — so the
/// starvation is proven to bite *inside* the binding phase and not only in the
/// much larger prelude that follows it.
///
/// The band is walked coarsely, then byte-by-byte just under the boundary: the
/// last group to be bound has only one function in it, so the caps that starve
/// exactly it are a handful of bytes wide.
#[test]
fn a_starved_install_panics_rather_than_half_installing() {
    let prior = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let boundary = table_phase_boundary();
    // Coarse across the whole starved band: spreads the first failing
    // allocation over the early binding groups, and (past the transition zone)
    // over the far larger prelude.
    let coarse = (110_000..260_000).step_by(64);
    // Byte-by-byte through the transition zone: the last group bound holds a
    // single function, so the caps that starve exactly it are a few bytes wide.
    let fine = boundary.saturating_sub(512)..=boundary + 512;
    let msgs: Vec<Result<(), String>> = coarse.chain(fine).map(install_capped).collect();
    let roomy = install_capped(crate::js::engine::JS_MEM_LIMIT);
    std::panic::set_hook(prior);

    assert!(
        msgs.iter()
            .any(|m| m.as_ref().err().is_some_and(|s| s.contains("syscall table"))),
        "no cap starved the syscall-table binding phase"
    );
    assert!(
        msgs.iter()
            .any(|m| m.as_ref().err().is_some_and(|s| s.contains("prelude"))),
        "no cap starved the prelude phase"
    );
    assert!(roomy.is_ok(), "install failed on a full-size heap");
}
