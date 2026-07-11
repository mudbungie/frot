//! Prelude facade + script-queue tests: the Node/Element/Document API, console,
//! geometry, and the §4 script discovery/order over the syscall table.

use super::super::{classify, is_js_type, Script};
use super::{drive, sess};
use crate::dom::Document;

#[test]
fn document_facade_queries_and_reads_the_tree() {
    let s = sess("<html><body><p id='x' class='lead'>hi</p></body></html>");
    assert_eq!(s.eval("document.querySelector('p').textContent").unwrap(), "hi");
    assert_eq!(s.eval("document.querySelector('#x').tagName").unwrap(), "P");
    assert_eq!(s.eval("document.querySelector('.lead').className").unwrap(), "lead");
    assert_eq!(s.eval("document.body.tagName").unwrap(), "BODY");
    assert_eq!(s.eval("document.documentElement.tagName").unwrap(), "HTML");
    assert_eq!(s.eval("document.querySelectorAll('p').length").unwrap(), "1");
    assert_eq!(s.eval("document.getElementById('x').id").unwrap(), "x");
    assert_eq!(s.eval("document.querySelector('#x').hasAttribute('class')").unwrap(), "true");
}

#[test]
fn innerhtml_and_construction_mutate_the_shared_arena() {
    let s = sess("<html><body><div id='host'></div></body></html>");
    s.eval("document.querySelector('#host').innerHTML = '<p>one</p><p>two</p>'").unwrap();
    assert_eq!(s.eval("document.querySelectorAll('#host p').length").unwrap(), "2");
    // Build a node the other way: createElement + textContent + appendChild.
    s.eval("var el = document.createElement('span'); el.textContent = 'z'; document.body.appendChild(el);")
        .unwrap();
    // The mutations land on the one document the pipeline will consume.
    assert_eq!(s.document().find_by_tag("span").len(), 1);
    assert_eq!(s.document().find_by_tag("p").len(), 2);
    assert!(s.eval("document.body.textContent").unwrap().contains("one"));
    // innerHTML round-trips through the arena serializer.
    assert_eq!(s.eval("document.querySelector('#host').innerHTML").unwrap(), "<p>one</p><p>two</p>");
}

#[test]
fn geometry_facade_exposes_rects_offsets_and_computed_style() {
    let s = sess("<html><body><div id='a'>hi</div><div id='b' style='display:none'>x</div></body></html>");
    // getBoundingClientRect shapes the [x,y,w,h] syscall into a DOMRect.
    assert_eq!(s.eval("document.querySelector('#a').getBoundingClientRect().width").unwrap(), "1280");
    assert_eq!(s.eval("document.querySelector('#a').getBoundingClientRect().bottom").unwrap(), "20");
    // offset* read the same box; a display:none element is all-zero.
    assert_eq!(s.eval("document.querySelector('#a').offsetHeight").unwrap(), "20");
    assert_eq!(s.eval("document.querySelector('#b').offsetWidth").unwrap(), "0");
    // getComputedStyle exposes the §8 subset (camelCase + getPropertyValue).
    assert_eq!(s.eval("getComputedStyle(document.querySelector('#b')).display").unwrap(), "none");
    assert_eq!(
        s.eval("getComputedStyle(document.querySelector('#a')).getPropertyValue('display')").unwrap(),
        "block"
    );
}

#[test]
fn console_routes_every_level_through_the_prelude() {
    let s = sess("<html></html>");
    s.eval("console.log('a', 1, {k:2}); console.error('boom');").unwrap();
    let logs = s.console();
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0].level, "log");
    assert_eq!(logs[0].text, "a 1 {\"k\":2}");
    assert_eq!(logs[1].level, "error");
    assert_eq!(logs[1].text, "boom");
}

#[test]
fn run_executes_inline_scripts_and_reclaims_the_mutated_document() {
    let (doc, report) = drive(
        "<body><div id='h'></div>\
         <script>document.getElementById('h').textContent = 'live';</script></body>",
    );
    assert_eq!(report.scripts, 1);
    assert_eq!(report.errors, 0);
    assert!(report.settled);
    let div = doc.find_by_tag("div")[0];
    assert_eq!(doc.text_content(div), "live");
}

#[test]
fn a_throwing_script_still_counts_as_executed_and_errored() {
    let (_doc, report) = drive("<body><script>throw new Error('x')</script></body>");
    assert_eq!((report.scripts, report.errors, report.settled), (1, 1, true));
}

#[test]
fn external_scripts_are_skipped_and_counted_until_subfetch() {
    let (_doc, report) = drive("<body><script src='app.js'></script></body>");
    assert_eq!((report.scripts, report.errors, report.settled), (0, 1, true));
}

#[test]
fn non_js_type_and_nomodule_scripts_are_skipped_uncounted() {
    // If either ran it would throw (bad JSON / undefined `should`); errors == 0
    // proves neither executed.
    let (_doc, report) = drive(
        "<body>\
         <script type='application/json'>{not json}</script>\
         <script nomodule>should.not.run()</script></body>",
    );
    assert_eq!((report.scripts, report.errors, report.settled), (0, 0, true));
}

#[test]
fn script_inserted_scripts_join_the_queue() {
    // §4.3: a script that appends another <script> runs the appended one too.
    let (doc, report) = drive(
        "<body><script>\
         var s = document.createElement('script');\
         s.textContent = \"document.body.appendChild(document.createElement('hr'))\";\
         document.body.appendChild(s);</script></body>",
    );
    assert_eq!(report.scripts, 2);
    assert_eq!(doc.find_by_tag("hr").len(), 1);
}

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
