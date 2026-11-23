use crate::dom::{Document, NodeId};

/// The target of `html` at a URL ending in `fragment`, as the `id` (or, for
/// an anchor, the `name`) of the element it names.
fn target(html: &str, fragment: &str) -> Option<String> {
    let mut doc = Document::parse(html);
    doc.set_url(&format!("https://example.com/p.html{fragment}"));
    let el = doc
        .target()
        .map(|id: NodeId| doc.element(id).unwrap().clone());
    el.map(|el| format!("{}:{}", el.name, el.attr("id").or(el.attr("name")).unwrap()))
}

const PAGE: &str = "<a name=n>a</a><p id=n>p</p><div id=t>d</div><a name=only>o</a>\
    <span id=%41>pct</span><span id=A>A</span><span id=é>e</span>";

#[test]
fn an_id_beats_an_earlier_anchor_name() {
    assert_eq!(target(PAGE, "#n").as_deref(), Some("p:n"));
    assert_eq!(target(PAGE, "#t").as_deref(), Some("div:t"));
    assert_eq!(target(PAGE, "#only").as_deref(), Some("a:only"));
}

#[test]
fn the_fragment_as_written_is_tried_before_it_is_decoded() {
    assert_eq!(target(PAGE, "#%41").as_deref(), Some("span:%41"));
    assert_eq!(target(PAGE, "#%C3%A9").as_deref(), Some("span:é"));
    assert_eq!(target("<p id=A>", "#%41").as_deref(), Some("p:A"));
}

#[test]
fn no_fragment_an_empty_one_or_an_unknown_name_indicates_nothing() {
    for fragment in ["", "#", "#top", "#missing"] {
        assert_eq!(target(PAGE, fragment), None, "{fragment:?}");
    }
    let mut doc = Document::parse("<p id=t>");
    doc.set_url("not a url#t");
    assert_eq!(doc.target(), None);
}

#[test]
fn a_parsed_document_has_no_url() {
    assert_eq!(Document::parse("<p id=t>").target(), None);
}
