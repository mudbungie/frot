use super::*;

fn find(doc: &Document, id: &str) -> NodeId {
    (0..doc.len() as NodeId)
        .find(|&n| doc.element(n).is_some_and(|e| e.attr("id") == Some(id)))
        .unwrap()
}

fn names(doc: &Document, ids: &[NodeId]) -> Vec<String> {
    ids.iter()
        .map(|&n| match &doc.node(n).kind {
            NodeKind::Element(el) => el.attr("id").unwrap_or(&el.name).to_string(),
            _ => doc.text_content(n),
        })
        .collect()
}

const PAGE: &str = "<div id=h><template shadowrootmode=open><b id=sb>s</b>\
    <slot id=sa name=a>fa</slot><slot id=sd><i id=fb>fallback</i></slot></template>\
    <em id=e slot=a>e</em><u id=u slot=zz>u</u></div><p id=p>plain</p>";

#[test]
fn the_flat_tree_composes_shadow_trees_and_slots() {
    let doc = Document::parse(PAGE);
    let by = |id| find(&doc, id);
    let kids = |id| names(&doc, &doc.flat_children(by(id)));
    assert_eq!(kids("h"), ["sb", "sa", "sd"]);
    assert_eq!(kids("sa"), ["e"]);
    assert_eq!(kids("sd"), ["fb"]); // nothing assigned: its fallback
    assert_eq!(kids("p"), ["plain"]);
    assert_eq!(doc.flat_parent(by("sb")), Some(by("h")));
    assert_eq!(doc.flat_parent(by("e")), Some(by("sa")));
    assert_eq!(doc.flat_parent(by("u")), None); // no slot takes it
    assert_eq!(doc.flat_parent(by("fb")), Some(by("sd")));
    let fa = doc.node(by("sa")).children[0];
    assert_eq!(doc.flat_parent(fa), None); // fallback of a slot that has nodes
    assert_eq!(doc.flat_parent(by("p")), doc.node(by("p")).parent);
    assert_eq!(doc.flat_parent(Document::ROOT), None);
}

#[test]
fn walk_flat_visits_what_renders_in_flat_order() {
    let doc = Document::parse(PAGE);
    let mut seen = Vec::new();
    doc.walk_flat(None, &mut |ev, entry| {
        if let (WalkEvent::Enter(_), NodeKind::Element(el)) = (ev, &entry.kind) {
            seen.push(el.attr("id").unwrap_or(&el.name).to_string());
        }
    });
    assert_eq!(
        seen,
        ["html", "head", "body", "h", "sb", "sa", "e", "sd", "fb", "p"]
    );
    let mut from = Vec::new();
    doc.walk_flat(Some(find(&doc, "sd")), &mut |ev, _| {
        if let WalkEvent::Exit(n) = ev {
            from.push(n);
        }
    });
    assert_eq!(from.len(), 3);
}
