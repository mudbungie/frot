//! Spike proofs for the four §1 bounding primitives + a real-bundle smoke run.

use std::time::Instant;

use super::*;

fn short_budget(ms: u64) -> Engine {
    Engine::with_limits(JS_MEM_LIMIT, Duration::from_millis(ms))
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
fn a_real_clock_tracks_the_host_and_ignores_manual_advance() {
    // The shipping clock is host-monotonic; the `advance` seam is inert on it
    // (tests only), so a bogus century-advance cannot move production time.
    let c = Clock::real();
    c.advance(Duration::from_secs(100));
    assert!(
        c.elapsed_nanos() < 1_000_000_000,
        "advance leaked into a real clock"
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
    // Primitive 2: wall-clock interrupt. A tight infinite loop must be
    // stopped by the budget rather than hang the host.
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
    let engine = Engine::with_limits(8 * 1024 * 1024, Duration::from_secs(5));
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
