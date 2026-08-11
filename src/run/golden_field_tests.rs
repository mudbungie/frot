//! Field-trial golden fixtures: pages pinned VERBATIM from live deployments
//! that broke frot in the wild, replayed offline through the full pipeline.
//! Unlike `golden_tests`' hand-assembled shells, each fixture here is the real
//! production page + its real assets (tests/fixtures/js/VERSIONS.md), so the
//! regression it pins is exactly the one the field trial observed. Served from
//! the in-process mock server; tests never touch the live network.

use super::golden_tests::{env, run_guarded, serve};

// TodoMVC "JavaScript ES6 Webpack" (bl-e5c3), pinned 2026-08-10 from
// https://todomvc.com/examples/javascript-es6/dist/. Its bundle's first
// statements alias DOM query results (`querySelector(All)` helpers) and then run
// `NodeList.prototype.forEach = Array.prototype.forEach` at module top level —
// the line that threw "NodeList is not defined" while the server-authored chrome
// still extracted, masking the dead app. base.js is the TodoMVC site chrome: its
// learn.json/GA paths are hostname- and status-gated, so against the mock server
// it runs clean and mutates nothing.
const PAGE: &str = include_str!("../../tests/fixtures/js/todomvc-es6.html");
const BUNDLE: &str = include_str!("../../tests/fixtures/js/todomvc-es6.bundle.js");
const BASE: &str = include_str!("../../tests/fixtures/js/todomvc-es6.base.js");

#[test]
fn todomvc_es6_bundle_initializes_with_zero_errors() {
    let (_s, url) = serve(PAGE, &[("/app.bundle.js", BUNDLE), ("/base.js", BASE)]);
    let (code, out) = run_guarded("todomvc-es6", &[&url, "--js", "--out", "text"]);
    assert_eq!(code, 0);
    let v = env(&out);
    assert_eq!(v["status"], "ok");
    // Both scripts (the deferred bundle + base.js) ran clean and settled — the
    // field trial saw errors=1 here. Live, base.js injects a third (analytics)
    // script behind a `hostname === "todomvc.com"` gate the mock never takes,
    // so offline the honest count is 2 where the live trial reported 3.
    assert_eq!(
        v["js"],
        serde_json::json!({"scripts": 2, "errors": 0, "settled": true})
    );
    // Proof the app actually initialized past the NodeList line: its `load`
    // handler built the view and rendered the live counter into the chrome.
    assert!(
        v["out"].as_str().unwrap().contains("0 items left"),
        "{}",
        v["out"]
    );
}
