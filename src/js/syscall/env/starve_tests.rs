//! `location_obj` / `url_parse` under a starved engine heap.
//!
//! Both build a JS object property by property, and the engine runs under a
//! hard heap cap (`engine::JS_MEM_LIMIT`). The guarantee under test is that an
//! allocation failure at *any* of those points surfaces as `Err` — which the
//! caller turns into a host error, counted through the §10 failure channel —
//! rather than panicking across the FFI boundary or yielding a half-built
//! `location`. The behavioural contract of both functions is covered by
//! `super::tests`; these tests exist for the failure edges only.

use super::*;
use rquickjs::{Context, Runtime};

/// Build a realm, cap the heap `headroom` bytes above what the realm already
/// costs, then run `f`. The cap is lifted before teardown so dropping the
/// runtime can still allocate.
fn starved<F>(headroom: usize, f: F) -> bool
where
    F: for<'js> FnOnce(&Ctx<'js>) -> rquickjs::Result<Object<'js>>,
{
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();
    let used = rt.memory_usage().malloc_size as usize;
    rt.set_memory_limit(used + headroom);
    let ok = ctx.with(|ctx| f(&ctx).is_ok());
    rt.set_memory_limit(usize::MAX);
    ok
}

/// Sweeping the available headroom moves the first failing allocation through
/// each property `decompose` writes. Every point must fail cleanly, and the
/// sweep must span both outcomes — proving the starvation bites and that a
/// roomy heap still builds the object.
fn sweep<F>(range: usize, f: F)
where
    F: Copy + for<'js> FnOnce(&Ctx<'js>) -> rquickjs::Result<Object<'js>>,
{
    let results: Vec<bool> = (0..range).map(|h| starved(h, f)).collect();
    assert!(results.contains(&false), "starvation never bit");
    assert!(
        results.contains(&true),
        "never succeeded even with headroom"
    );
}

/// A parseable URL decomposes into nine allocated members.
#[test]
fn location_of_a_parseable_url_fails_cleanly_when_starved() {
    sweep(6000, |ctx| {
        location_obj(ctx, "https://user@example.com:8443/a/b?q=1#frag")
    });
}

/// An unparseable URL still writes `href` plus eight empty members.
#[test]
fn location_of_an_unparseable_url_fails_cleanly_when_starved() {
    sweep(4000, |ctx| location_obj(ctx, "not a url at all"));
}

/// `new URL(spec)` — the valid path decomposes; the invalid path writes only
/// the `valid` flag.
#[test]
fn url_parse_fails_cleanly_when_starved() {
    sweep(6000, |ctx| {
        url_parse(ctx, "../c?x=1#h", Some("https://example.com:99/a/b"))
    });
    sweep(4000, |ctx| {
        url_parse(ctx, "https://example.com/plain", None)
    });
    sweep(3000, |ctx| url_parse(ctx, "::nonsense::", None));
}
