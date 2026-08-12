//! Spike proofs for the four §1 bounding primitives + a real-bundle smoke run.

use std::time::{Duration, Instant};

use super::*;

/// An engine whose *compute* budget is `ms` of real CPU time — the production
/// unit, dialed down so a spinning script trips it fast and deterministically.
fn short_budget(ms: u64) -> Engine {
    Engine::with_bounds(
        JS_MEM_LIMIT,
        Deadline::on(Clock::cpu(), Duration::from_millis(ms)),
        Deadline::network(),
    )
}

#[test]
fn eval_returns_a_value() {
    assert_eq!(Engine::new().eval("1 + 2").unwrap(), "3");
}

#[test]
fn a_manual_clock_is_exactly_what_it_is_advanced_by() {
    // The injected-clock seam (bl-e707): a manual clock is frozen until advanced,
    // and reads exactly the sum of its advances — the basis of every deterministic
    // observable-time proof.
    let c = Clock::manual();
    assert_eq!(c.elapsed_nanos(), 0);
    c.advance(Duration::from_millis(250));
    c.advance(Duration::from_millis(750));
    assert_eq!(c.elapsed_nanos(), 1_000_000_000);
}

#[test]
fn a_host_clock_tracks_the_host_and_ignores_manual_advance() {
    // The shipping clocks are host-monotonic; the `advance` seam is inert on them
    // (tests only), so a bogus century-advance cannot move production time.
    for c in [Clock::wall(), Clock::cpu()] {
        c.advance(Duration::from_secs(100));
        assert!(
            c.elapsed_nanos() < 1_000_000_000,
            "advance leaked into a host clock"
        );
    }
}

#[test]
fn the_cpu_clock_charges_work_and_not_waiting() {
    // The whole point of the split (js.md §5, bl-8dc0): frot's *own* work is
    // spent in CPU time, which host contention does not inflate, while elapsed
    // wall time prices in the host's scheduling luck. Sleeping is elapsed time
    // nobody computed in, so it must reach the wall clock and not the CPU one.
    let (cpu, wall) = (Clock::cpu(), Clock::wall());
    std::thread::sleep(Duration::from_millis(50));
    let (c, w) = (cpu.elapsed_nanos(), wall.elapsed_nanos());
    assert!(w >= 50_000_000, "the wall clock did not see the wait: {w}");
    assert!(c < w / 2, "waiting was charged to CPU: cpu={c} wall={w}");
}

#[test]
fn the_compute_budget_is_cpu_and_the_network_deadline_is_wall() {
    // Two windows, two clocks, driven apart: spending all the *wall* time expires
    // the §6 dispatch window while execution continues untouched, and spending
    // all the *CPU* time stops execution — neither bound can be tripped by the
    // other's unit, which is exactly what made a busy host change the envelope.
    let (cpu, wall) = (Clock::manual(), Clock::manual());
    let engine = Engine::with_bounds(
        JS_MEM_LIMIT,
        Deadline::on(cpu.clone(), Duration::from_millis(EXEC_CPU_MS)),
        Deadline::on(wall.clone(), Duration::from_millis(NET_BUDGET_MS)),
    );
    engine.arm();
    wall.advance(Duration::from_millis(NET_BUDGET_MS));
    assert!(
        engine.deadline().expired(),
        "the wall window is the §6 handle"
    );
    assert_eq!(engine.eval_armed("1 + 1").unwrap(), "2");
    cpu.advance(Duration::from_millis(EXEC_CPU_MS));
    assert_eq!(
        engine.eval_armed("while (true) {}").unwrap_err(),
        EvalError::Budget
    );
}

#[test]
fn default_matches_new() {
    // Cover the `Default` seam Phase-4 consumers will construct through.
    assert_eq!(Engine::default().eval("`ok`").unwrap(), "ok");
}

#[test]
fn an_unstringifiable_completion_is_the_empty_string_not_an_error() {
    // A completion value whose `ToString` throws (a bare null-prototype object,
    // or a framework proxy — Vue's `mount()` returns one) is the *host's*
    // coercion problem, not the page's: it degrades to "" rather than surfacing
    // as an exception (js.md §10 — errors are the page's throws). The script ran
    // fine; only reading its completion for the host protocol failed.
    assert_eq!(Engine::new().eval("Object.create(null)").unwrap(), "");
}

#[test]
fn exception_surfaces_the_message() {
    let err = Engine::new().eval("throw new Error('boom')").unwrap_err();
    match err {
        EvalError::Exception(msg) => assert!(msg.contains("boom"), "got: {msg}"),
        other => panic!("expected exception, got {other:?}"),
    }
}

#[test]
fn syntax_error_is_an_exception() {
    assert!(matches!(
        Engine::new().eval("function (").unwrap_err(),
        EvalError::Exception(_)
    ));
}

#[test]
fn eval_setup_is_exempt_from_the_page_budget() {
    // Prelude install is host setup, not page script (js.md §5, bl-5ac3). A
    // zero budget — a deadline already in the past for any *armed* run — must
    // not trip `eval_setup`: it disarms first, so however slow the environment
    // setup completes. The same engine's page-budget path still trips, proving
    // the exemption is scoped to setup, not a disabled interrupt.
    let engine = short_budget(0);
    assert_eq!(
        engine
            .eval_setup("globalThis.__setup = 7; String(__setup)")
            .unwrap(),
        "7"
    );
    assert_eq!(
        engine.eval("while (true) {}").unwrap_err(),
        EvalError::Budget
    );
}

#[test]
fn interrupt_hook_bounds_an_infinite_loop() {
    // Primitive 2: the budget interrupt. A tight infinite loop burns CPU, so the
    // `EXEC_CPU_MS` budget must stop it rather than let it hang the host.
    let start = Instant::now();
    let err = short_budget(20).eval("while (true) {}").unwrap_err();
    assert_eq!(err, EvalError::Budget);
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "interrupt was too slow"
    );
}

#[test]
fn memory_cap_stops_a_giant_allocation() {
    // Primitive 3: memory limit (default C allocator). A 256 MiB buffer under
    // an 8 MiB heap cap must throw, not OOM the host — and it is not a budget
    // trip (the deadline is generous).
    let engine = Engine::with_bounds(
        8 * 1024 * 1024,
        Deadline::on(Clock::cpu(), Duration::from_secs(5)),
        Deadline::network(),
    );
    assert!(matches!(
        engine
            .eval("new Uint8Array(256 * 1024 * 1024)")
            .unwrap_err(),
        EvalError::Exception(_)
    ));
}

#[test]
fn microtasks_drain_after_eval() {
    // Primitive 4: host-driven job queue. A promise continuation must run
    // during the post-eval drain.
    let engine = Engine::new();
    engine
        .eval("globalThis.x = 0; Promise.resolve().then(() => { globalThis.x = 42; });")
        .unwrap();
    assert_eq!(engine.eval("String(globalThis.x)").unwrap(), "42");
}

#[test]
fn smoke_runs_the_react_production_bundle() {
    // §1 falsifiable check: a real minified framework bundle parses and runs,
    // attaching its API to the global object.
    let engine = Engine::new();
    // A minified UMD bundle falls back to the `self` global when top-level
    // `this` is not the realm object (quickjs eval `this` is undefined). The
    // full `self`/`window` aliasing is subtask 8; the spike needs only this
    // one line to prove the engine *runs* the bundle.
    engine.eval("globalThis.self = globalThis;").unwrap();
    let bundle = include_str!("react.production.min.js");
    engine.eval(bundle).unwrap();
    assert_eq!(
        engine.eval("typeof React.createElement").unwrap(),
        "function"
    );
    assert_eq!(engine.eval("typeof React.useState").unwrap(), "function");
}

#[test]
fn a_page_classic_script_is_sloppy_and_frot_s_own_js_is_strict() {
    // The bl-0679 seam, at the engine: one mode per kind of source. The prelude
    // and the §5 drivers are frot's, written strict; a page classic script is the
    // page's, and browsers run it sloppy unless its own source opts in.
    let engine = Engine::new();
    assert_eq!(
        engine
            .eval_script("sveltekit_ish = 1; typeof sveltekit_ish")
            .unwrap(),
        "number"
    );
    assert!(matches!(
        engine.eval_armed("host_undeclared = 1"),
        Err(EvalError::Exception(_))
    ));
    // A source-level directive still wins in a classic script.
    assert!(matches!(
        engine.eval_script("'use strict'; opted_in = 1"),
        Err(EvalError::Exception(_))
    ));
}
