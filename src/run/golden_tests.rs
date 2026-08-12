//! Phase 4.9 golden fixture suite (js.md §9): pinned, vendored real-world
//! framework bundles driven end-to-end through the full pipeline (fetch → parse
//! → `--js` → needs → views), asserting on the actual envelope and views, not
//! unit internals. This is where the prelude's breadth is exercised (js.md §3:
//! "prelude coverage actually lives in the fixtures"). The bundles are served
//! from an in-process `mockito` server, so tests never touch the live network.
//!
//! The bundle classes live here — a React 17 UMD SPA shell (renders + clears
//! `needs-js`), a React 19 `createRoot` app, a Vue 3 app, and a jQuery page
//! (mutations show up in the views) — and each is driven through
//! [`run_guarded`], which enforces the §5 CPU margin. The hand-written classes
//! (ES modules, the beyond-the-shim page) are in `golden_shim_tests`, which
//! shares this module's harness.

use super::*;
use crate::js::engine::{Clock, Deadline, EXEC_CPU_MS, NET_BUDGET_MS};
use crate::js::Bounds;
use std::time::Duration;

// Pinned bundles (tests/fixtures/js/VERSIONS.md) — data, not dependencies.
const REACT: &str = include_str!("../../tests/fixtures/js/react.production.min.js");
const REACT_DOM: &str = include_str!("../../tests/fixtures/js/react-dom.production.min.js");
const VUE: &str = include_str!("../../tests/fixtures/js/vue.global.prod.js");
const JQUERY: &str = include_str!("../../tests/fixtures/js/jquery.min.js");
// React 19 (createRoot, concurrent) — react+react-dom+app bundled to one IIFE.
const REACT19_TODO: &str = include_str!("../../tests/fixtures/js/react19-todo.bundle.js");

/// The §5 CPU-margin guard (`bl-18df`): the share of the compute budget a golden
/// bundle may spend and still be a *margin* rather than a near miss. Derived from
/// [`EXEC_CPU_MS`], never a second hardcoded number — resize the budget and the
/// guard moves with it.
///
/// Half is deliberately loose: the worst cost yet observed is 130 ms (measured
/// 2026-07-24 on 16 cores — 32–59 ms release, 41–61 ms under `llvm-cov`, 74–130
/// ms under `llvm-cov` at 4× CPU oversubscription), so the guard keeps a ≥3.8×
/// margin and cannot fail on load, while still catching a real regression —
/// prelude bloat, an accidental O(n²) syscall, a bundle that starts spinning.
const CPU_GUARD_MS: u64 = EXEC_CPU_MS / 2;

/// The §5 bounds every golden runs under (`bl-c81a`).
///
/// Compute stays on the real [`Clock::cpu`] — host-independent, and the very
/// thing [`run_guarded`] meters. The network window keeps its shipping budget
/// but spends it on a *frozen* [`Clock::manual`] the test never advances,
/// because these fixtures are served from an in-process `mockito` server:
/// there is no real network wait to be honest about, and the wall window —
/// armed once per run and spanning the whole run — was instead being spent by
/// the *compute* of the heaviest bundle under parallel build load, reporting
/// `stopped: "network"` on a correct run. That is the same host-load dependence
/// `bl-8dc0` removed from the compute side, arriving through the other clock.
/// The bound is not mocked away, only handed to the test: `golden_bounds_tests`
/// drives it to expiry and still gets `stopped: "network"`.
pub(super) fn golden_bounds() -> Bounds {
    Bounds {
        cpu: Deadline::compute(),
        net: Deadline::on(Clock::manual(), Duration::from_millis(NET_BUDGET_MS)),
    }
}

pub(super) fn run_capture(args: &[&str]) -> (u8, String) {
    capture_bounded(args, golden_bounds())
}

/// [`run_capture`] with the run's §5 bounds supplied — the seam the goldens'
/// frozen network window and the negative controls both go through.
pub(super) fn capture_bounded(args: &[&str], bounds: Bounds) -> (u8, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let code = run_bounded(&argv, &mut out, &mut err, bounds);
    (code, String::from_utf8(out).unwrap())
}

/// [`run_capture`] with the §5 margin enforced: `fixture`'s whole run must cost
/// under [`CPU_GUARD_MS`] of **CPU** time.
///
/// Measured on a [`Clock::cpu`] — the same `CLOCK_THREAD_CPUTIME_ID` the compute
/// deadline is spent on, and the JS run is single-threaded on this one — so the
/// assertion is exactly as load-sensitive as `settled` itself now is, which is
/// the property under test. Reading a wall clock here would reintroduce the
/// host-load dependence `bl-8dc0` removed. The measurement spans the whole
/// pipeline, not just the JS phase, so it can only over-charge the budget.
///
/// This is the enforcement of what js.md §13 OQ-1 previously only *recorded*: a
/// margin that lives in a doc stops being true without anyone noticing.
pub(super) fn run_guarded(fixture: &str, args: &[&str]) -> (u8, String) {
    let cpu = Clock::cpu();
    let captured = run_capture(args);
    let ms = cpu.elapsed_nanos() / 1_000_000;
    assert!(
        ms < CPU_GUARD_MS,
        "golden fixture {fixture} burned {ms} ms CPU, over the §5 guard of \
         {CPU_GUARD_MS} ms (half of EXEC_CPU_MS = {EXEC_CPU_MS} ms)"
    );
    captured
}

pub(super) fn env(s: &str) -> Value {
    serde_json::from_str(s.trim()).expect("envelope JSON")
}

/// Serve a shell page plus its named JS assets, and return the live server (kept
/// alive by the caller) and its base URL. Every fixture is a shell whose only
/// body content arrives via external `<script src>` (so the pre-`--js` document
/// is genuinely empty — the honest `needs-js` baseline, js.md §10).
pub(super) fn serve(page: &str, assets: &[(&str, &str)]) -> (mockito::ServerGuard, String) {
    let mut server = mockito::Server::new();
    server
        .mock("GET", "/")
        .with_status(200)
        .with_body(page)
        .create();
    for (path, body) in assets {
        server
            .mock("GET", *path)
            .with_status(200)
            .with_header("content-type", "text/javascript")
            .with_body(*body)
            .create();
    }
    let url = server.url();
    (server, url)
}

const REACT_PAGE: &str = "<html><body><div id='root'></div>\
    <script src='/react.js'></script><script src='/react-dom.js'></script>\
    <script src='/app.js'></script></body></html>";
const REACT_APP: &str = "var e=React.createElement;function App(){\
    return e('h1',null,'Hello from React');}\
    ReactDOM.render(e(App),document.getElementById('root'));";

#[test]
fn react_shell_needs_js_without_and_renders_with() {
    let (_s, url) = serve(
        REACT_PAGE,
        &[
            ("/react.js", REACT),
            ("/react-dom.js", REACT_DOM),
            ("/app.js", REACT_APP),
        ],
    );
    // Static shell (no --js): empty body, so the honest signal is needs-js.
    let (_c, before) = run_capture(&[&url, "--out", "text"]);
    assert_eq!(env(&before)["status"], "needs");
    assert_eq!(env(&before)["needs"], serde_json::json!(["js"]));
    // With --js the UMD bundle + ReactDOM.render fill the root: needs-js clears,
    // three scripts ran clean and settled.
    let (code, out) = run_guarded("react17", &[&url, "--js", "--out", "text"]);
    assert_eq!(code, 0);
    let v = env(&out);
    assert_eq!(v["status"], "ok");
    assert!(
        v["out"].as_str().unwrap().contains("Hello from React"),
        "{}",
        v["out"]
    );
    assert_eq!(
        v["js"],
        serde_json::json!({"scripts": 3, "errors": 0, "settled": true})
    );
}

// The capstone (bl-4640): a *modern*-React CSR app must actually render. Unlike
// the React 17 fixture (classic `ReactDOM.render`), this is `createRoot` — the
// concurrent path a real React 18/19 SPA uses, mounting into an initially-empty
// `<div id='root'>`. Its init reads `history.state` and `new URL(location.href)
// .searchParams` (the SPA-router surface the field trial's React 19 todomvc
// crashed on when both were absent), and it mounts a controlled `<input>` (which
// made React set `node.defaultValue` — a getter-only accessor until this task
// gave it a setter). One `<script>` because react+react-dom+app are one bundle.
const REACT19_PAGE: &str = "<html><body><div id='root'></div>\
    <script src='/app.js'></script></body></html>";

#[test]
fn react19_createroot_app_renders_from_empty_root() {
    let (_s, url) = serve(REACT19_PAGE, &[("/app.js", REACT19_TODO)]);
    // Static shell (no --js): the empty root is honestly needs-js.
    let (_c, before) = run_capture(&[&url, "--out", "text"]);
    assert_eq!(env(&before)["status"], "needs");
    assert_eq!(env(&before)["needs"], serde_json::json!(["js"]));
    // With --js the app renders client-side: needs-js clears, one script ran
    // clean and the scheduler-driven mount settled with zero errors.
    let (code, out) = run_guarded("react19", &[&url, "--js", "--out", "text"]);
    assert_eq!(code, 0);
    let v = env(&out);
    assert_eq!(v["status"], "ok");
    let text = v["out"].as_str().unwrap();
    // Rendered content is present: the heading, the URL-derived filter line
    // (proves URL/searchParams ran), and the seeded todo items.
    assert!(
        text.contains("Todos") && text.contains("Filter: all"),
        "{text}"
    );
    assert!(
        text.contains("Buy milk") && text.contains("Ship frot"),
        "{text}"
    );
    assert_eq!(
        v["js"],
        serde_json::json!({"scripts": 1, "errors": 0, "settled": true})
    );
    // The ax view proves the interactive shape: a named textbox and a 3-item
    // list, mounted where the shell had only an empty div.
    let (_c, ax) = run_capture(&[&url, "--js", "--out", "ax"]);
    let tree = env(&ax)["out"].to_string();
    assert!(
        tree.contains("\"textbox\"") && tree.contains("New todo"),
        "{tree}"
    );
    assert!(tree.contains("\"list\""), "{tree}");
}

const VUE_PAGE: &str = "<html><body><div id='app'></div>\
    <script src='/vue.js'></script><script src='/app.js'></script></body></html>";
const VUE_APP: &str = "Vue.createApp({template:'<p>{{msg}}</p>',\
    data(){return{msg:'Hello from Vue'};}}).mount('#app');";

#[test]
fn vue_app_renders_and_clears_needs_js() {
    let (_s, url) = serve(VUE_PAGE, &[("/vue.js", VUE), ("/app.js", VUE_APP)]);
    let (_c, before) = run_capture(&[&url, "--out", "text"]);
    assert_eq!(env(&before)["status"], "needs");
    // Vue's compiler + reactive mount render the template; the completion value
    // of mount() is a proxy that will not ToString — the engine's defensive
    // coercion keeps that a non-error (js.md §10).
    let (code, out) = run_guarded("vue3", &[&url, "--js", "--out", "text"]);
    assert_eq!(code, 0);
    let v = env(&out);
    assert_eq!(v["status"], "ok");
    assert!(
        v["out"].as_str().unwrap().contains("Hello from Vue"),
        "{}",
        v["out"]
    );
    assert_eq!(
        v["js"],
        serde_json::json!({"scripts": 2, "errors": 0, "settled": true})
    );
}

const JQUERY_PAGE: &str = "<html><body><div id='root'></div>\
    <script src='/jquery.js'></script><script src='/app.js'></script></body></html>";
// Ready fires on the host-dispatched DOMContentLoaded (js.md §4.4); the injected
// <p class="mounted"> is a real arena mutation both content and structural views
// then surface.
const JQUERY_APP: &str = "$(function(){\
    $('#root').append('<p class=\"mounted\">jQuery mounted</p>');});";

#[test]
fn jquery_page_settles_and_mutations_show_in_views() {
    let (_s, url) = serve(
        JQUERY_PAGE,
        &[("/jquery.js", JQUERY), ("/app.js", JQUERY_APP)],
    );
    let (code, out) = run_guarded("jquery", &[&url, "--js", "--out", "text"]);
    assert_eq!(code, 0);
    let v = env(&out);
    assert_eq!(v["status"], "ok");
    assert!(
        v["out"].as_str().unwrap().contains("jQuery mounted"),
        "{}",
        v["out"]
    );
    assert_eq!(
        v["js"],
        serde_json::json!({"scripts": 2, "errors": 0, "settled": true})
    );
    // Structural proof the DOM mutation landed: the raw `dom` view shows the
    // injected element, class and all.
    let (_c, dom) = run_capture(&[&url, "--js", "--out", "dom"]);
    let dom_str = env(&dom)["out"].to_string();
    assert!(
        dom_str.contains("mounted"),
        "dom view missing injected node: {dom_str}"
    );
}
