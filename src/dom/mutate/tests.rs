use crate::dom::*;

fn el_name(doc: &Document, id: NodeId) -> String {
    match &doc.node(id).kind {
        NodeKind::Element(e) => e.name.clone(),
        other => panic!("expected element, got {other:?}"),
    }
}

fn attrs(doc: &Document, id: NodeId) -> Vec<Attr> {
    match &doc.node(id).kind {
        NodeKind::Element(e) => e.attrs.clone(),
        other => panic!("expected element, got {other:?}"),
    }
}

#[test]
fn parsed_document_starts_at_generation_zero() {
    let doc = Document::parse("<p>x</p>");
    assert_eq!(doc.generation(), 0);
}

#[test]
fn create_element_appends_a_detached_lowercased_node() {
    let mut doc = Document::parse("<body></body>");
    let before = doc.len();
    let id = doc.create_element("DIV");
    assert_eq!(
        id as usize, before,
        "new node lands at the end of the arena"
    );
    assert_eq!(el_name(&doc, id), "div");
    assert_eq!(doc.node(id).parent, None);
    assert!(!doc.roots().contains(&id), "detached: not a root");
    assert_eq!(doc.generation(), 1);
}

#[test]
fn create_text_appends_a_text_node() {
    let mut doc = Document::default();
    let id = doc.create_text("hello");
    assert_eq!(doc.node(id).kind, NodeKind::Text("hello".into()));
    assert_eq!(doc.generation(), 1);
}

#[test]
fn set_attr_adds_then_overwrites() {
    let mut doc = Document::default();
    let id = doc.create_element("a");
    doc.set_attr(id, "HREF", "one"); // None arm: new attribute
    assert_eq!(
        attrs(&doc, id),
        vec![Attr {
            name: "href".into(),
            value: "one".into()
        }]
    );
    doc.set_attr(id, "href", "two"); // Some arm: overwrite in place
    assert_eq!(
        attrs(&doc, id),
        vec![Attr {
            name: "href".into(),
            value: "two".into()
        }]
    );
}

#[test]
fn remove_attr_drops_the_named_attribute() {
    let mut doc = Document::default();
    let id = doc.create_element("a");
    doc.set_attr(id, "href", "x");
    doc.set_attr(id, "rel", "y");
    doc.remove_attr(id, "href");
    doc.remove_attr(id, "absent"); // retain no-op still bumps generation
    assert_eq!(
        attrs(&doc, id),
        vec![Attr {
            name: "rel".into(),
            value: "y".into()
        }]
    );
}

#[test]
fn set_text_replaces_text_node_data() {
    let mut doc = Document::default();
    let id = doc.create_text("old");
    doc.set_text(id, "new");
    assert_eq!(doc.node(id).kind, NodeKind::Text("new".into()));
}

#[test]
fn attr_and_text_ops_are_total_no_ops_on_the_wrong_kind() {
    let mut doc = Document::default();
    let text = doc.create_text("t");
    let elem = doc.create_element("p");
    // Attribute ops on a text node change nothing but still bump generation.
    let g = doc.generation();
    doc.set_attr(text, "id", "x");
    doc.remove_attr(text, "id");
    // set_text on an element node is likewise a no-op.
    doc.set_text(elem, "y");
    assert_eq!(doc.node(text).kind, NodeKind::Text("t".into()));
    assert_eq!(attrs(&doc, elem), vec![]);
    assert_eq!(
        doc.generation(),
        g + 3,
        "no-op ops still count as mutations"
    );
}

#[test]
fn insert_child_links_a_fresh_node_at_index() {
    let mut doc = Document::default();
    let parent = doc.create_element("ul");
    let a = doc.create_element("li");
    let b = doc.create_element("li");
    let c = doc.create_element("li");
    doc.insert_child(parent, a, 0);
    doc.insert_child(parent, b, 1); // append at end
    doc.insert_child(parent, c, 1); // splice into the middle
    assert_eq!(doc.node(parent).children, vec![a, c, b]);
    assert_eq!(doc.node(a).parent, Some(parent));
}

#[test]
fn insert_child_moves_a_node_that_already_has_a_parent() {
    let mut doc = Document::default();
    let p1 = doc.create_element("div");
    let p2 = doc.create_element("div");
    let kid = doc.create_element("span");
    doc.insert_child(p1, kid, 0);
    doc.insert_child(p2, kid, 0); // unlink from p1, relink under p2
    assert!(doc.node(p1).children.is_empty());
    assert_eq!(doc.node(p2).children, vec![kid]);
    assert_eq!(doc.node(kid).parent, Some(p2));
}

#[test]
fn detach_unlinks_a_child_but_keeps_the_arena_entry() {
    let mut doc = Document::default();
    let parent = doc.create_element("div");
    let kid = doc.create_element("span");
    doc.insert_child(parent, kid, 0);
    let len_before = doc.len();
    doc.detach(kid);
    assert!(doc.node(parent).children.is_empty());
    assert_eq!(doc.node(kid).parent, None);
    assert_eq!(doc.len(), len_before, "entry stays in the arena");
    assert_eq!(el_name(&doc, kid), "span", "NodeId still valid");
}

#[test]
fn detach_removes_a_root_from_the_root_list() {
    let mut doc = Document::parse("<p>x</p>");
    let root = doc.roots()[0];
    let roots_before = doc.roots().len();
    doc.detach(root);
    assert_eq!(doc.roots().len(), roots_before - 1);
    assert!(!doc.roots().contains(&root));
}

#[test]
fn detach_of_a_parentless_orphan_is_a_no_op_relink() {
    let mut doc = Document::default();
    let orphan = doc.create_element("div");
    doc.detach(orphan); // parent None, not a root: retain touches nothing
    assert_eq!(doc.node(orphan).parent, None);
    assert!(!doc.roots().contains(&orphan));
}

#[test]
fn parse_fragment_absorbs_detached_top_level_nodes() {
    let mut doc = Document::parse("<body></body>");
    let ids = doc.parse_fragment("<p>hi</p><span>yo</span>");
    assert_eq!(ids.len(), 2);
    for &id in &ids {
        assert_eq!(
            doc.node(id).parent,
            None,
            "top-level fragment nodes are detached"
        );
        assert!(!doc.roots().contains(&id));
    }
    assert_eq!(el_name(&doc, ids[0]), "p");
    assert_eq!(el_name(&doc, ids[1]), "span");
}

#[test]
fn absorbed_fragment_becomes_reachable_once_spliced_in() {
    let mut doc = Document::parse("<body></body>");
    let body = doc.find_by_tag("body")[0];
    let ids = doc.parse_fragment("<p>deep <b>text</b></p>");
    doc.insert_child(body, ids[0], 0);
    assert_eq!(doc.find_by_tag("p").len(), 1);
    assert_eq!(doc.text_content(body), "deep text");
}

#[test]
fn every_mutation_bumps_the_generation_counter() {
    let mut doc = Document::default();
    let mut expect = 0;
    let mut step = |g: u64| {
        expect += 1;
        assert_eq!(g, expect);
    };
    let a = doc.create_element("a");
    step(doc.generation());
    let t = doc.create_text("x");
    step(doc.generation());
    doc.set_attr(a, "id", "k");
    step(doc.generation());
    doc.remove_attr(a, "id");
    step(doc.generation());
    doc.set_text(t, "y");
    step(doc.generation());
    doc.insert_child(a, t, 0);
    step(doc.generation());
    doc.detach(t);
    step(doc.generation());
    doc.parse_fragment("<p>p</p>");
    step(doc.generation());
}
