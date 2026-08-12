//! Binding the syscall table under a starved engine heap.
//!
//! [`super::bind`] registers nine groups of host functions, each of which
//! allocates. Registering the table is *host setup*, not page script, so a
//! failure is not a recoverable page error: [`super::install`] turns any `Err`
//! here into a loud `.expect` panic. The guarantee under test is that every one
//! of those allocation points *reaches* that panic — an `Err`, never a silent
//! half-installed table, which would leave page scripts running against a realm
//! missing some syscalls.
//!
//! Starvation is applied to `bind` **directly**, never through `install`, which
//! runs with the heap cap lifted (`Engine::setup`, js.md §5): the phase after
//! binding *parses* the prelude, and quickjs's parser is not
//! allocation-failure-safe (bl-c385 — `js_parse_block` ignores a failed
//! `push_scope`, so `pop_scope` reads `fd->scopes[garbage]`: a segfault, not an
//! error). Binding allocates but parses nothing, so it is starvable.

use super::*;
use crate::dom::Document;
use crate::fetch::FetchSession;
use crate::js::engine::{Deadline, Engine, EvalError};
use crate::js::geometry::{self, StyleSource};
use crate::js::probe::ProbeLog;
use crate::js::subfetch;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

fn engine(mem_limit: usize) -> Engine {
    Engine::with_bounds(mem_limit, Deadline::compute(), Deadline::network())
}

fn host(engine: &Engine, probe: Option<ProbeLog>) -> Host {
    let session = FetchSession::new(Vec::new());
    let cookie = session.cookie_jar();
    Host {
        doc: Rc::new(RefCell::new(Document::parse("<html><body></body></html>"))),
        console: Rc::new(RefCell::new(Vec::new())),
        geo: Rc::new(RefCell::new(geometry::Geometry::new(StyleSource::Bare))),
        env: Env {
            url: "https://example.com/".into(),
            user_agent: "frot-test/1".into(),
            accept_language: "en-US,en;q=0.5".into(),
        },
        counters: Counters {
            denials: Rc::new(RefCell::new(0)),
            reported: Rc::new(RefCell::new(0)),
            messages: Rc::new(RefCell::new(Vec::new())),
        },
        subfetch: Rc::new(RefCell::new(subfetch::Subfetch::new(
            session,
            "https://example.com/",
            engine.deadline(),
        ))),
        cookie,
        clock: engine.clock(),
        probe,
        current: Rc::new(std::cell::Cell::new(None)),
    }
}

/// Sweeping the heap cap moves the first failing allocation through the whole
/// binding phase; the band is the measured one where the realm itself fits but
/// the table may not. Every cap must bind fully or fail cleanly, and both
/// outcomes must appear — so the starvation is proven to bite rather than the
/// sweep having walked a band that is uniformly roomy.
///
/// The groups are applied as one uniform sequence (`super::GROUPS`), so *which*
/// group a given cap starves does not matter: they share a single failure path,
/// and any cap landing in the binding phase exercises it. That is why a coarse
/// walk suffices here — no hunt for the narrow band that starves one particular
/// group. The sweep runs twice, with and without the measure instrument's extra
/// `__frot_probe` binding (bl-bd4e), because that one rides the same fold.
fn sweep(probe: Option<ProbeLog>) {
    let bound: Vec<bool> = (110_000..140_000)
        .step_by(64)
        .map(|cap| {
            let e = engine(cap);
            let h = host(&e, probe.clone());
            e.context().with(|ctx| bind(&ctx, &h).is_ok())
        })
        .collect();
    assert!(bound.contains(&false), "no cap starved the binding phase");
    assert!(bound.contains(&true), "no cap in the band bound the table");
}

#[test]
fn a_starved_binding_fails_rather_than_half_installing() {
    sweep(None);
}

#[test]
fn a_starved_probe_binding_fails_the_same_way() {
    sweep(Some(Rc::new(RefCell::new(BTreeMap::new()))));
}

/// The bl-c385 fix: `install` is exempt from the heap cap, so a cap far below
/// what evaluating the prelude *peaks* at (measured ~1.3 MiB, against the 256
/// KiB here) still yields a complete API instead of a starved parse. Read back
/// through the same exemption, because the cap is in force again for page
/// scripts — which is the next test.
#[test]
fn install_is_exempt_from_the_heap_cap() {
    let engine = engine(256 * 1024);
    install(&engine, host(&engine, None));
    assert_eq!(
        engine.eval_setup("typeof document.querySelector").unwrap(),
        "function"
    );
}

/// The exemption is scoped to setup: page scripts still meet the cap the engine
/// was built with, so lifting it is not a disabled limit.
#[test]
fn the_heap_cap_is_back_in_force_after_install() {
    let engine = engine(8 * 1024 * 1024);
    install(&engine, host(&engine, None));
    assert_eq!(
        engine.eval("typeof document.querySelector").unwrap(),
        "function"
    );
    assert!(matches!(
        engine.eval("new Uint8Array(32 * 1024 * 1024).length"),
        Err(EvalError::Exception(_))
    ));
}
