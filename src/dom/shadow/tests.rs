use super::*;

fn ids(doc: &Document, ids: &[NodeId]) -> Vec<String> {
    ids.iter()
        .map(|&n| match &doc.node(n).kind {
            NodeKind::Element(el) => el.attr("id").unwrap_or(&el.name).to_string(),
            _ => doc.text_content(n),
        })
        .collect()
}

#[test]
fn the_parser_attaches_a_declarative_root_instead_of_a_child() {
    let doc = Document::parse(
        "<div id=h><template shadowrootmode=Closed shadowrootclonable shadowrootslotassignment=MANUAL>\
         <b>s</b></template>x</div><a id=a><template shadowrootmode=open>no</template></a>\
         <p id=p><template shadowrootmode=open>one</template><template shadowrootmode=open>two</template></p>\
         <p id=q><template shadowrootmode=bogus>b</template><template>t</template></p>",
    );
    let by = |id| doc.element_by_id(id).unwrap();
    let h = by("h");
    let root = doc.shadow_root(h).unwrap();
    assert_eq!(doc.node(root).parent, None);
    assert_eq!(ids(&doc, &doc.node(root).children), ["b"]);
    assert_eq!(ids(&doc, &doc.node(h).children), ["x"]);
    let s = doc.shadow(root).unwrap();
    assert!(s.host == h && s.closed && s.declarative && s.clonable && s.manual);
    assert!(!s.serializable && !s.delegates_focus);
    assert_eq!(doc.shadow(h), None);
    // Not a host, a second root, an invalid mode, a plain template: children.
    assert_eq!(doc.shadow_root(by("a")), None);
    assert_eq!(doc.node(by("a")).children.len(), 1);
    assert_eq!(doc.text_content(doc.shadow_root(by("p")).unwrap()), "one");
    assert_eq!(doc.node(by("p")).children.len(), 1);
    assert_eq!(doc.shadow_root(by("q")), None);
    // A template's contents are a fragment, never a root.
    let t = doc.node(by("q")).children[1];
    assert_eq!(doc.shadow_root(t), None);
    // Fragment parsing makes no root.
    let mut doc = doc;
    let f = doc.parse_fragment(
        "<div><template shadowrootmode=open>z</template></div>",
        "body",
        false,
    );
    let div = doc.node(f).children[0];
    assert_eq!(doc.shadow_root(div), None);
}

#[test]
fn attach_refuses_as_firefox_does_and_reuses_a_declarative_root() {
    let mut doc =
        Document::parse("<div id=d><template shadowrootmode=open><i>x</i></template></div>");
    let d = doc.element_by_id("d").unwrap();
    let init = |host, closed| Shadow {
        host,
        closed,
        ..Shadow::default()
    };
    assert_eq!(
        doc.attach_shadow(init(d, true)),
        Err("Unable to re-attach to existing ShadowDOM")
    );
    let root = doc.shadow_root(d).unwrap();
    assert_eq!(doc.attach_shadow(init(d, false)), Ok(root));
    assert!(doc.node(root).children.is_empty());
    assert!(!doc.shadow(root).unwrap().declarative);
    assert_eq!(
        doc.attach_shadow(init(d, false)),
        Err("Unable to re-attach to existing ShadowDOM")
    );
    let a = doc.create_element("a");
    assert_eq!(
        doc.attach_shadow(init(a, false)),
        Err("Unable to attach ShadowDOM")
    );
    let text = doc.create_text("t");
    assert_eq!(
        doc.attach_shadow(init(text, false)),
        Err("Unable to attach ShadowDOM")
    );
    let x = doc.create_element("x-y");
    let r = doc.attach_shadow(init(x, false)).unwrap();
    assert_eq!(doc.tree_root(r), r);
    assert!(!doc.connected(r));
    doc.insert_child(d, x, None).unwrap();
    assert!(doc.connected(r));
    // A host cannot go inside its own shadow tree.
    assert!(doc.insert_child(r, x, None).is_err());
    assert!(doc.insert_child(r, d, None).is_err());
}

/// The element with id `id` in the tree under `from`.
fn find(doc: &Document, from: NodeId, id: &str) -> NodeId {
    let mut hit = None;
    doc.walk(Some(from), &mut |ev, entry| {
        if let (super::super::WalkEvent::Enter(n), NodeKind::Element(el)) = (ev, &entry.kind) {
            hit = hit.or((el.attr("id") == Some(id)).then_some(n));
        }
    });
    hit.unwrap()
}

#[test]
fn slots_take_children_by_name_and_fall_back() {
    let doc = Document::parse(
        "<div id=h><template shadowrootmode=open><slot name=a id=sa></slot><slot id=sd></slot>\
         <slot name=a id=late></slot><i><slot name=n id=sn>fb</slot></i></template>\
         <b slot=a id=b>1</b>t<!--c--><u slot=zz id=u>2</u></div><slot id=lone>l</slot>",
    );
    let h = doc.element_by_id("h").unwrap();
    let root = doc.shadow_root(h).unwrap();
    let by = |id| match id {
        "lone" | "u" => doc.element_by_id(id).unwrap(),
        _ => find(&doc, root, id),
    };
    let kids = doc.node(h).children.clone();
    assert_eq!(ids(&doc, &doc.assigned(by("sa"))), ["b"]);
    assert_eq!(ids(&doc, &doc.assigned(by("sd"))), ["t"]);
    assert!(doc.assigned(by("late")).is_empty());
    assert!(doc.assigned(by("sn")).is_empty());
    assert!(doc.assigned(by("lone")).is_empty());
    assert_eq!(doc.assigned_slot(kids[0]), Some(by("sa")));
    assert_eq!(doc.assigned_slot(kids[1]), Some(by("sd")));
    assert_eq!(doc.assigned_slot(kids[2]), None); // a comment is no slottable
    assert_eq!(doc.assigned_slot(by("u")), None);
    assert_eq!(doc.assigned_slot(by("sa")), None); // not a host's child
    assert_eq!(doc.assigned_slot(Document::ROOT), None); // no parent at all
    assert_eq!(doc.tree_root(by("sn")), root);
}
