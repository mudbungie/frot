use super::*;
use crate::dom::Document;

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
    let hits = query_all(&doc, None, "p").unwrap();
    assert_eq!(ids_to_tags(&doc, &hits), vec!["p", "p"]);
}

#[test]
fn class_and_descendant_combinator_resolve() {
    let doc = Document::parse("<div><section><p class='x'>a</p></section><p>b</p></div>");
    let hits = query_all(&doc, None, "div p.x").unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(doc.text_content(hits[0]), "a");
}

#[test]
fn selector_list_unions_matches() {
    let doc = Document::parse("<a>1</a><b>2</b><i>3</i>");
    let hits = query_all(&doc, None, "a, i").unwrap();
    assert_eq!(ids_to_tags(&doc, &hits), vec!["a", "i"]);
}

#[test]
fn element_scope_returns_strict_descendants_only() {
    let doc = Document::parse("<div id='r'><span>a</span><div><span>b</span></div></div>");
    let r = doc.find_by_tag("div")[0]; // outer div#r
    let hits = query_all(&doc, Some(r), "span").unwrap();
    // Both spans are descendants of r; r itself (a div) is never returned.
    let texts: Vec<_> = hits.iter().map(|&i| doc.text_content(i)).collect();
    assert_eq!(texts, vec!["a", "b"]);
    // Querying `div` under r finds the inner div but not r itself.
    let divs = query_all(&doc, Some(r), "div").unwrap();
    assert_eq!(divs.len(), 1);
}

#[test]
fn element_scope_keeps_ancestor_context_above_root() {
    // `body p` must still match a p under root even though `body` is above root.
    let doc = Document::parse("<body><main id='m'><p>hit</p></main></body>");
    let m = doc.find_by_tag("main")[0];
    let hits = query_all(&doc, Some(m), "body p").unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(doc.text_content(hits[0]), "hit");
}

#[test]
fn unsupported_pseudo_class_is_an_error() {
    let doc = Document::parse("<p>a</p>");
    assert_eq!(query_all(&doc, None, "p:hover"), Err(UnsupportedSelector));
}

#[test]
fn pseudo_element_selector_is_an_error() {
    let doc = Document::parse("<p>a</p>");
    assert_eq!(query_all(&doc, None, "p::before"), Err(UnsupportedSelector));
}

#[test]
fn unparseable_selector_is_an_error() {
    let doc = Document::parse("<p>a</p>");
    assert_eq!(query_all(&doc, None, ""), Err(UnsupportedSelector));
    assert_eq!(query_all(&doc, None, "."), Err(UnsupportedSelector));
}

#[test]
fn text_nodes_between_elements_are_skipped_not_matched() {
    // Exercises the non-element recursion arm: text siblings carry no children
    // but the walk must still descend structural wrappers around them.
    let doc = Document::parse("<ul>x<li>one</li>y<li>two</li></ul>");
    let hits = query_all(&doc, None, "li").unwrap();
    assert_eq!(ids_to_tags(&doc, &hits), vec!["li", "li"]);
}
