//! Offline coverage for the capability-surface probe instrument (`bl-bd4e`).
//!
//! Two claims are guarded here, both offline and deterministic (the live corpus
//! in `examples/probe_corpus.rs` is dated evidence, never CI): the instrument
//! **records** the surfaces a page touches, and it **preserves** frot's real
//! values while doing so (the honesty line, `identity.md` §10). Prelude JS is
//! not `llvm-cov`-visible; these end-to-end runs exercise it, and the Rust
//! `__frot_probe` sink and [`measure`]/[`rank`] paths, to 100%.

use std::collections::HashMap;

use super::{rank, ProbeLog};
use crate::dom::Document;
use crate::js::{measure, Env, Report, Session, StyleSource};

fn env() -> Env {
    Env {
        url: "https://example.com/".into(),
        user_agent: "frot-test/1".into(),
        accept_language: "en-US,en;q=0.5".into(),
    }
}

fn probe_page(html: &str) -> (Report, HashMap<String, u32>) {
    let m = measure(Document::parse(html), StyleSource::Bare, env());
    (m.report, m.probes.into_iter().collect())
}

/// One page that touches every watched surface once; the instrument records each
/// through `__frot_probe`, and the run reports a clean single-script execution.
#[test]
fn every_watched_surface_is_recorded() {
    let (report, p) = probe_page(
        "<body><script>\
         try { document.createElement('canvas').getContext('2d'); } catch (e) {}\
         try { document.createElement('canvas').getContext('webgl'); } catch (e) {}\
         void (typeof AudioContext); void (typeof OfflineAudioContext);\
         void (typeof Worker); void (typeof WebAssembly);\
         void (typeof RTCPeerConnection); void (typeof WebGLRenderingContext);\
         void (typeof WebSocket); void (typeof crypto); void (typeof performance);\
         void (typeof Intl); void (typeof screen); void (typeof devicePixelRatio);\
         void navigator.plugins; void navigator.mimeTypes; void navigator.mediaDevices;\
         void navigator.hardwareConcurrency; void navigator.webdriver;\
         void navigator.languages; void document.fonts;\
         void new Date().getTimezoneOffset();\
         void Object.getOwnPropertyDescriptor(navigator, 'webdriver');\
         void Object.getPrototypeOf(navigator);\
         (function named() {}).toString();\
         </script></body>",
    );
    assert_eq!(
        (report.scripts, report.errors, report.settled),
        (1, 0, true)
    );
    for surface in [
        "canvas.getContext(2d)",
        "canvas.getContext(webgl)",
        "AudioContext",
        "OfflineAudioContext",
        "Worker",
        "WebAssembly",
        "RTCPeerConnection",
        "WebGLRenderingContext",
        "WebSocket",
        "crypto",
        "performance",
        "Intl",
        "screen",
        "devicePixelRatio",
        "navigator.plugins",
        "navigator.mimeTypes",
        "navigator.mediaDevices",
        "navigator.hardwareConcurrency",
        "navigator.webdriver",
        "navigator.languages",
        "document.fonts",
        "Date.getTimezoneOffset",
        "Object.getOwnPropertyDescriptor(identity)",
        "Object.getPrototypeOf(identity)",
        "Function.prototype.toString",
    ] {
        assert!(p.contains_key(surface), "missing probe: {surface}");
    }
}

/// The tally is ranked by count descending, ties broken by surface name — the
/// property that makes the table deterministic for a fixed page (VISION §1).
#[test]
fn probes_rank_by_count_then_name() {
    let m = measure(
        Document::parse(
            "<body><script>\
             void (typeof Worker); void (typeof Worker);\
             void (typeof AudioContext); void (typeof crypto);\
             </script></body>",
        ),
        StyleSource::Bare,
        env(),
    );
    // Worker (2) outranks the count-1 group, which is name-sorted.
    assert_eq!(m.probes[0], ("Worker".to_string(), 2));
    let names: Vec<&str> = m.probes.iter().map(|(n, _)| n.as_str()).collect();
    let audio = names.iter().position(|&n| n == "AudioContext").unwrap();
    let crypto = names.iter().position(|&n| n == "crypto").unwrap();
    assert!(audio < crypto, "tie broken by name ascending: {names:?}");
}

/// [`rank`] directly, so the tie-break comparator is covered without leaning on
/// engine iteration order.
#[test]
fn rank_orders_a_raw_tally() {
    let mut counts = std::collections::BTreeMap::new();
    counts.insert("b".to_string(), 1u32);
    counts.insert("a".to_string(), 1u32);
    counts.insert("z".to_string(), 3u32);
    assert_eq!(
        rank(&counts),
        vec![
            ("z".to_string(), 3),
            ("a".to_string(), 1),
            ("b".to_string(), 1),
        ]
    );
}

/// A page that touches nothing watched records nothing — the instrument is a
/// passive observer, not a source of probes of its own.
#[test]
fn an_uninstrumented_page_records_nothing() {
    let m = measure(
        Document::parse("<body><script>var x = 1 + 1;</script></body>"),
        StyleSource::Bare,
        env(),
    );
    assert!(m.probes.is_empty(), "unexpected probes: {:?}", m.probes);
    assert_eq!((m.report.scripts, m.report.errors), (1, 0));
}

/// The honesty line (`identity.md` §10): wrapping a surface to record it must not
/// change what frot returns. Absent globals stay `undefined`, `getContext` stays
/// `null`, and the identity facts stay exactly what the shim reports — the read
/// is merely counted.
#[test]
fn recording_preserves_frots_real_values() {
    let log = ProbeLog::default();
    let s = Session::measuring(
        Document::parse("<body></body>"),
        StyleSource::Bare,
        env(),
        &crate::fetch::FetchSession::new(Vec::new()),
        log.clone(),
    );
    assert_eq!(s.eval("typeof AudioContext").unwrap(), "undefined");
    // Worker is now a present masquerade (bl-342a) — the probe still forwards
    // frot's real value unchanged, and that value is now the constructor.
    assert_eq!(s.eval("typeof Worker").unwrap(), "function");
    assert_eq!(
        s.eval("String(document.createElement('canvas').getContext('2d'))")
            .unwrap(),
        "null"
    );
    // A non-CANVAS element's getContext stays undefined (the else arm).
    assert_eq!(
        s.eval("String(document.createElement('div').getContext('2d'))")
            .unwrap(),
        "undefined"
    );
    // Identity facts are unchanged, and userAgent (not a fingerprint watchlist
    // entry) is forwarded silently — read by every page, not a probe signal.
    assert_eq!(s.eval("navigator.userAgent").unwrap(), "frot-test/1");
    assert_eq!(s.eval("String(navigator.webdriver)").unwrap(), "false");
    // …but the webdriver read *was* recorded, and userAgent was not.
    assert!(log.borrow().contains_key("navigator.webdriver"));
    assert!(!log.borrow().contains_key("navigator.userAgent"));
}
