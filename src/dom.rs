//! Thin DOM facade over `html5ever` + `markup5ever_rcdom`.
//!
//! The rest of the crate consumes this module's [`Document`] without ever
//! touching `markup5ever`/`rcdom` types directly. That lets us swap the
//! parser/storage later (e.g. arena-allocated nodes for `--css` side tables)
//! without rewriting every view.

use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};

pub type NodeId = u32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attr {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub name: String,
    pub attrs: Vec<Attr>,
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str())
    }

    /// The HTML `hidden` attribute — the single home of the UA rule
    /// `[hidden] { display: none }` (HTML §15.3.1). `hidden` is a boolean
    /// attribute, so *presence* is the fact and the value is noise
    /// (`hidden=""`, `hidden="false"` and `hidden="until-found"` are all
    /// not-rendered). Two consumers keep it honest and cannot drift: the CSS
    /// cascade's UA-implicit `display` step (`css::computed`), which routes it
    /// on to `--css` text, the AX tree and geometry, and the needs detector's
    /// non-content skip (`needs::not_rendered`), which has no cascade to
    /// consult without `--css`. [`Document::concealed`] is the structural half
    /// of the same question — hidden for what the *parent* is.
    pub fn hidden(&self) -> bool {
        self.attr("hidden").is_some()
    }

    /// `<input type=hidden>` — the other half of the same UA "hidden elements"
    /// block (HTML §15.3.1), and the one declaration in it marked
    /// `!important`, so unlike [`Element::hidden`] no author rule can render
    /// it. Kept a distinct predicate for exactly that reason: same rule, two
    /// origins. The AX side is separate and stays separate — the HTML-AAM gives
    /// `type=hidden` *no role* (`ax::role`), which is what removes it from
    /// `--out ax` when no styles are computed at all.
    pub fn hidden_input(&self) -> bool {
        self.name == "input"
            && self
                .attr("type")
                .is_some_and(|t| t.eq_ignore_ascii_case("hidden"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    /// The document itself: the one node with no parent whose children are the
    /// document's top-level nodes. Always [`Document::ROOT`], never elsewhere.
    Document,
    /// A document fragment: a parentless holder of nodes. It is never a child
    /// — inserting one moves its children instead ([`Document::insert_child`]).
    Fragment,
    Element(Element),
    Text(String),
    Comment(String),
    /// `<!DOCTYPE name PUBLIC "public" "system">`: what `DocumentType`'s
    /// `name`, `publicId` and `systemId` read (each '' when absent).
    Doctype([String; 3]),
}

#[derive(Debug, Clone)]
pub struct NodeEntry {
    pub kind: NodeKind,
    pub children: Vec<NodeId>,
    pub parent: Option<NodeId>,
    /// The one fragment an element owns outside its child list: a
    /// `<template>`'s contents (js.md §15). It is not a child — `walk`,
    /// selectors and `textContent` never enter it — and its own top-level
    /// nodes have it as their parent.
    pub content: Option<NodeId>,
}

#[derive(Debug, Clone, Copy)]
pub enum WalkEvent {
    Enter(NodeId),
    Exit(NodeId),
}

/// The arena. Node [`Document::ROOT`] is the document node; every other entry
/// is reachable from it or detached (parentless). The document's top-level
/// nodes are that node's children — one list, so "the roots" and "the
/// document's children" cannot drift (`bl-196a`).
#[derive(Debug, Clone)]
pub struct Document {
    nodes: Vec<NodeEntry>,
    /// Bumped by every §2 mutation op (see [`mutate`]). Per-generation
    /// `Styles`/`Layout` caches downstream invalidate when it changes; a freshly
    /// parsed document sits at generation 0.
    generation: u64,
    /// The fragment of the document's URL, as written (still
    /// percent-encoded): what [`Document::target`] resolves. `None` for a URL
    /// with no `#`.
    fragment: Option<String>,
    /// What scripts did to the page that leaves no mark in the tree: focus,
    /// the top layer ([`live::Live`]).
    live: live::Live,
}

/// The empty document: the document node alone.
impl Default for Document {
    fn default() -> Self {
        Document {
            nodes: vec![NodeEntry {
                kind: NodeKind::Document,
                children: Vec::new(),
                parent: None,
                content: None,
            }],
            generation: 0,
            fragment: None,
            live: live::Live::default(),
        }
    }
}

impl Document {
    /// The document node's id. It is entry 0 of every arena, so a view that
    /// walks [`Document::roots`] never meets it and a parent chain ends on it.
    pub const ROOT: NodeId = 0;

    pub fn parse(html: &str) -> Self {
        let rc = html5ever::parse_document(RcDom::default(), Default::default()).one(html);
        let mut doc = Document::default();
        doc.absorb_children(&rc.document, Self::ROOT);
        doc
    }

    /// The document's top-level nodes: the document node's children.
    pub fn roots(&self) -> &[NodeId] {
        &self.nodes[Self::ROOT as usize].children
    }

    pub fn node(&self, id: NodeId) -> &NodeEntry {
        &self.nodes[id as usize]
    }

    /// The arena's size, the document node included — the length every
    /// `NodeId`-keyed side table is built to.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn walk(&self, start: Option<NodeId>, f: &mut dyn FnMut(WalkEvent, &NodeEntry)) {
        let starts: Vec<NodeId> = match start {
            Some(id) => vec![id],
            None => self.roots().to_vec(),
        };
        for id in starts {
            self.walk_node(id, f);
        }
    }

    fn walk_node(&self, id: NodeId, f: &mut dyn FnMut(WalkEvent, &NodeEntry)) {
        let entry = &self.nodes[id as usize];
        f(WalkEvent::Enter(id), entry);
        let kids = entry.children.clone();
        for c in kids {
            self.walk_node(c, f);
        }
        f(WalkEvent::Exit(id), &self.nodes[id as usize]);
    }

    pub fn find_by_tag(&self, name: &str) -> Vec<NodeId> {
        let mut out = Vec::new();
        self.walk(None, &mut |ev, entry| {
            if let WalkEvent::Enter(id) = ev {
                if let NodeKind::Element(e) = &entry.kind {
                    if e.name == name {
                        out.push(id);
                    }
                }
            }
        });
        out
    }

    /// The DOM's `textContent` (DOM §4.4): for a **comment** its own data, for
    /// anything else the concatenated text of its Text descendants — comments
    /// among them contributing nothing, which is why a comment asked about
    /// itself needs the first arm. Without it a comment reads as `''` through
    /// every JS door onto it (`textContent`, `nodeValue`, `data`) and the
    /// markup serializer emits `<!---->`, deleting the one thing the node is.
    pub fn text_content(&self, id: NodeId) -> String {
        if let NodeKind::Comment(c) = &self.node(id).kind {
            return c.clone();
        }
        let mut s = String::new();
        self.walk(Some(id), &mut |ev, entry| {
            if let WalkEvent::Enter(_) = ev {
                if let NodeKind::Text(t) = &entry.kind {
                    s.push_str(t);
                }
            }
        });
        s
    }

    /// Copy the parsed node `h` and its subtree into the arena under `parent`
    /// (not yet in `parent`'s child list). A `<template>`'s contents, which
    /// html5ever keeps beside its children, become the fragment it owns.
    fn absorb(&mut self, h: &Handle, parent: Option<NodeId>) -> NodeId {
        let id = self.push(node_data_to_kind(&h.data), parent);
        if let NodeData::Element {
            template_contents, ..
        } = &h.data
        {
            if let Some(contents) = template_contents.borrow().as_ref() {
                let fragment = self.push(NodeKind::Fragment, None);
                self.absorb_children(contents, fragment);
                self.nodes[id as usize].content = Some(fragment);
            }
        }
        self.absorb_children(h, id);
        id
    }

    /// Absorb every child of `h` and append it to `id`'s child list.
    fn absorb_children(&mut self, h: &Handle, id: NodeId) {
        let kids: Vec<_> = h.children.borrow().iter().cloned().collect();
        for k in kids {
            let cid = self.absorb(&k, Some(id));
            self.nodes[id as usize].children.push(cid);
        }
    }

    /// Append a node with no children to the arena and return its id.
    fn push(&mut self, kind: NodeKind, parent: Option<NodeId>) -> NodeId {
        let id = self.nodes.len() as NodeId;
        self.nodes.push(NodeEntry {
            kind,
            children: Vec::new(),
            parent,
            content: None,
        });
        id
    }
}

fn node_data_to_kind(data: &NodeData) -> NodeKind {
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

mod conceal;
mod form;
mod hierarchy;
pub mod live;
mod mutate;
mod pseudo;
mod select;
mod target;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod concealed_tests;
