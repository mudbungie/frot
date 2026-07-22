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
use crate::fetch::FetchSession;
use crate::js::engine::{Engine, EXEC_BUDGET_MS};
use crate::js::geometry::{self, StyleSource};
use crate::js::subfetch;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

fn env() -> Env {
    Env {
        url: "https://example.com/".into(),
        user_agent: "frot-test/1".into(),
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
        let session = FetchSession::new(Vec::new());
        let cookie = session.cookie_jar();
        let sf = Rc::new(RefCell::new(subfetch::Subfetch::new(
            session,
            "https://example.com/",
            engine.deadline(),
        )));
        install(
            &engine,
            Host {
                doc,
                console: Rc::new(RefCell::new(Vec::new())),
                geo,
                env: env(),
                counters,
                subfetch: sf,
                cookie,
                probe: None,
            },
        );
    })
    .map_err(|p| {
        p.downcast_ref::<String>()
            .cloned()
            .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default()
    })
}

/// Sweeping the heap cap moves the first failing allocation through the binding
/// phase and then through the prelude. Every cap must either install fully or
/// panic. The sweep asserts all three outcomes appear: a table-binding failure,
/// a prelude failure, and a clean install — so the starvation is proven to bite
/// *inside* the binding phase and not only in the much larger prelude that
/// follows it.
///
/// The groups are applied as one uniform sequence (`super::GROUPS`), so *which*
/// group a given cap starves does not matter: they share a single failure path,
/// and any cap landing in the binding phase exercises it. That is why a coarse
/// walk suffices here — no hunt for the narrow band that starves one particular
/// group.
#[test]
fn a_starved_install_panics_rather_than_half_installing() {
    let prior = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let msgs: Vec<Result<(), String>> =
        (110_000..260_000).step_by(64).map(install_capped).collect();
    let roomy = install_capped(crate::js::engine::JS_MEM_LIMIT);
    std::panic::set_hook(prior);

    assert!(
        msgs.iter().any(|m| m
            .as_ref()
            .err()
            .is_some_and(|s| s.contains("syscall table"))),
        "no cap starved the syscall-table binding phase"
    );
    assert!(
        msgs.iter()
            .any(|m| m.as_ref().err().is_some_and(|s| s.contains("prelude"))),
        "no cap starved the prelude phase"
    );
    assert!(roomy.is_ok(), "install failed on a full-size heap");
}
