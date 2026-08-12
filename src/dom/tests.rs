use super::*;

fn first_element(doc: &Document, name: &str) -> NodeId {
    *doc.find_by_tag(name).first().unwrap()
}

fn element_of(doc: &Document, id: NodeId) -> Element {
    let entry = doc.node(id);
    assert!(matches!(entry.kind, NodeKind::Element(_)));
    if let NodeKind::Element(el) = &entry.kind {
        el.clone()
    } else {
        unreachable!()
    }
}

#[test]
fn parse_empty_string_yields_implicit_html_skeleton() {
    let doc = Document::parse("");
    assert!(!doc.is_empty(), "parser injects <html>/<head>/<body>");
    assert!(!doc.find_by_tag("html").is_empty());
    assert!(!doc.find_by_tag("head").is_empty());
    assert!(!doc.find_by_tag("body").is_empty());
}

#[test]
fn parse_doctype_node_is_preserved() {
    let doc = Document::parse("<!doctype html><html><body></body></html>");
    let mut saw_doctype = false;
    doc.walk(None, &mut |ev, e| {
        if let WalkEvent::Enter(_) = ev {
            if matches!(e.kind, NodeKind::Doctype) {
                saw_doctype = true;
            }
        }
    });
    assert!(saw_doctype, "doctype absorbed as a root or sibling");
}

#[test]
fn elements_have_lowercase_names_and_attrs() {
    let doc = Document::parse("<HTML><BODY><A HREF='X'>k</A></BODY></HTML>");
    let a = first_element(&doc, "a");
    let el = element_of(&doc, a);
    assert_eq!(el.name, "a");
    assert_eq!(el.attr("href"), Some("X"));
    assert_eq!(el.attr("missing"), None);
}

#[test]
fn text_nodes_carry_content() {
    let doc = Document::parse("<p>hi <b>there</b></p>");
    let p = first_element(&doc, "p");
    assert_eq!(doc.text_content(p), "hi there");
}

#[test]
fn a_comment_asked_about_itself_is_its_own_data() {
    // DOM §4.4: `textContent` of a Comment is its data — the same fact `data`
    // and `nodeValue` expose, and the one the markup serializer needs to write
    // `<!--x-->` back out (bl-273b). An element's textContent still skips the
    // comments among its descendants.
    let doc = Document::parse("<p>hi<!--anchor-->there</p>");
    let p = first_element(&doc, "p");
    let comment = doc.node(p).children[1];
    assert!(matches!(doc.node(comment).kind, NodeKind::Comment(_)));
    assert_eq!(doc.text_content(comment), "anchor");
    assert_eq!(doc.text_content(p), "hithere");
}

#[test]
fn parent_links_are_set() {
    let doc = Document::parse("<div><span>x</span></div>");
    let span = first_element(&doc, "span");
    let parent = doc.node(span).parent.expect("span has parent");
    let el = element_of(&doc, parent);
    assert_eq!(el.name, "div");
}

#[test]
fn walk_yields_enter_and_exit_in_source_order() {
    let doc = Document::parse("<p><a>1</a><b>2</b></p>");
    let p = first_element(&doc, "p");
    let mut log: Vec<String> = Vec::new();
    doc.walk(Some(p), &mut |ev, e| match (ev, &e.kind) {
        (WalkEvent::Enter(_), NodeKind::Element(el)) => log.push(format!("+{}", el.name)),
        (WalkEvent::Exit(_), NodeKind::Element(el)) => log.push(format!("-{}", el.name)),
        (WalkEvent::Enter(_), NodeKind::Text(t)) => log.push(format!("t:{}", t)),
        _ => {}
    });
    assert_eq!(log, vec!["+p", "+a", "t:1", "-a", "+b", "t:2", "-b", "-p"]);
}

#[test]
fn find_by_tag_finds_all_occurrences() {
    let doc = Document::parse("<p>a</p><p>b</p><p>c</p>");
    let ps = doc.find_by_tag("p");
    assert_eq!(ps.len(), 3);
}

#[test]
fn comments_are_absorbed_as_comment_nodes() {
    let doc = Document::parse("<html><body><!-- hi --></body></html>");
    let mut saw = false;
    doc.walk(None, &mut |ev, e| {
        if let WalkEvent::Enter(_) = ev {
            if let NodeKind::Comment(c) = &e.kind {
                if c.contains("hi") {
                    saw = true;
                }
            }
        }
    });
    assert!(saw);
}

#[test]
fn document_len_grows_with_content() {
    let doc1 = Document::parse("<html><body></body></html>");
    let doc2 = Document::parse("<html><body><p>x</p><p>y</p></body></html>");
    assert!(doc2.len() > doc1.len());
}

#[test]
fn walk_without_start_visits_every_root() {
    let doc = Document::parse("<!doctype html><html><body></body></html>");
    let mut roots_visited = 0;
    doc.walk(None, &mut |ev, _| {
        if matches!(ev, WalkEvent::Enter(_)) {
            roots_visited += 1;
        }
    });
    assert!(roots_visited >= doc.roots().len());
}

#[test]
fn nodekind_equality_works() {
    let a = NodeKind::Text("x".into());
    let b = NodeKind::Text("x".into());
    assert_eq!(a, b);
    assert_eq!(NodeKind::Doctype, NodeKind::Doctype);
    let e1 = NodeKind::Element(Element {
        name: "p".into(),
        attrs: vec![Attr {
            name: "id".into(),
            value: "x".into(),
        }],
    });
    let e2 = e1.clone();
    assert_eq!(e1, e2);
}

#[test]
fn empty_document_walk_is_noop() {
    let doc = Document::default();
    assert!(doc.is_empty());
    assert!(doc.roots().is_empty());
    let mut hits = 0;
    doc.walk(None, &mut |_, _| hits += 1);
    assert_eq!(hits, 0);
}

#[test]
fn document_default_is_empty() {
    let doc: Document = Default::default();
    assert_eq!(doc.len(), 0);
}

#[test]
fn node_data_to_kind_collapses_document_and_pi_to_empty_comment() {
    use html5ever::tendril::StrTendril;
    use markup5ever_rcdom::NodeData;

    let doc = node_data_to_kind(&NodeData::Document);
    assert_eq!(doc, NodeKind::Comment(String::new()));
    let pi = node_data_to_kind(&NodeData::ProcessingInstruction {
        target: StrTendril::from_slice("xml"),
        contents: StrTendril::from_slice("foo"),
    });
    assert_eq!(pi, NodeKind::Comment(String::new()));
}

#[test]
fn parse_preserves_attribute_order() {
    let doc = Document::parse("<a id='1' class='c' href='/' rel='ext'>x</a>");
    let a = first_element(&doc, "a");
    let el = element_of(&doc, a);
    let names: Vec<&str> = el.attrs.iter().map(|x| x.name.as_str()).collect();
    assert_eq!(names, ["id", "class", "href", "rel"]);
}
