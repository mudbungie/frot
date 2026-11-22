//! Each refusal, with the words Firefox 153.0esr gives it (identity.md §3.21),
//! and the insertions next to it that are allowed.

use super::super::{Document, NodeId, NodeKind};

/// `<!DOCTYPE html><html>…`: the doctype, the root element, the body.
fn page() -> (Document, NodeId, NodeId, NodeId) {
    let doc = Document::parse("<!DOCTYPE html><html><body><p>t</p></body></html>");
    let doctype = doc.roots()[0];
    let html = doc.roots()[1];
    let body = doc.find_by_tag("body")[0];
    (doc, doctype, html, body)
}

const DOC: NodeId = Document::ROOT;

#[test]
fn only_a_document_or_an_element_takes_children() {
    let (mut doc, doctype, _, body) = page();
    let el = doc.create_element("b");
    let text = doc.create_text("t");
    let comment = doc.create_comment("c");
    for (parent, name) in [
        (text, "Text"),
        (comment, "Comment"),
        (doctype, "DocumentType"),
    ] {
        assert_eq!(
            doc.insert_child(parent, el, None),
            Err(format!("Cannot add children to a {name}"))
        );
    }
    assert_eq!(doc.insert_child(body, el, None), Ok(()));
    assert_eq!(doc.insert_child(el, text, None), Ok(()));
    assert_eq!(doc.insert_child(body, comment, None), Ok(()));
}

#[test]
fn a_node_never_goes_under_itself_or_its_descendant() {
    let (mut doc, _, html, body) = page();
    let ancestor = Err("The new child is an ancestor of the parent".to_string());
    assert_eq!(doc.insert_child(body, body, None), ancestor);
    assert_eq!(doc.insert_child(body, html, None), ancestor);
    // The document is every connected node's ancestor, so that is the reason
    // it gets there; under a detached element it is refused for what it is.
    assert_eq!(doc.insert_child(body, DOC, None), ancestor);
    assert_eq!(doc.insert_child(DOC, DOC, None), ancestor);
    let loose = doc.create_element("div");
    assert_eq!(
        doc.insert_child(loose, DOC, None),
        Err("May not add a Document as a child".into())
    );
    // Refused before anything moved: the tree is as it was.
    assert_eq!(doc.node(html).parent, Some(DOC));
    assert_eq!(doc.node(body).parent, Some(html));
    assert_eq!(doc.generation(), 1);
}

#[test]
fn a_doctype_and_text_have_their_places() {
    let (mut doc, doctype, _, body) = page();
    assert_eq!(
        doc.insert_child(body, doctype, None),
        Err("Cannot insert a DocumentType as a child of an Element".into())
    );
    let text = doc.create_text("t");
    assert_eq!(
        doc.insert_child(DOC, text, None),
        Err("Cannot insert a Text as a child of a Document".into())
    );
    // A comment may sit anywhere among a document's children.
    let comment = doc.create_comment("c");
    assert_eq!(doc.insert_child(DOC, comment, Some(doctype)), Ok(()));
    assert_eq!(doc.roots()[0], comment);
}

#[test]
fn a_document_holds_one_element_and_one_doctype_doctype_first() {
    let (mut doc, doctype, html, _) = page();
    let one_el = Err("Cannot have more than one Element child of a Document".to_string());
    let one_dt = Err("Cannot have more than one DocumentType child of a Document".to_string());
    let div = doc.create_element("div");
    assert_eq!(doc.insert_child(DOC, div, None), one_el);
    // A node already there counts against itself, as in the binary.
    assert_eq!(doc.insert_child(DOC, html, None), one_el);
    assert_eq!(doc.insert_child(DOC, html, Some(html)), one_el);
    assert_eq!(doc.insert_child(DOC, doctype, Some(html)), one_dt);

    doc.detach(html);
    assert_eq!(
        doc.insert_child(DOC, html, Some(doctype)),
        Err("Cannot insert a root element before the doctype".into())
    );
    assert_eq!(doc.insert_child(DOC, html, None), Ok(()));
    doc.detach(doctype);
    assert_eq!(
        doc.insert_child(DOC, doctype, None),
        Err("Cannot have a DocumentType node after the root element".into())
    );
    // A comment between them is neither.
    let comment = doc.create_comment("c");
    assert_eq!(doc.insert_child(DOC, comment, Some(html)), Ok(()));
    assert_eq!(doc.insert_child(DOC, doctype, Some(html)), Ok(()));
    assert_eq!(doc.roots(), [comment, doctype, html]);
}

/// `replaceChild`'s question: the node being replaced is set aside, and its
/// place is the insertion point.
#[test]
fn a_replacement_is_judged_without_the_node_it_replaces() {
    let (mut doc, doctype, html, body) = page();
    let other = doc.create_element("html");
    assert_eq!(doc.refusal(DOC, other, None, Some(html)), None);
    assert_eq!(
        doc.refusal(DOC, other, None, Some(doctype)).as_deref(),
        Some("Cannot have more than one Element child of a Document")
    );
    let text = doc.create_text("t");
    assert_eq!(
        doc.refusal(DOC, text, None, Some(html)).as_deref(),
        Some("Cannot insert a Text as a child of a Document")
    );
    assert_eq!(
        doc.refusal(body, html, None, Some(doc.node(body).children[0]))
            .as_deref(),
        Some("The new child is an ancestor of the parent")
    );
    // An element replacing the root's only content: nothing to count.
    assert_eq!(
        doc.refusal(body, text, None, Some(doc.node(body).children[0])),
        None
    );
    assert!(matches!(doc.node(DOC).kind, NodeKind::Document));
}
