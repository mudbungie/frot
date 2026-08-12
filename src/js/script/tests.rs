//! `<script>` classification unit tests (js.md §4.1): totality on non-element
//! ids and the JS-type recognizer, driven directly for full Rust coverage.

use super::{classify, external_intent, initial_externals, is_js_type, Script};
use crate::dom::Document;
use crate::fetch::Intent;

#[test]
fn classify_is_total_on_non_element_nodes() {
    // `find_by_tag` only yields elements, but classify stays total: a text node
    // has nothing to run.
    let doc = Document::parse("<body>hi</body>");
    let body = doc.find_by_tag("body")[0];
    let text = doc.node(body).children[0];
    assert!(matches!(classify(&doc, text, None), Script::Skip));
}

#[test]
fn js_type_recognizes_javascript_and_modules_only() {
    assert!(is_js_type(None));
    assert!(is_js_type(Some(" Text/JavaScript ")));
    assert!(is_js_type(Some("module")));
    assert!(!is_js_type(Some("application/json")));
    assert!(!is_js_type(Some("text/template")));
}

#[test]
fn external_intent_maps_module_and_classic() {
    assert!(matches!(external_intent(true), Intent::Module));
    assert!(matches!(external_intent(false), Intent::ClassicScript));
}

#[test]
fn initial_externals_lists_only_static_external_scripts_in_order() {
    // Classic + module externals, in document order, with their intents; the
    // inline, the non-JS `type`, and the empty `src` contribute no resource.
    let doc = Document::parse(
        "<script src='/a.js'></script>\
         <script>inline()</script>\
         <script type='module' src='/b.mjs'></script>\
         <script type='application/json' src='/c.json'></script>\
         <script src=''></script>",
    );
    let got = initial_externals(&doc, "https://example.com/dir/page");
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].0, "https://example.com/a.js");
    assert!(matches!(got[0].1, Intent::ClassicScript));
    assert_eq!(got[1].0, "https://example.com/b.mjs");
    assert!(matches!(got[1].1, Intent::Module));
}

#[test]
fn external_src_resolves_against_the_document_base() {
    // bl-409e: discovery emits what a browser would fetch — the `src` joined
    // onto the document base URL, not the raw attribute and not the page URL.
    let doc = Document::parse(
        "<base href='https://cdn.example.com/build/'>\
         <script src='app.js'></script>\
         <script type='module' src='/pinned.mjs'></script>",
    );
    let got = initial_externals(&doc, "https://example.com/dir/page");
    assert_eq!(got[0].0, "https://cdn.example.com/build/app.js");
    assert_eq!(got[1].0, "https://cdn.example.com/pinned.mjs");
}

#[test]
fn external_src_without_a_base_keeps_the_raw_reference() {
    // No absolute anchor (an unparseable page URL): the `src` passes through
    // rather than being invented, and the §6 fetch fails on it as before.
    let doc = Document::parse("<script src='/a.js'></script>");
    assert_eq!(initial_externals(&doc, "not a url")[0].0, "/a.js");
}
