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
use crate::js::engine::{Deadline, Engine};
use crate::js::geometry::{self, StyleSource};
use crate::js::subfetch;
use std::cell::RefCell;
use std::rc::Rc;

fn env() -> Env {
    Env {
        url: "https://example.com/".into(),
        user_agent: "frot-test/1".into(),
        accept_language: "en-US,en;q=0.5".into(),
    }
}

/// Install the table on an engine capped at `mem_limit` bytes, reporting which
/// phase failed. `Ok(())` is a complete install; `Err(msg)` is the panic the
/// starved phase raised, caught here so the sweep can continue.
fn install_capped(mem_limit: usize) -> Result<(), String> {
    std::panic::catch_unwind(|| {
        let engine = Engine::with_bounds(mem_limit, Deadline::compute(), Deadline::network());
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
                clock: engine.clock(),
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
///
/// The sweep raises ~2 300 *deliberate* panics, so it silences the panic hook to
/// keep the test output readable. The hook is **process-global** and libtest runs
/// tests concurrently, so a blanket `set_hook(|_| {})` silences every *other*
/// test's panic message for the whole (multi-second) window — which is exactly why
/// `bl-a0e7`'s `persona_gold` flake surfaced as a test name in the summary with an
/// **empty** `failures:` block, no panic text even under `--nocapture` and
/// `RUST_BACKTRACE=full`. So the silencer is scoped to the sweeping thread and
/// every other thread's panic is forwarded to the hook we displaced; the probe
/// below pins that forwarding, since a regression here is invisible by definition.
#[test]
fn a_starved_install_panics_rather_than_half_installing() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let sweeper = std::thread::current().id();
    // `Arc` so the displaced hook survives the silencer and can be reinstated: a
    // `set_hook` closure owning it outright could never give it back.
    let prior = Arc::new(std::panic::take_hook());
    let forwarded = Arc::new(AtomicBool::new(false));
    let (fwd, seen) = (Arc::clone(&prior), Arc::clone(&forwarded));
    std::panic::set_hook(Box::new(move |info| {
        // Silence *this* thread's deliberate starvation panics only; anyone else
        // panicking during the window still gets their message printed.
        if std::thread::current().id() != sweeper {
            seen.store(true, Ordering::Relaxed);
            fwd(info);
        }
    }));
    let msgs: Vec<Result<(), String>> =
        (110_000..260_000).step_by(64).map(install_capped).collect();
    let roomy = install_capped(crate::js::engine::JS_MEM_LIMIT);
    // Still inside the silenced window: a foreign thread's panic must reach the
    // displaced hook. Its message printing to stderr is the assertion succeeding.
    let probe = std::thread::spawn(|| panic!("bl-a0e7 probe: this message MUST be visible"));
    assert!(probe.join().is_err(), "the probe thread did not panic");
    std::panic::set_hook(Box::new(move |info| prior(info)));
    assert!(
        forwarded.load(Ordering::Relaxed),
        "the sweep's silencer swallowed a concurrent thread's panic — \
         every other test's failures would be invisible while it runs"
    );

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
