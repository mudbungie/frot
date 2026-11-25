use super::*;

#[test]
fn connected_is_rooted_at_the_document() {
    let mut doc = Document::parse("<p id=a>x</p>");
    let p = doc.find_by_tag("p")[0];
    assert!(doc.connected(p));
    assert!(doc.connected(Document::ROOT));
    let made = doc.create_element("i");
    assert!(!doc.connected(made));
    doc.insert_child(p, made, None).unwrap();
    assert!(doc.connected(made));
}

#[test]
fn custom_nodes_are_the_candidates_in_tree_order() {
    let mut doc = Document::parse("<x-a><b is=y></b><i><x-c></x-c></i></x-a><button>z</button>");
    let names = |doc: &Document, ids: Vec<NodeId>| -> Vec<String> {
        ids.iter()
            .map(|&n| doc.element(n).unwrap().name.clone())
            .collect()
    };
    assert_eq!(
        names(&doc, doc.custom_nodes(Document::ROOT)),
        ["x-a", "b", "x-c"]
    );
    let button = doc.find_by_tag("button")[0];
    assert!(doc.custom_nodes(button).is_empty());
    doc.set_custom(button);
    assert!(doc.custom(button));
    assert_eq!(names(&doc, doc.custom_nodes(button)), ["button"]);
}
