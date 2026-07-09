//! Spike proofs for the four §1 bounding primitives + a real-bundle smoke run.

use super::*;

fn short_budget(ms: u64) -> Engine {
    Engine::with_limits(JS_MEM_LIMIT, Duration::from_millis(ms))
}

#[test]
fn eval_returns_a_value() {
    assert_eq!(Engine::new().eval("1 + 2").unwrap(), "3");
}

#[test]
fn default_matches_new() {
    // Cover the `Default` seam Phase-4 consumers will construct through.
    assert_eq!(Engine::default().eval("`ok`").unwrap(), "ok");
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
