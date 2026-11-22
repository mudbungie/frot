use super::*;
use crate::dom::{Document, NodeKind};

fn ids_to_tags(doc: &Document, ids: &[NodeId]) -> Vec<String> {
    ids.iter()
        .map(|&i| match &doc.node(i).kind {
            NodeKind::Element(e) => e.name.clone(),
            other => panic!("expected element, got {other:?}"),
        })
        .collect()
}

#[test]
fn whole_document_query_matches_every_element() {
    let doc = Document::parse("<div><p class='x'>a</p><p>b</p></div>");
    let hits = query_all(&doc, Document::ROOT, "p").unwrap();
    assert_eq!(ids_to_tags(&doc, &hits), vec!["p", "p"]);
}

#[test]
fn class_and_descendant_combinator_resolve() {
    let doc = Document::parse("<div><section><p class='x'>a</p></section><p>b</p></div>");
    let hits = query_all(&doc, Document::ROOT, "div p.x").unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(doc.text_content(hits[0]), "a");
}

#[test]
fn selector_list_unions_matches() {
    let doc = Document::parse("<a>1</a><b>2</b><i>3</i>");
    let hits = query_all(&doc, Document::ROOT, "a, i").unwrap();
    assert_eq!(ids_to_tags(&doc, &hits), vec!["a", "i"]);
}

#[test]
fn element_scope_returns_strict_descendants_only() {
    let doc = Document::parse("<div id='r'><span>a</span><div><span>b</span></div></div>");
    let r = doc.find_by_tag("div")[0]; // outer div#r
    let hits = query_all(&doc, r, "span").unwrap();
    // Both spans are descendants of r; r itself (a div) is never returned.
    let texts: Vec<_> = hits.iter().map(|&i| doc.text_content(i)).collect();
    assert_eq!(texts, vec!["a", "b"]);
    // Querying `div` under r finds the inner div but not r itself.
    let divs = query_all(&doc, r, "div").unwrap();
    assert_eq!(divs.len(), 1);
}

#[test]
fn element_scope_keeps_ancestor_context_above_root() {
    // `body p` must still match a p under root even though `body` is above root.
    let doc = Document::parse("<body><main id='m'><p>hit</p></main></body>");
    let m = doc.find_by_tag("main")[0];
    let hits = query_all(&doc, m, "body p").unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(doc.text_content(hits[0]), "hit");
}

/// A query inside a subtree that is in no document resolves against that
/// subtree's own top, not against the document it is not part of.
#[test]
fn a_detached_subtree_is_queried_from_its_own_top() {
    let mut doc = Document::parse("<p>in</p>");
    let top = doc.create_element("div");
    let frag = doc.parse_fragment("<p><b>out</b></p>");
    let ids = doc.node(frag).children.clone();
    doc.insert_child(top, frag, None).unwrap();
    assert_eq!(query_all(&doc, top, "p").unwrap(), [ids[0]]);
    assert_eq!(query_all(&doc, top, "div b").unwrap().len(), 1);
    assert_eq!(query_all(&doc, ids[0], "div > p > b").unwrap().len(), 1);
    assert!(query_all(&doc, Document::ROOT, "b").unwrap().is_empty());
}

#[test]
fn unsupported_pseudo_class_is_an_error() {
    let doc = Document::parse("<p>a</p>");
    assert_eq!(
        query_all(&doc, Document::ROOT, "p:hover"),
        Err(UnsupportedSelector)
    );
}

#[test]
fn pseudo_element_selector_is_an_error() {
    let doc = Document::parse("<p>a</p>");
    assert_eq!(
        query_all(&doc, Document::ROOT, "p::before"),
        Err(UnsupportedSelector)
    );
}

#[test]
fn unparseable_selector_is_an_error() {
    let doc = Document::parse("<p>a</p>");
    assert_eq!(
        query_all(&doc, Document::ROOT, ""),
        Err(UnsupportedSelector)
    );
    assert_eq!(
        query_all(&doc, Document::ROOT, "."),
        Err(UnsupportedSelector)
    );
}

#[test]
fn text_nodes_between_elements_are_skipped_not_matched() {
    // Exercises the non-element recursion arm: text siblings carry no children
    // but the walk must still descend structural wrappers around them.
    let doc = Document::parse("<ul>x<li>one</li>y<li>two</li></ul>");
    let hits = query_all(&doc, Document::ROOT, "li").unwrap();
    assert_eq!(ids_to_tags(&doc, &hits), vec!["li", "li"]);
}

#[test]
fn logical_pseudo_classes_are_queryable() {
    let doc = Document::parse("<p class='x'>a</p><p hidden>b</p><i>c</i><b>d</b>");
    let texts = |list: &str| -> Vec<String> {
        let hits = query_all(&doc, Document::ROOT, list).unwrap();
        hits.iter().map(|&i| doc.text_content(i)).collect()
    };
    assert_eq!(texts("p:not([hidden])"), ["a"]);
    assert_eq!(texts(":is(i, b)"), ["c", "d"]);
    assert_eq!(texts(":where(p):not(.x)"), ["b"]);
}

#[test]
fn an_unsupported_selector_inside_a_logical_pseudo_class_is_an_error() {
    let doc = Document::parse("<p>a</p>");
    for bad in [
        "p:is(:hover)",
        ":where(p, :focus)",
        "p:not(:hover)",
        ":is(p::before)",
    ] {
        assert_eq!(
            query_all(&doc, Document::ROOT, bad),
            Err(UnsupportedSelector),
            "{bad}"
        );
    }
}

#[test]
fn sibling_combinators_are_queryable_and_see_past_the_scope_root() {
    let doc = Document::parse("<h2>a</h2><div id=r><p>b</p><i>c</i><p>d</p></div><u>e</u>");
    let texts = |root: NodeId, list: &str| -> Vec<String> {
        let hits = query_all(&doc, root, list).unwrap();
        hits.iter().map(|&i| doc.text_content(i)).collect()
    };
    assert_eq!(texts(Document::ROOT, "i + p"), ["d"]);
    assert_eq!(texts(Document::ROOT, "p ~ *"), ["c", "d"]);
    assert_eq!(texts(Document::ROOT, "div ~ u, h2 + div > i"), ["c", "e"]);
    let r = query_all(&doc, Document::ROOT, "#r").unwrap()[0];
    assert_eq!(texts(r, "h2 + div p"), ["b", "d"]);
    assert_eq!(texts(r, "p ~ p"), ["d"]);
}

#[test]
fn first_and_last_child_are_queryable() {
    let doc = Document::parse("<ul><li>a</li><li>b</li><li>c</li></ul>");
    let texts = |list: &str| -> Vec<String> {
        let hits = query_all(&doc, Document::ROOT, list).unwrap();
        hits.iter().map(|&i| doc.text_content(i)).collect()
    };
    assert_eq!(texts("li:first-child"), ["a"]);
    assert_eq!(texts("li:last-child"), ["c"]);
    assert_eq!(texts("li:not(:first-child, :last-child)"), ["b"]);
    assert_eq!(
        query_all(&doc, Document::ROOT, "li:only-child"),
        Err(UnsupportedSelector)
    );
}
