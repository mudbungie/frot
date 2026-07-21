//! Phase 4.9 golden fixture suite (js.md §9): pinned, vendored real-world
//! framework bundles driven end-to-end through the full pipeline (fetch → parse
//! → `--js` → needs → views), asserting on the actual envelope and views, not
//! unit internals. This is where the prelude's breadth is exercised (js.md §3:
//! "prelude coverage actually lives in the fixtures"). The bundles are served
//! from an in-process `mockito` server, so tests never touch the live network.
//!
//! Four classes, per the task: a React SPA shell (renders + clears `needs-js`),
//! a Vue app, a jQuery page (mutations show up in the views), and a
//! beyond-the-shim page that stays a shell and honestly reports `needs-js` even
//! though every script ran clean (§10 outcome-based re-detection).

use super::*;

// Pinned bundles (tests/fixtures/js/VERSIONS.md) — data, not dependencies.
const REACT: &str = include_str!("../../tests/fixtures/js/react.production.min.js");
const REACT_DOM: &str = include_str!("../../tests/fixtures/js/react-dom.production.min.js");
const VUE: &str = include_str!("../../tests/fixtures/js/vue.global.prod.js");
const JQUERY: &str = include_str!("../../tests/fixtures/js/jquery.min.js");
const ESM_GREETER: &str = include_str!("../../tests/fixtures/js/esm-greeter.mjs");
// React 19 (createRoot, concurrent) — react+react-dom+app bundled to one IIFE.
const REACT19_TODO: &str = include_str!("../../tests/fixtures/js/react19-todo.bundle.js");

fn run_capture(args: &[&str]) -> (u8, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let code = run_io(&argv, &mut out, &mut err);
    (code, String::from_utf8(out).unwrap())
}

fn env(s: &str) -> Value {
    serde_json::from_str(s.trim()).expect("envelope JSON")
}

/// Serve a shell page plus its named JS assets, and return the live server (kept
/// alive by the caller) and its base URL. Every fixture is a shell whose only
/// body content arrives via external `<script src>` (so the pre-`--js` document
/// is genuinely empty — the honest `needs-js` baseline, js.md §10).
fn serve(page: &str, assets: &[(&str, &str)]) -> (mockito::ServerGuard, String) {
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
    let (code, out) = run_capture(&[&url, "--js", "--out", "text"]);
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
    let (code, out) = run_capture(&[&url, "--js", "--out", "text"]);
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
    let (code, out) = run_capture(&[&url, "--js", "--out", "text"]);
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
    let (code, out) = run_capture(&[&url, "--js", "--out", "text"]);
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
