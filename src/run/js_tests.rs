//! End-to-end `--js` behavior (`docs/design/js.md` §9/§10): page scripts run
//! before the rest of the pipeline, the additive `js` block reports the run,
//! and `needs::detect` re-runs on the post-JS document — an SPA shell the shim
//! fills clears `needs-js`, one it can't still surfaces it.

use super::*;

fn run_capture(args: &[&str]) -> (u8, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let code = run_io(&argv, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn parse_envelope(s: &str) -> Value {
    serde_json::from_str(s.trim()).expect("envelope JSON")
}

fn serve(body: &str) -> (mockito::ServerGuard, mockito::Mock, String) {
    let mut server = mockito::Server::new();
    let m = server
        .mock("GET", "/")
        .with_status(200)
        .with_body(body)
        .create();
    let url = server.url();
    (server, m, url)
}

// An SPA shell: empty body, filled only by its external bundle. The inline
// script's own source would count as body text (the Phase-1 heuristic reads
// `text_content`), so the shell case must use an external `src`.
const SHELL: &str =
    "<html><body><div id='root'></div><script src='/app.js'></script></body></html>";

#[test]
fn js_runs_inline_scripts_populates_content_and_emits_the_js_block() {
    let (_s, _m, url) = serve(
        "<html><body><div id='root'></div>\
         <script>document.getElementById('root').textContent = 'hello world';</script>\
         </body></html>",
    );
    // Without `--js` the inline script never runs, so the text view (which skips
    // `<script>`) is empty.
    let (_c, before, _) = run_capture(&[&url, "--out", "text"]);
    assert_eq!(parse_envelope(&before)["out"], "");
    assert!(parse_envelope(&before).get("js").is_none());
    // With `--js` the script mutates the DOM and the block reports the run.
    let (code, out, _) = run_capture(&[&url, "--js", "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["out"], "hello world");
    assert_eq!(v["js"], serde_json::json!({"scripts": 1, "errors": 0, "settled": true}));
}

#[test]
fn without_js_a_shell_needs_js_and_emits_no_block() {
    // §10: the `js` block is emitted only when `--js` is on.
    let (_s, _m, url) = serve(SHELL);
    let (code, out, _) = run_capture(&[&url, "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "needs");
    assert_eq!(v["needs"], serde_json::json!(["js"]));
    assert!(v.get("js").is_none());
}

#[test]
fn a_shell_whose_bundle_does_not_load_still_needs_js_with_the_block() {
    // The server mocks only `/`, so the `/app.js` subfetch resolves to a non-2xx
    // (§6) — the external script is skipped-and-counted (§4.2), the body stays
    // empty, `needs-js` survives (§10), and the honest `js` block rides along.
    let (_s, _m, url) = serve(SHELL);
    let (code, out, _) = run_capture(&[&url, "--js", "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "needs");
    assert_eq!(v["needs"], serde_json::json!(["js"]));
    assert_eq!(v["js"], serde_json::json!({"scripts": 0, "errors": 1, "settled": true}));
}

#[test]
fn an_external_bundle_is_fetched_and_run_clearing_needs_js() {
    // §4/§6: the external `<script src>` is fetched under the subfetch policy and
    // executed; it fills the shell, so `needs-js` clears and the run reports one
    // script, no errors.
    let mut server = mockito::Server::new();
    let _page = server
        .mock("GET", "/")
        .with_status(200)
        .with_body("<html><body><div id='root'></div><script src='/app.js'></script></body></html>")
        .create();
    let _bundle = server
        .mock("GET", "/app.js")
        .with_status(200)
        .with_body("document.getElementById('root').textContent = 'hydrated';")
        .create();
    let url = server.url();
    let (code, out, _) = run_capture(&[&url, "--js", "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["out"], "hydrated");
    assert_eq!(v["js"], serde_json::json!({"scripts": 1, "errors": 0, "settled": true}));
}

#[test]
fn a_page_that_fetches_json_populates_from_the_response() {
    // §6 fetch end to end: a same-origin GET is frozen and its body drives a DOM
    // mutation inside the `.then` microtask, before the pipeline reads the DOM.
    let mut server = mockito::Server::new();
    let _page = server
        .mock("GET", "/")
        .with_status(200)
        .with_body(
            "<html><body><div id='root'></div><script>\
             fetch('/data').then(function(r){return r.text();})\
             .then(function(t){document.getElementById('root').textContent = t;});\
             </script></body></html>",
        )
        .create();
    let _data = server.mock("GET", "/data").with_status(200).with_body("from-fetch").create();
    let url = server.url();
    let (code, out, _) = run_capture(&[&url, "--js", "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["out"], "from-fetch");
    assert_eq!(v["js"]["errors"], 0);
}

#[test]
fn js_with_css_seeds_the_authored_style_source() {
    // The `--js --css` path builds the geometry cache from authored CSS
    // (`StyleSource::Authored`, js.md §8); here it just runs end-to-end.
    let (_s, _m, url) = serve(
        "<html><head><style>#root{display:block}</style></head>\
         <body><div id='root'></div>\
         <script>document.getElementById('root').textContent = 'styled';</script>\
         </body></html>",
    );
    let (code, out, _) = run_capture(&[&url, "--js", "--css", "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["out"], "styled");
    assert_eq!(v["js"], serde_json::json!({"scripts": 1, "errors": 0, "settled": true}));
}

#[test]
fn a_failed_external_src_is_skipped_and_counted() {
    // §4.2: the `/app.js` subfetch fails (unmocked → non-2xx), so the external
    // script is skipped-and-counted like a failed stylesheet — `errors: 1`, and
    // the static DOM is otherwise untouched.
    let (_s, _m, url) = serve(
        "<html><body><p>static</p><script src='/app.js'></script></body></html>",
    );
    let (code, out, _) = run_capture(&[&url, "--js", "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["out"], "static");
    assert_eq!(v["js"], serde_json::json!({"scripts": 0, "errors": 1, "settled": true}));
}
