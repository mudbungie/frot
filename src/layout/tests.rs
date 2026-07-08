use super::*;
use crate::dom::Document;

/// Parse `html` and compute UA-implicit + inline-`style=` display for it — the
/// pair every `compute` call takes.
fn styled(html: &str) -> (Document, Styles) {
    let doc = Document::parse(html);
    let styles = crate::css::compute(&doc);
    (doc, styles)
}

fn id(doc: &Document, tag: &str) -> NodeId {
    doc.find_by_tag(tag).first().copied().unwrap()
}

#[test]
fn viewport_width_is_the_1280_constant() {
    assert_eq!(VIEWPORT_WIDTH, 1280);
}

#[test]
fn rendered_element_gets_a_placeholder_zero_box() {
    let (doc, styles) = styled("<p>hi</p>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    assert_eq!(layout.rect(id(&doc, "p")), Some(Rect::ZERO));
}

#[test]
fn display_none_subtree_generates_no_box() {
    let (doc, styles) = styled("<div style=\"display:none\"><p>x</p></div>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    // The hidden element itself has no box...
    assert_eq!(layout.rect(id(&doc, "div")), None);
    // ...and its subtree is never walked, so the child stays None too.
    assert_eq!(layout.rect(id(&doc, "p")), None);
}

#[test]
fn non_rendered_tags_and_their_subtrees_generate_no_box() {
    let (doc, styles) = styled("<body><script>var x=1;</script><p>ok</p></body>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    // A non-rendered element (script) and the always-present head are skipped.
    assert_eq!(layout.rect(id(&doc, "script")), None);
    assert_eq!(layout.rect(id(&doc, "head")), None);
    // A normal sibling still gets its box.
    assert_eq!(layout.rect(id(&doc, "p")), Some(Rect::ZERO));
}

#[test]
fn non_element_nodes_and_out_of_box_ids_return_none() {
    let (doc, styles) = styled("<p>text</p>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    // The text node inside <p> is walked but is not an element: no box.
    let text_id = doc.node(id(&doc, "p")).children[0];
    assert!(matches!(doc.node(text_id).kind, NodeKind::Text(_)));
    assert_eq!(layout.rect(text_id), None);
}
