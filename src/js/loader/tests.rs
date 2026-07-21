//! ES-module resolver/loader tests (js.md §4.1/§6). Every resolve and load
//! branch is driven as real module evaluation through [`run`] — the honest path
//! a `type="module"` page takes — over `file://` sibling fixtures and a local
//! `mockito` server (never the real network, the repo test rule). Counts are the
//! §10 contract: a resolved import is `errors: 0`; an unresolvable specifier or a
//! failed/non-2xx module fetch is one counted error, the run still settling.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::super::{run, run_with, Env, Report, StyleSource};
use crate::dom::Document;
use crate::fetch::FetchSession;

/// A unique temp dir for the `file://` module fixtures, cleaned by the caller.
fn tmpdir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("frot-mod-{tag}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn page_url(dir: &Path) -> String {
    url::Url::from_file_path(dir.join("page.html")).unwrap().to_string()
}

fn env(url: &str) -> Env {
    Env { url: url.into(), user_agent: "frot-test/1".into() }
}

/// Drive a page whose module imports resolve against `url`.
fn drive_at(html: &str, url: &str) -> (Document, Report) {
    run(Document::parse(html), StyleSource::Bare, env(url), &FetchSession::new(Vec::new()))
}

fn module_page(body: &str) -> String {
    format!("<body><div id=root></div><script type=module>{body}</script></body>")
}

#[test]
fn a_relative_import_resolves_loads_and_runs() {
    // The whole graph rides `file://` subfetch: the inline module imports a
    // vendored sibling, whose export drives a real DOM mutation. errors: 0.
    let dir = tmpdir("rel");
    fs::write(dir.join("dep.js"), "export const msg = 'imported!';").unwrap();
    let (doc, r) = drive_at(
        &module_page("import {msg} from './dep.js'; document.getElementById('root').textContent = msg;"),
        &page_url(&dir),
    );
    assert_eq!((r.scripts, r.errors, r.settled), (1, 0, true));
    let root = doc.find_by_tag("div")[0];
    assert!(doc.text_content(root).contains("imported!"), "{}", doc.text_content(root));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn nested_and_root_relative_and_absolute_specifiers_all_resolve() {
    // Cover every non-bare specifier form the resolver joins: `./`, `../`, `/`,
    // and an absolute `file://` URL — each importing chain must load.
    let dir = tmpdir("forms");
    fs::create_dir_all(dir.join("sub")).unwrap();
    fs::write(dir.join("a.js"), "export const a = 1;").unwrap();
    fs::write(dir.join("sub/b.js"), "import {a} from '../a.js'; export const b = a + 1;").unwrap();
    let abs = url::Url::from_file_path(dir.join("a.js")).unwrap().to_string();
    let body = format!(
        "import {{b}} from './sub/b.js'; import {{a as a2}} from '/{}/a.js'; \
         import {{a}} from '{abs}'; \
         document.getElementById('root').textContent = String(a + a2 + b);",
        // reconstruct the absolute dir path for the root-relative `/…` form
        dir.strip_prefix("/").unwrap().to_string_lossy(),
    );
    let (doc, r) = drive_at(&module_page(&body), &page_url(&dir));
    assert_eq!((r.scripts, r.errors, r.settled), (1, 0, true));
    let root = doc.find_by_tag("div")[0];
    assert_eq!(doc.text_content(root), "4"); // a(1) + a2(1) + b(2)
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_bare_specifier_is_unresolvable_and_counts() {
    // No import map (js.md §4.1): a bare specifier fails resolution exactly as in
    // a browser without one — one counted error, the run still settles.
    let dir = tmpdir("bare");
    let (_doc, r) = drive_at(&module_page("import 'lodash';"), &page_url(&dir));
    assert_eq!((r.scripts, r.errors, r.settled), (1, 1, true));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_failed_module_fetch_counts_as_one_error() {
    // A relative import of an absent sibling: the loader's subfetch fails
    // (transport), so the `import` throws — one counted §10 error.
    let dir = tmpdir("miss");
    let (_doc, r) = drive_at(&module_page("import './nope.js';"), &page_url(&dir));
    assert_eq!((r.scripts, r.errors, r.settled), (1, 1, true));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_non_2xx_module_fetch_counts_as_one_error() {
    // A 404 module body loads but is not ok: the loader refuses it (§6), the
    // import throws, counted once.
    let mut server = mockito::Server::new();
    let _m = server.mock("GET", "/dep.js").with_status(404).with_body("nope").create();
    let (_doc, r) = drive_at(
        &module_page("import './dep.js';"),
        &format!("{}/page.html", server.url()),
    );
    assert_eq!((r.scripts, r.errors, r.settled), (1, 1, true));
}

#[test]
fn a_bogus_page_url_fails_resolution_for_a_relative_import() {
    // env.url is a real final URL in production, but the shim tolerates a bogus
    // one; a relative specifier then cannot join against it — a counted resolve
    // failure, not a panic (the resolver's `Url::parse(base)` error arm).
    let (_doc, r) = drive_at(&module_page("import './dep.js';"), "not a url");
    assert_eq!((r.scripts, r.errors, r.settled), (1, 1, true));
}

#[test]
fn a_top_level_throw_counts_exactly_once() {
    // A synchronous module throw rejects its evaluation promise; quickjs may
    // report that twice, but the rejection watcher (js.md §10) folds it to one.
    let dir = tmpdir("throw");
    let (_doc, r) = drive_at(&module_page("throw new Error('boom');"), &page_url(&dir));
    assert_eq!((r.scripts, r.errors, r.settled), (1, 1, true));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_rejected_top_level_await_counts_once() {
    // Top-level await that rejects settles during the microtask drain; the
    // watcher counts the unhandled rejection exactly once.
    let dir = tmpdir("tla");
    let (_doc, r) = drive_at(&module_page("await Promise.reject(new Error('x'));"), &page_url(&dir));
    assert_eq!((r.scripts, r.errors, r.settled), (1, 1, true));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_clean_module_with_no_imports_settles() {
    // The resolver/loader are never consulted; the module resolves cleanly.
    let dir = tmpdir("clean");
    let (doc, r) = drive_at(
        &module_page("document.getElementById('root').textContent = 'ok';"),
        &page_url(&dir),
    );
    assert_eq!((r.scripts, r.errors, r.settled), (1, 0, true));
    let root = doc.find_by_tag("div")[0];
    assert_eq!(doc.text_content(root), "ok");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_endless_module_trips_the_budget_and_is_unsettled() {
    // A module that never returns hits the single wall-clock deadline (§5): the
    // run is unsettled and the trip is counted, like a classic script.
    let dir = tmpdir("budget");
    let (_doc, r) = run_with(
        Document::parse(&module_page("while (true) {}")),
        StyleSource::Bare,
        env(&page_url(&dir)),
        &FetchSession::new(Vec::new()),
        Duration::from_millis(20),
    );
    assert_eq!((r.scripts, r.errors, r.settled), (1, 1, false));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_dead_deadline_refuses_module_loads_and_the_run_is_unsettled() {
    // With the budget already spent when the import dispatches, the §6 seam
    // refuses it ("run budget exhausted") even if the engine interrupt — which
    // fires only between JS instructions — never trips inside the blocking load
    // chain; and the run-driver's conclusion check clears `settled`
    // deterministically (§5): quiescence reached by refusing work is not
    // quiescence within budget. Either path yields the same honest report.
    let dir = tmpdir("dead");
    fs::write(dir.join("dep.js"), "export const x = 1;").unwrap();
    let (_doc, r) = run_with(
        Document::parse(&module_page("import {x} from './dep.js';")),
        StyleSource::Bare,
        env(&page_url(&dir)),
        &FetchSession::new(Vec::new()),
        Duration::ZERO,
    );
    assert_eq!((r.scripts, r.errors, r.settled), (1, 1, false));
    fs::remove_dir_all(&dir).unwrap();
}
