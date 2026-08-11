//! Field-trial golden fixtures: pages pinned VERBATIM from live deployments
//! that broke frot in the wild, replayed offline through the full pipeline.
//! Unlike `golden_tests`' hand-assembled shells, each fixture here is the real
//! production page + its real assets (tests/fixtures/js/VERSIONS.md), so the
//! regression it pins is exactly the one the field trial observed. Served from
//! the in-process mock server; tests never touch the live network.

use super::golden_tests::{env, run_guarded, serve};

// TodoMVC "React" (bl-3a36), pinned 2026-08-10 from
// https://todomvc.com/examples/react/dist/. A React 18-era CSR shell whose
// commit phase runs `autoFocus && stateNode.focus()` on the new-todo input;
// without Element#focus the throw is caught by React's captureCommitPhaseError
// and, with no error boundary, the whole root unmounts — a settled run, one
// "not a function" report, and an empty shell still reading needs-js. The
// vendored react19 golden carries the same commit code but its app never sets
// autoFocus, which is exactly why it passed while this page died.
const REACT_PAGE: &str = include_str!("../../tests/fixtures/js/react-todomvc.html");
const REACT_BUNDLE: &str = include_str!("../../tests/fixtures/js/react-todomvc.bundle.js");
const REACT_BASE: &str = include_str!("../../tests/fixtures/js/react-todomvc.base.js");

// Vite production shell (bl-3a36), page pinned 2026-08-10 from
// https://vite-react-tailwind-template.pages.dev/; the module is the verbatim
// `vite:build-import-analysis` modulepreload polyfill cut from its bundle plus
// one render line (VERSIONS.md). Every Vite build ships this polyfill at module
// top level: absent `link.relList` its `supports("modulepreload")` early-return
// never runs and it falls through to `new MutationObserver` — the module
// rejects, which is a count-only §10 error, so the field trial saw errors=1
// with messages=[] even under --js-errors.
const VITE_PAGE: &str = include_str!("../../tests/fixtures/js/vite-preload.html");
const VITE_MODULE: &str = include_str!("../../tests/fixtures/js/vite-preload.mjs");

#[test]
fn react_todomvc_autofocus_commit_renders_and_clears_needs_js() {
    let (_s, url) = serve(
        REACT_PAGE,
        &[("/app.bundle.js", REACT_BUNDLE), ("/base.js", REACT_BASE)],
    );
    let (code, out) = run_guarded("react-todomvc", &[&url, "--js", "--out", "text"]);
    assert_eq!(code, 0);
    let v = env(&out);
    assert_eq!(v["status"], "ok");
    // The deferred bundle + base.js ran clean and settled (live, base.js injects
    // a hostname-gated third analytics script the mock never takes, so the field
    // trial's scripts=3 is honestly 2 offline — same as the es6 fixture).
    assert_eq!(
        v["js"],
        serde_json::json!({"scripts": 2, "errors": 0, "settled": true})
    );
    // Proof the commit phase survived autoFocus: the app rendered its footer.
    assert!(
        v["out"].as_str().unwrap().contains("0 items left"),
        "{}",
        v["out"]
    );
}

#[test]
fn vite_modulepreload_polyfill_early_returns_on_rellist() {
    let (_s, url) = serve(VITE_PAGE, &[("/assets/index-D22riwjH.js", VITE_MODULE)]);
    let (code, out) = run_guarded("vite-preload", &[&url, "--js", "--out", "text"]);
    assert_eq!(code, 0);
    let v = env(&out);
    assert_eq!(v["status"], "ok");
    // Both scripts — the inline theme snippet and the module — ran clean: the
    // polyfill took its relList.supports("modulepreload") early return instead
    // of dying on the deliberately absent MutationObserver.
    assert_eq!(
        v["js"],
        serde_json::json!({"scripts": 2, "errors": 0, "settled": true})
    );
    assert!(
        v["out"].as_str().unwrap().contains("polyfill survived"),
        "{}",
        v["out"]
    );
}

// TodoMVC "Vue" (bl-79db), pinned 2026-08-10 from
// https://todomvc.com/examples/vue/dist/. A Vue 3.5/Vite 8 CSR shell whose
// entire UI lives behind vue-router: RouterView at START_LOCATION mounts a
// comment-node placeholder, and the router-ready re-render computes its patch
// container as `parentNode(placeholder)`. With createComment a host-side fake
// (`_id: -1`) that insertion silently dropped, that parent read undefined and
// the patch died in Vue's scheduler — which logs caught errors via
// console.error in production builds, a channel frot records but does not
// count. Hence the field trial's silent verdict: settled, errors=0, empty
// shell, needs-js.
const VUE_PAGE: &str = include_str!("../../tests/fixtures/js/vue-todomvc.html");
const VUE_BUNDLE: &str = include_str!("../../tests/fixtures/js/vue-todomvc.bundle.js");
const VUE_BASE: &str = include_str!("../../tests/fixtures/js/vue-todomvc.base.js");

#[test]
fn vue_todomvc_router_view_renders_via_real_comment_anchor() {
    let (_s, url) = serve(
        VUE_PAGE,
        &[
            ("/assets/index-CO9Gq1IP.js", VUE_BUNDLE),
            ("/base.js", VUE_BASE),
        ],
    );
    let (code, out) = run_guarded("vue-todomvc", &[&url, "--js", "--out", "text"]);
    assert_eq!(code, 0);
    let v = env(&out);
    assert_eq!(v["status"], "ok");
    // The module bundle + base.js ran clean and settled (live, base.js injects
    // a hostname-gated analytics script the mock never takes, so the field
    // trial's scripts=3 is honestly 2 offline — same as the other TodoMVCs).
    assert_eq!(
        v["js"],
        serde_json::json!({"scripts": 2, "errors": 0, "settled": true})
    );
    // Proof the router-ready patch navigated from the real comment anchor: the
    // route component mounted and rendered the footer counter.
    assert!(
        v["out"].as_str().unwrap().contains("0 items left"),
        "{}",
        v["out"]
    );
}

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
