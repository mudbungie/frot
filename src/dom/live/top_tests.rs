//! Light dismiss and modal inertness (`bl-2dcd`), as Chrome 158 has them.

use crate::dom::Document;

const PAGE: &str =
    "<div popover id=a1></div><div popover=AUTO id=a2><div popover id=a3></div></div>\
    <div popover=hint id=h1><div popover id=a4></div></div><div popover=hint id=h2></div>\
    <div popover=manual id=m1></div><div popover=bogus id=b1></div><p id=p></p>\
    <main id=main><dialog id=d><button id=db></button></dialog></main>";

fn open(doc: &Document) -> String {
    let ids = ["a1", "a2", "a3", "a4", "h1", "h2", "m1", "b1"];
    let on = |i: &str| doc.popover_open(doc.element_by_id(i).unwrap());
    ids.into_iter()
        .filter(|i| on(i))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn showing_a_popover_dismisses_the_ones_it_light_dismisses() {
    let mut doc = Document::parse(PAGE);
    let id = |doc: &Document, i| doc.element_by_id(i).unwrap();
    for i in ["m1", "b1", "a1", "a2", "a3"] {
        let n = id(&doc, i);
        doc.show_popover(n);
    }
    assert_eq!(open(&doc), "a2 a3 m1 b1");
    let (h1, h2, a4) = (id(&doc, "h1"), id(&doc, "h2"), id(&doc, "a4"));
    doc.show_popover(h1);
    assert_eq!(open(&doc), "a2 a3 h1 m1 b1");
    doc.show_popover(h2);
    assert_eq!(open(&doc), "a2 a3 h2 m1 b1");
    doc.show_popover(h1);
    doc.show_popover(a4);
    assert_eq!(open(&doc), "a4 h1 m1 b1");
    let d = id(&doc, "d");
    doc.show_modal(d);
    assert_eq!(open(&doc), "m1 b1");
}

#[test]
fn a_modal_dialog_makes_the_rest_of_the_page_inert() {
    let mut doc = Document::parse(PAGE);
    let id = |doc: &Document, i| doc.element_by_id(i).unwrap();
    let (d, db, p, main) = (
        id(&doc, "d"),
        id(&doc, "db"),
        id(&doc, "p"),
        id(&doc, "main"),
    );
    assert!(!doc.blocked(p) && doc.top_modal().is_none());
    doc.show_modal(d);
    assert_eq!(doc.top_modal(), Some(d));
    assert!(doc.blocked(p) && doc.blocked(main) && !doc.blocked(db) && !doc.blocked(d));
    assert!(doc.inside(db, main) && !doc.inside(main, db));
    let tree = crate::ax::tree::ax_tree(&doc, None);
    assert_eq!(
        tree.to_string(),
        r#"[{"role":"dialog","name":null,"children":[{"role":"button","name":null,"children":[]}]}]"#
    );
    doc.set_modal(d, false);
    assert!(!doc.blocked(p));
}
