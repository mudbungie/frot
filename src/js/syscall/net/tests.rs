//! `outcome_obj` under a starved engine heap.
//!
//! Shaping an [`Outcome`] into a JS object allocates, and the engine runs under
//! a hard heap cap (`engine::JS_MEM_LIMIT`). Every allocation in here is
//! therefore a real failure point. The guarantee under test is that *each* one
//! surfaces as `Err` — a rejected promise / XHR error at the prelude, counted
//! through the §10 failure channel — and never a panic across the FFI boundary.
//!
//! The happy paths (a real `Got` and a real `Failed` reaching the prelude) are
//! covered end-to-end by `js::tests::net` against a live mock origin; these
//! tests exist for the failure edges only.

use super::*;
use crate::js::subfetch::Frozen;
use rquickjs::{Context, Runtime};

fn got() -> Outcome {
    Outcome::Got(Frozen {
        ok: true,
        status: 200,
        url: "https://example.com/x".into(),
        body: "hello".into(),
        headers: vec![
            ("content-type".into(), "text/plain".into()),
            ("x-trailer".into(), "1".into()),
        ],
    })
}

/// Build a realm, cap the heap `headroom` bytes above what the realm already
/// costs, then shape `outcome`. Returns whether it succeeded. The cap is
/// lifted before teardown so dropping the runtime can still allocate.
fn shape_with_headroom(outcome: Outcome, headroom: usize) -> bool {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();
    let used = rt.memory_usage().malloc_size as usize;
    rt.set_memory_limit(used + headroom);
    let ok = ctx.with(|ctx| outcome_obj(&ctx, outcome).is_ok());
    rt.set_memory_limit(usize::MAX);
    ok
}

/// Sweeping the available headroom moves the first failing allocation through
/// every `?` in the success path in turn. No headroom is allowed to panic, and
/// the sweep must span both outcomes — proving the starvation actually bites
/// and that a roomy heap still succeeds.
#[test]
fn got_shaping_fails_cleanly_at_every_allocation_point() {
    let results: Vec<bool> = (0..6000).map(|h| shape_with_headroom(got(), h)).collect();
    assert!(results.contains(&false), "starvation never bit");
    assert!(
        results.contains(&true),
        "never succeeded even with headroom"
    );
}

/// The refusal path allocates too (the object, and the `error` string), and
/// carries the same guarantee.
#[test]
fn failed_shaping_fails_cleanly_at_every_allocation_point() {
    let results: Vec<bool> = (0..3000)
        .step_by(8)
        .map(|h| shape_with_headroom(Outcome::Failed("refused: not GET".into()), h))
        .collect();
    assert!(results.contains(&false), "starvation never bit");
    assert!(
        results.contains(&true),
        "never succeeded even with headroom"
    );
}

/// Build a realm, cap the heap `headroom` above its resting cost, then shape the
/// resource-timing array (`bl-e707`). The same allocation-failure guarantee as
/// `outcome_obj`: every `?` in the success path surfaces as `Err`, never a panic.
fn shape_timings_with_headroom(timings: Vec<(String, f64, f64)>, headroom: usize) -> bool {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();
    let used = rt.memory_usage().malloc_size as usize;
    rt.set_memory_limit(used + headroom);
    let ok = ctx.with(|ctx| timings_array(&ctx, timings).is_ok());
    rt.set_memory_limit(usize::MAX);
    ok
}

/// Sweeping headroom walks the first failing allocation through every `?` of the
/// timings array — the outer array, each triple, and each `[name, start, dur]`
/// element set — proving each fails cleanly and a roomy heap still succeeds.
#[test]
fn timings_shaping_fails_cleanly_at_every_allocation_point() {
    let sample = || {
        vec![
            ("https://example.com/a.js".to_string(), 1.0, 2.0),
            ("https://example.com/b.css".to_string(), 3.0, 4.0),
        ]
    };
    let results: Vec<bool> = (0..4000)
        .map(|h| shape_timings_with_headroom(sample(), h))
        .collect();
    assert!(results.contains(&false), "starvation never bit");
    assert!(
        results.contains(&true),
        "never succeeded even with headroom"
    );
}

/// With a roomy heap the shaped object carries every response field, including
/// the `[[name, value], …]` headers array the prelude reads back.
#[test]
fn a_roomy_heap_shapes_the_full_response_object() {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();
    ctx.with(|ctx| {
        let obj = outcome_obj(&ctx, got()).unwrap();
        assert!(obj.get::<_, bool>("ok").unwrap());
        assert_eq!(obj.get::<_, u16>("status").unwrap(), 200);
        assert_eq!(obj.get::<_, String>("body").unwrap(), "hello");
        let headers: Vec<Vec<String>> = obj.get("headers").unwrap();
        assert_eq!(headers[0], vec!["content-type", "text/plain"]);
        assert_eq!(headers[1], vec!["x-trailer", "1"]);
    });
}
