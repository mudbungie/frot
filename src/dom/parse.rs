//! Parsing into the arena: html5ever's tree, copied node by node. The parsed
//! `RcDom` never outlives the call; nothing but the arena keeps its shape.

use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};

use super::{Attr, Document, Element, NodeId, NodeKind};

impl Document {
    pub fn parse(html: &str) -> Self {
        let rc = html5ever::parse_document(RcDom::default(), Default::default()).one(html);
        let mut doc = Document::default();
        doc.absorb_children(&rc.document, Self::ROOT, true);
        doc
    }

    /// Copy the parsed node `h` and its subtree into the arena under `parent`
    /// (not yet in `parent`'s child list). A `<template>`'s contents, which
    /// html5ever keeps beside its children, become the fragment it owns.
    fn absorb(&mut self, h: &Handle, parent: Option<NodeId>, dsd: bool) -> NodeId {
        let id = self.push(node_data_to_kind(&h.data), parent);
        if let NodeData::Element {
            template_contents, ..
        } = &h.data
        {
            if let Some(contents) = template_contents.borrow().as_ref() {
                let fragment = self.push(NodeKind::Fragment, None);
                self.absorb_children(contents, fragment, dsd);
                self.nodes[id as usize].content = Some(fragment);
            }
        }
        self.absorb_children(h, id, dsd);
        id
    }

    /// Absorb every child of `h` and append it to `id`'s child list — except
    /// a declarative shadow root, when `dsd` admits one (the document's
    /// parse does, `innerHTML` does not): a `<template shadowrootmode>` child
    /// of a host that has no shadow root yet becomes that root, its contents
    /// the root's children, and is no child at all (HTML, "in head" insertion
    /// mode; html5ever leaves the attach to the tree, and rcdom declines it).
    pub(super) fn absorb_children(&mut self, h: &Handle, id: NodeId, dsd: bool) {
        let kids: Vec<_> = h.children.borrow().iter().cloned().collect();
        for k in kids {
            if let Some((init, contents)) = dsd.then(|| self.declarative(&k, id)).flatten() {
                let root = self.append_root(init);
                self.absorb_children(&contents, root, dsd);
                continue;
            }
            let cid = self.absorb(&k, Some(id), dsd);
            self.nodes[id as usize].children.push(cid);
        }
    }
}

pub(super) fn node_data_to_kind(data: &NodeData) -> NodeKind {
    match data {
        NodeData::Element { name, attrs, .. } => NodeKind::Element(Element {
            name: name.local.to_string().to_ascii_lowercase(),
            attrs: attrs
                .borrow()
                .iter()
                .map(|a| Attr {
                    name: a.name.local.to_string().to_ascii_lowercase(),
                    value: a.value.to_string(),
                })
                .collect(),
        }),
        NodeData::Text { contents } => NodeKind::Text(contents.borrow().to_string()),
        NodeData::Comment { contents } => NodeKind::Comment(contents.to_string()),
        NodeData::Doctype {
            name,
            public_id,
            system_id,
        } => NodeKind::Doctype([name, public_id, system_id].map(|t| t.to_string())),
        // html5ever in HTML mode never emits Document/PI as a child; we map
        // them to empty comments rather than carry an `Option` everywhere.
        NodeData::Document | NodeData::ProcessingInstruction { .. } => {
            NodeKind::Comment(String::new())
        }
    }
}
