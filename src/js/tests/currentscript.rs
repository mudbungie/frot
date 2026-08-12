//! `document.currentScript` (js.md §4.1, bl-a19d): the executing script's
//! identity, which the host owns and the shim only queries.
//!
//! Found live across at least six Next/Turbopack sites (nextjs.org,
//! tailwindcss.com, nodejs.org, …): a fetched chunk derives its own URL from
//! `document.currentScript` to register itself, and with no such property the
//! chunk threw — 36 errors on one page, a shell rendered, in pages Chrome
//! renders. The property is a *query* over the host's one cell, so the facts
//! worth pinning are when it is set, when it is `null`, and that nothing can
//! leave it stale.

use super::{drive, drive_at};

/// Read an attribute the probe wrote onto `<body>`.
fn body_attr(doc: &crate::dom::Document, name: &str) -> String {
    let body = doc.find_by_tag("body")[0];
    let crate::dom::NodeKind::Element(el) = &doc.node(body).kind else {
        unreachable!("body is an element")
    };
    el.attr(name).unwrap_or("<absent>").to_string()
}

#[test]
fn an_inline_classic_script_is_its_own_current_script() {
    // Wrappers are minted per read (js.md §2: they cache no arena state), so the
    // element is identified by what the arena holds, not by object identity.
    let (doc, report) = drive(
        "<body><script id='one' data-chunk='a'>\
           document.body.setAttribute('data-id', document.currentScript.getAttribute('id'));\
           document.body.setAttribute('data-chunk', document.currentScript.dataset.chunk);\
         </script>\
         <script id='two'>\
           document.body.setAttribute('data-second', document.currentScript.getAttribute('id'));\
         </script></body>",
    );
    assert_eq!(body_attr(&doc, "data-id"), "one");
    assert_eq!(body_attr(&doc, "data-chunk"), "a");
    // Each script sees itself, not the one before it.
    assert_eq!(body_attr(&doc, "data-second"), "two");
    assert_eq!((report.scripts, report.errors), (2, 0));
}

#[test]
fn an_external_classic_script_reports_its_own_resolved_src() {
    // The Turbopack shape: a fetched chunk asks which URL it came from. `src`
    // reflects RESOLVED (as in Firefox), so a chunk can `new URL(...)` it — the
    // `data:` source carries its own bytes and is a URL in its own right.
    let (doc, report) = drive(
        "<body><script src=\"data:text/javascript,\
         document.body.setAttribute('data-src', document.currentScript.src);\
         document.body.setAttribute('data-attr', document.currentScript.getAttribute('src'))\
         \"></script></body>",
    );
    assert!(
        body_attr(&doc, "data-src").starts_with("data:text/javascript,"),
        "src: {}",
        body_attr(&doc, "data-src")
    );
    assert_eq!(body_attr(&doc, "data-src"), body_attr(&doc, "data-attr"));
    assert_eq!((report.scripts, report.errors), (1, 0));
}

#[test]
fn a_relative_src_resolves_against_the_document_url() {
    // The chunk-path derivation Next does: `new URL(currentScript.src).pathname`.
    // The subfetch of the absent sibling fails (offline, deterministic), so the
    // script never runs — what is pinned here is the *reflection*, read from the
    // inline script that precedes it.
    let (doc, _report) = drive_at(
        "<body><script>\
           document.body.setAttribute('data-src', document.querySelector('#c').src);\
         </script><script id='c' src='chunks/app.js'></script></body>",
        "file:///frot-no-such-dir/page.html",
    );
    assert_eq!(
        body_attr(&doc, "data-src"),
        "file:///frot-no-such-dir/chunks/app.js"
    );
}

#[test]
fn it_is_null_outside_script_execution_even_after_a_throw() {
    // The cleanup contract: the host restores the prior value after every
    // script, so a script that *throws* cannot strand its own identity — the
    // listener and the timer that follow both see `null`, as in a browser.
    let (doc, report) = drive(
        "<body><script>\
           document.addEventListener('DOMContentLoaded', function () {\
             document.body.setAttribute('data-dcl', String(document.currentScript));\
           });\
           setTimeout(function () {\
             document.body.setAttribute('data-timer', String(document.currentScript));\
           }, 0);\
           throw new Error('the chunk that blew up');\
         </script></body>",
    );
    assert_eq!(body_attr(&doc, "data-dcl"), "null");
    assert_eq!(body_attr(&doc, "data-timer"), "null");
    assert_eq!((report.scripts, report.errors), (1, 1));
}

#[test]
fn a_module_has_no_current_script() {
    // By spec: `currentScript` is `null` during module evaluation. The host never
    // sets it for a module, so this is the same one query answering honestly.
    let (doc, report) = drive(
        "<body><script type='module'>\
           document.body.setAttribute('data-mod', String(document.currentScript));\
         </script></body>",
    );
    assert_eq!(body_attr(&doc, "data-mod"), "null");
    assert_eq!((report.scripts, report.errors), (1, 0));
}
