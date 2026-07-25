//! The golden suite's non-bundle classes: pages built from hand-written inline
//! scripts rather than a vendored framework bundle. They exercise the §4.1
//! module loader and the §10 outcome-based re-detection, and they cost too
//! little compute to be worth the CPU guard — the vendored bundles in
//! `golden_tests`, whose harness this shares, are what price the §5 budget.

use super::golden_tests::{env, run_capture, serve};

// Pinned fixture module (tests/fixtures/js/VERSIONS.md) — data, not a dependency.
const ESM_GREETER: &str = include_str!("../../tests/fixtures/js/esm-greeter.mjs");

const BEYOND_PAGE: &str = "<html><body><div id='root'></div>\
    <script src='/app.js'></script></body></html>";
// Content gated behind a click — an interaction frot never performs (js.md §11).
// Every script runs clean and the loop settles, yet the impression is still a
// shell, so the post-settle re-detection honestly reports needs-js (§10).
const BEYOND_APP: &str = "document.addEventListener('click',function(){\
    document.getElementById('root').textContent='revealed';});";

// A `type="module"` page: the entry module (served, so the pre-`--js` body is a
// genuinely empty shell) imports a vendored *relative* module, whose export
// drives the render. A real module graph resolved and linked through the §6
// loader (js.md §4.1) — the entry module's imports resolve against its own
// fetched URL.
const ESM_PAGE: &str = "<html><body><div id='root'></div>\
    <script type='module' src='/app.mjs'></script></body></html>";
const ESM_APP: &str = "import {greet} from './esm-greeter.mjs';\
    document.getElementById('root').textContent = greet('frot');";

#[test]
fn esm_module_page_imports_renders_and_clears_needs_js() {
    let (_s, url) = serve(
        ESM_PAGE,
        &[("/app.mjs", ESM_APP), ("/esm-greeter.mjs", ESM_GREETER)],
    );
    // Static shell (no --js): empty body — the honest signal is needs-js.
    let (_c, before) = run_capture(&[&url, "--out", "text"]);
    assert_eq!(env(&before)["status"], "needs");
    assert_eq!(env(&before)["needs"], serde_json::json!(["js"]));
    // With --js the module links its relative import through the §6 loader and
    // renders: needs-js clears, one script ran clean, settled.
    let (code, out) = run_capture(&[&url, "--js", "--out", "text"]);
    assert_eq!(code, 0);
    let v = env(&out);
    assert_eq!(v["status"], "ok");
    assert!(
        v["out"]
            .as_str()
            .unwrap()
            .contains("Hello from an ES module"),
        "{}",
        v["out"]
    );
    assert_eq!(
        v["js"],
        serde_json::json!({"scripts": 1, "errors": 0, "settled": true})
    );
}

// A module whose import is unresolvable (a bare specifier, no import map, js.md
// §4.1): linking throws, counted once, the run still settling — its body never
// runs, so the empty shell stays honestly needs-js (§10).
const ESM_FAIL_PAGE: &str = "<html><body><div id='root'></div>\
    <script type='module' src='/fail.mjs'></script></body></html>";
const ESM_FAIL_APP: &str = "import _ from 'nonexistent-pkg';\
    document.getElementById('root').textContent = 'unreached';";

#[test]
fn esm_unresolvable_import_counts_one_error_and_still_needs_js() {
    let (_s, url) = serve(ESM_FAIL_PAGE, &[("/fail.mjs", ESM_FAIL_APP)]);
    let (code, out) = run_capture(&[&url, "--js", "--out", "text"]);
    assert_eq!(code, 0);
    let v = env(&out);
    assert_eq!(v["status"], "needs");
    assert_eq!(v["needs"], serde_json::json!(["js"]));
    assert_eq!(
        v["js"],
        serde_json::json!({"scripts": 1, "errors": 1, "settled": true})
    );
}

#[test]
fn beyond_shim_page_settles_clean_yet_still_needs_js() {
    let (_s, url) = serve(BEYOND_PAGE, &[("/app.js", BEYOND_APP)]);
    let (code, out) = run_capture(&[&url, "--js", "--out", "text"]);
    assert_eq!(code, 0);
    let v = env(&out);
    assert_eq!(v["status"], "needs");
    assert_eq!(v["needs"], serde_json::json!(["js"]));
    // Outcome-based, not exception-based (§10): the run succeeded and settled;
    // the empty impression alone is the honest trigger.
    assert_eq!(
        v["js"],
        serde_json::json!({"scripts": 1, "errors": 0, "settled": true})
    );
}
