//! Reflected attributes are accessors (js.md §3, bl-d313).
//!
//! `Element.id` was getter-only, so `divWrapper.id = 'myCanvas'` — the plainest
//! line in the MDN basic-modules example — threw in every ES module (modules are
//! strict) and the page never rendered. The property is one accessor over the
//! attribute, so what these pin is that *setting it is setting the attribute*:
//! every other reader of the arena sees it at once, there being nowhere else for
//! the fact to live.

use super::{drive, sess};
use crate::dom::{Document, NodeKind};

/// The `id` attribute the *arena* holds for the first `<div>` — the fact every
/// consumer downstream of the JS phase (views, cascade, layout) reads.
fn div_id(doc: &Document) -> String {
    let div = doc.find_by_tag("div")[0];
    let NodeKind::Element(el) = &doc.node(div).kind else {
        unreachable!("div is an element")
    };
    el.attr("id").unwrap_or("<absent>").to_string()
}

#[test]
fn setting_id_sets_the_attribute_and_every_reader_agrees() {
    let s = sess("<body><div></div></body>");
    // One assignment, then each way of asking: the property, the attribute, the
    // selector engine, and getElementById.
    assert_eq!(
        s.eval(
            "var d = document.querySelector('div');\
             d.id = 'first';\
             [d.id, d.getAttribute('id'), \
              document.querySelector('#first') !== null, \
              document.getElementById('first').tagName].join('|')"
        )
        .unwrap(),
        "first|first|true|DIV"
    );
    // Overwrite, then clear: '' is a present-but-empty attribute, as in a browser.
    assert_eq!(
        s.eval("var d = document.querySelector('div'); d.id = 'second'; d.id")
            .unwrap(),
        "second"
    );
    assert_eq!(
        s.eval(
            "var d = document.querySelector('div'); d.id = ''; \
             [d.id, d.getAttribute('id'), d.hasAttribute('id')].join('|')"
        )
        .unwrap(),
        "||true"
    );
    // Coercion is String(v), so a number reads back as its decimal form.
    assert_eq!(
        s.eval("var d = document.querySelector('div'); d.id = 42; typeof d.id + ':' + d.id")
            .unwrap(),
        "string:42"
    );
    // An absent id is '' — never null or undefined (WebIDL).
    assert_eq!(
        s.eval("var d = document.querySelector('div'); d.removeAttribute('id'); d.id")
            .unwrap(),
        ""
    );
}

#[test]
fn id_set_from_js_is_in_the_arena_the_pipeline_reads() {
    // The serialization half: the run's post-JS document — what the views, the
    // cascade and layout consume — carries the attribute, because the setter
    // wrote it through the same mutation syscall `setAttribute` uses.
    let (doc, report) = drive(
        "<body><script>\
           var d = document.createElement('div'); d.id = 'made'; document.body.appendChild(d);\
         </script></body>",
    );
    assert_eq!(div_id(&doc), "made");
    assert_eq!((report.scripts, report.errors), (1, 0));
}

#[test]
fn a_detached_element_keeps_the_id_it_was_given_when_it_lands() {
    // Order does not matter: the attribute is on the node, not on its position.
    // (The MDN example sets `id` *after* appending; libraries do it before.)
    let s = sess("<body></body>");
    assert_eq!(
        s.eval(
            "var d = document.createElement('div'); d.id = 'later';\
             var seen = document.getElementById('later') === null;\
             document.body.appendChild(d);\
             seen + '|' + document.getElementById('later').id"
        )
        .unwrap(),
        "true|later"
    );
}

#[test]
fn duplicate_ids_resolve_to_the_first_in_document_order() {
    // Nothing indexes ids — `getElementById` is a selector query over the one
    // arena — so a second element taking a live id simply loses the lookup, as
    // in a browser.
    let s = sess("<body><p id='dup'>first</p><p>second</p></body>");
    assert_eq!(
        s.eval(
            "document.querySelectorAll('p')[1].id = 'dup';\
             document.getElementById('dup').textContent + '|' + \
             document.querySelectorAll('#dup').length"
        )
        .unwrap(),
        "first|2"
    );
}

#[test]
fn an_es_module_can_assign_id_and_class_name() {
    // The bug as reported: modules are strict, so assigning a getter-only
    // property is a TypeError that kills the module before it renders. The whole
    // reflected-string family goes through one maker, so `className` and `type`
    // are pinned here beside `id`.
    let (doc, report) = drive(
        "<body><script type='module'>\
           const d = document.createElement('div');\
           d.id = 'from-module'; d.className = 'wrapper'; \
           document.body.appendChild(d);\
           const i = document.createElement('input'); i.type = 'checkbox'; d.appendChild(i);\
           document.body.setAttribute('data-ok', \
             document.getElementById('from-module').className + ':' + \
             document.querySelector('#from-module input').type);\
         </script></body>",
    );
    assert_eq!(div_id(&doc), "from-module");
    let body = doc.find_by_tag("body")[0];
    let NodeKind::Element(el) = &doc.node(body).kind else {
        unreachable!("body is an element")
    };
    assert_eq!(el.attr("data-ok"), Some("wrapper:checkbox"));
    assert_eq!((report.scripts, report.errors), (1, 0));
}

#[test]
fn setting_id_invalidates_the_style_cache_it_shares_a_generation_with() {
    // The §8 geometry/style cache is keyed on the document's generation counter,
    // which every mutation syscall bumps. Because the setter IS `setAttribute`,
    // `d.id = 'tagged'` bumps it too — so the second read is recomputed under the
    // rule the new id now matches, not served stale from before the write.
    let s = crate::js::Session::new(
        Document::parse(
            "<html><head><style>#tagged{display:none}</style></head>\
             <body><div>x</div></body></html>",
        ),
        crate::js::StyleSource::Authored(Vec::new()),
        super::test_env(),
        &crate::fetch::FetchSession::new(Vec::new()),
    );
    assert_eq!(
        s.eval(
            "var d = document.querySelector('div');\
             [getComputedStyle(d).display, (d.id = 'tagged'), getComputedStyle(d).display]\
               .join('|')"
        )
        .unwrap(),
        "block|tagged|none"
    );
}
