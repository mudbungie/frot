//! Properties browsers make WRITABLE (js.md §3, bl-273b) — `outerHTML`, the
//! `[PutForwards]` pair `style`/`classList`, and `document.body`.
//!
//! Each was an accessor with a getter and no setter. Since bl-0679 classic page
//! scripts are sloppy, as in a browser, so an assignment to one was a **silent
//! no-op** (a TypeError only in a module): the page ran to completion, reported
//! no error, and simply did not carry the change — the degraded-looks-complete
//! failure VISION principle 5 forbids. So every test here writes and then asks
//! the *arena* — the one place the fact lives — whether the write arrived.
//!
//! `outerHTML`'s getter was separately dishonest: it serialized the tag and
//! nothing else, so a read-modify-write silently deleted every attribute in the
//! subtree. The expected markup below is Chrome 139's, byte for byte.

use super::{drive, sess};
use crate::dom::{Document, NodeKind};

/// An attribute the *arena* holds — what the views, the cascade and layout read.
fn attr(doc: &Document, tag: &str, name: &str) -> Option<String> {
    let id = doc.find_by_tag(tag)[0];
    let NodeKind::Element(el) = &doc.node(id).kind else {
        unreachable!("{tag} is an element")
    };
    el.attr(name).map(str::to_string)
}

#[test]
fn outer_html_serializes_attributes_void_elements_and_escapes() {
    let s = sess("<html><body><div id='host'></div></body></html>");
    // The reported bug: `<div id=x class=y>` read back as `<div>`.
    assert_eq!(
        s.eval(
            "var h = document.querySelector('#host');\
             h.innerHTML = '<div id=\"x\" class=\"y z\" data-q=\"1\">hi</div>';\
             h.firstChild.outerHTML"
        )
        .unwrap(),
        "<div id=\"x\" class=\"y z\" data-q=\"1\">hi</div>"
    );
    // A void element takes no end tag and no self-closing slash.
    assert_eq!(
        s.eval("document.createElement('br').outerHTML").unwrap(),
        "<br>"
    );
    // Escaping (WHATWG HTML §13.3): `&`, `<`, `>` always; `\"` only in an
    // attribute value, where it would otherwise end the value.
    assert_eq!(
        s.eval(
            "var d = document.createElement('div');\
             d.setAttribute('t', 'a\"b&c<d>e'); d.outerHTML"
        )
        .unwrap(),
        "<div t=\"a&quot;b&amp;c&lt;d&gt;e\"></div>"
    );
    assert_eq!(
        s.eval("var d = document.createElement('div'); d.textContent = 'a&b<c>d\"e'; d.outerHTML")
            .unwrap(),
        "<div>a&amp;b&lt;c&gt;d\"e</div>"
    );
    // Raw-text elements are the exception: a `<` inside <style>/<script> stays a
    // `<`, because re-parsing that content never sees markup.
    assert_eq!(
        s.eval(
            "var h = document.querySelector('#host');\
             h.innerHTML = '<style>a{content:\"<&>\"}</style>'; h.innerHTML"
        )
        .unwrap(),
        "<style>a{content:\"<&>\"}</style>"
    );
    // A comment serializes as itself, so Vue's router anchors survive a round
    // trip instead of dissolving into the empty string.
    assert_eq!(
        s.eval(
            "var h = document.querySelector('#host');\
             h.innerHTML = '<!--c--><i>q</i>'; h.innerHTML + '|' + h.firstChild.textContent"
        )
        .unwrap(),
        "<!--c--><i>q</i>|c"
    );
}

#[test]
fn a_read_modify_write_of_inner_html_keeps_the_whole_subtree() {
    // The harm the getter did: `el.innerHTML = el.innerHTML` — the shape every
    // append-a-row snippet has — used to strip every attribute in the subtree
    // and leave the page looking fine. One serializer, both directions, so the
    // arena after the round trip is the arena before it plus the appended row.
    let (doc, report) = drive(
        "<body><ul id='list'><li class='row' data-k='1'><a href='/a'>one</a></li></ul>\
         <script>\
           var l = document.getElementById('list');\
           l.innerHTML = l.innerHTML + '<li class=\"row\" data-k=\"2\">two</li>';\
         </script></body>",
    );
    assert_eq!(attr(&doc, "li", "class").as_deref(), Some("row"));
    assert_eq!(attr(&doc, "li", "data-k").as_deref(), Some("1"));
    assert_eq!(attr(&doc, "a", "href").as_deref(), Some("/a"));
    assert_eq!(doc.find_by_tag("li").len(), 2);
    assert_eq!((report.scripts, report.errors), (1, 0));
}

#[test]
fn setting_outer_html_replaces_the_element_where_it_stood() {
    let s = sess("<html><body><div id='host'></div></body></html>");
    // The fragment lands in the replaced element's slot among its siblings, and
    // the old element is out of the tree.
    assert_eq!(
        s.eval(
            "var h = document.querySelector('#host');\
             h.innerHTML = '<i>a</i><div id=\"gone\">x</div><i>b</i>';\
             var d = document.querySelector('#gone');\
             d.outerHTML = '<p>1</p><p>2</p>';\
             h.innerHTML + '|' + (d.parentNode === null)"
        )
        .unwrap(),
        "<i>a</i><p>1</p><p>2</p><i>b</i>|true"
    );
    // The empty string is not a special case — it is the general path with
    // nothing to insert, so the element is simply removed.
    assert_eq!(
        s.eval(
            "var h = document.querySelector('#host');\
             h.innerHTML = '<div>x</div><b>y</b>';\
             h.firstChild.outerHTML = ''; h.innerHTML"
        )
        .unwrap(),
        "<b>y</b>"
    );
    // No parent, nowhere to be replaced: the spec's error, thrown — never the
    // silence a getter-only property gave a sloppy script.
    assert_eq!(
        s.eval(
            "var d = document.createElement('div');\
             try { d.outerHTML = '<p></p>'; 'no throw' } catch (e) { e.name }"
        )
        .unwrap(),
        "NoModificationAllowedError"
    );
}

#[test]
fn a_widget_that_swaps_itself_out_reaches_the_arena_from_a_sloppy_script() {
    // The bl-0679 shape: a classic script is sloppy, so this assignment used to
    // vanish without an error and the placeholder stayed on the page.
    let (doc, report) = drive(
        "<body><div id='ph' class='loading'>spinner</div>\
         <script>\
           document.getElementById('ph').outerHTML = \
             '<section id=\"real\" data-src=\"x\">loaded</section>';\
         </script></body>",
    );
    assert!(doc.find_by_tag("div").is_empty(), "placeholder still there");
    assert_eq!(attr(&doc, "section", "data-src").as_deref(), Some("x"));
    assert!(!doc.text_content(0).contains("spinner"));
    assert_eq!((report.scripts, report.errors), (1, 0));
}

#[test]
fn put_forwards_style_and_class_list_write_through_to_the_attribute() {
    let s = sess("<html><body><div id='host'></div></body></html>");
    // `el.style = v` IS `el.style.cssText = v`, `el.classList = v` IS
    // `el.classList.value = v` — the write lands on the attribute, and the list
    // reading it back is live over that same attribute.
    assert_eq!(
        s.eval(
            "var d = document.querySelector('#host');\
             d.style = 'color:red'; d.classList = 'a b';\
             [d.getAttribute('style'), d.style.cssText, d.getAttribute('class'),\
              d.className, d.classList.length, d.classList.contains('b')].join('|')"
        )
        .unwrap(),
        "color:red|color:red|a b|a b|2|true"
    );
    // [LegacyNullToEmptyString] on cssText: `el.style = null` clears the
    // declaration, it does not write the string "null".
    assert_eq!(
        s.eval(
            "var d = document.querySelector('#host');\
             d.style = 'color:red'; d.style = null; d.style.cssText"
        )
        .unwrap(),
        ""
    );
    // relList is the third member of the family, and where a browser has no
    // such accessor at all (a plain div) there is nothing to forward to.
    assert_eq!(
        s.eval(
            "var l = document.createElement('link'); l.relList = 'stylesheet';\
             var d = document.createElement('div'); d.relList = 'x';\
             l.getAttribute('rel') + '|' + String(d.relList)"
        )
        .unwrap(),
        "stylesheet|undefined"
    );
}

#[test]
fn a_style_assignment_from_a_sloppy_script_reaches_the_cascade() {
    // The whole point: the write is not merely readable back in JS, it is the
    // attribute — so the §8 style cache is invalidated and getComputedStyle,
    // which the views read after settle, answers from the new declaration.
    let (doc, report) = drive(
        "<body><div>x</div>\
         <script>\
           var d = document.querySelector('div');\
           d.style = 'display:none'; d.classList = 'hidden';\
           d.setAttribute('data-seen', getComputedStyle(d).display);\
         </script></body>",
    );
    assert_eq!(attr(&doc, "div", "style").as_deref(), Some("display:none"));
    assert_eq!(attr(&doc, "div", "class").as_deref(), Some("hidden"));
    assert_eq!(attr(&doc, "div", "data-seen").as_deref(), Some("none"));
    assert_eq!((report.scripts, report.errors), (1, 0));
}

#[test]
fn document_body_is_replaced_by_assignment_and_rejects_a_non_body() {
    let s = sess("<html><body><p>old</p></body></html>");
    // Replacement, through the DOM's own `replaceChild`: one body before, one
    // after, and it is the new one.
    assert_eq!(
        s.eval(
            "var b = document.createElement('body'); b.innerHTML = '<p>new</p>';\
             document.body = b;\
             document.body.innerHTML + '|' + document.querySelectorAll('body').length"
        )
        .unwrap(),
        "<p>new</p>|1"
    );
    // Assigning the body it already has is the general path with its own
    // successor as the reference sibling — not a special case, and not a
    // document that lost its body.
    assert_eq!(
        s.eval(
            "document.body = document.body;\
             document.body.innerHTML + '|' + document.querySelectorAll('body').length"
        )
        .unwrap(),
        "<p>new</p>|1"
    );
    // Anything that is not a body or a frameset throws, as in a browser —
    // getter-only had swallowed both the swap and the type error.
    assert_eq!(
        s.eval(
            "try { document.body = document.createElement('div'); 'no throw' }\
             catch (e) { e.name }"
        )
        .unwrap(),
        "HierarchyRequestError"
    );
    assert_eq!(
        s.eval("try { document.body = null; 'no throw' } catch (e) { e.name }")
            .unwrap(),
        "HierarchyRequestError"
    );
    // A frameset is the other legal value.
    assert_eq!(
        s.eval("document.body = document.createElement('frameset'); document.body.tagName")
            .unwrap(),
        "FRAMESET"
    );
}

#[test]
fn replacing_a_node_that_is_not_a_child_throws_instead_of_moving_it() {
    // `replaceChild` is the one replacement primitive `document.body`'s setter
    // is defined over. Totality would be silent corruption here — the node
    // would land at the front of a parent that never held the one it replaced —
    // so the spec's NotFoundError is the honest answer.
    let s = sess("<html><body><p>a</p></body></html>");
    assert_eq!(
        s.eval(
            "try { document.body.replaceChild(document.createElement('i'),\
                     document.createElement('u')); 'no throw' } catch (e) { e.name }"
        )
        .unwrap(),
        "NotFoundError"
    );
}
