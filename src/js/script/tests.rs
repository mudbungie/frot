//! `<script>` classification unit tests (js.md §4.1): totality on non-element
//! ids and the JS-type recognizer, driven directly for full Rust coverage.

use super::{classify, is_js_type, Script};
use crate::dom::Document;

#[test]
fn classify_is_total_on_non_element_nodes() {
    // `find_by_tag` only yields elements, but classify stays total: a text node
    // has nothing to run.
    let doc = Document::parse("<body>hi</body>");
    let body = doc.find_by_tag("body")[0];
    let text = doc.node(body).children[0];
    assert!(matches!(classify(&doc, text), Script::Skip));
}

#[test]
fn js_type_recognizes_javascript_and_modules_only() {
    assert!(is_js_type(None));
    assert!(is_js_type(Some(" Text/JavaScript ")));
    assert!(is_js_type(Some("module")));
    assert!(!is_js_type(Some("application/json")));
    assert!(!is_js_type(Some("text/template")));
}
