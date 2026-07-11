//! End-to-end checks that the JS prelude facade (Node/Element/Document +
//! console) is wired over the syscalls. Prelude JS lines are not `llvm-cov`
//! visible; these guard the facade until the golden fixture suite (4.9) owns it.

use super::Session;
use crate::dom::Document;

fn sess(html: &str) -> Session {
    Session::new(Document::parse(html))
}

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
