//! Thin DOM facade over `html5ever` + `markup5ever_rcdom`.
//!
//! The rest of the crate consumes this module's [`Document`] without ever
//! touching `markup5ever`/`rcdom` types directly. That lets us swap the
//! parser/storage later (e.g. arena-allocated nodes for `--css` side tables)
//! without rewriting every view.

use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{NodeData, RcDom};

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
    Element(Element),
    Text(String),
    Comment(String),
    Doctype,
}

#[derive(Debug, Clone)]
pub struct NodeEntry {
    pub kind: NodeKind,
    pub children: Vec<NodeId>,
    pub parent: Option<NodeId>,
}

#[derive(Debug, Clone, Copy)]
pub enum WalkEvent {
    Enter(NodeId),
    Exit(NodeId),
}

#[derive(Debug, Default, Clone)]
pub struct Document {
    nodes: Vec<NodeEntry>,
    roots: Vec<NodeId>,
    /// Bumped by every §2 mutation op (see [`mutate`]). Per-generation
    /// `Styles`/`Layout` caches downstream invalidate when it changes; a freshly
    /// parsed document sits at generation 0.
    generation: u64,
}

impl Document {
    pub fn parse(html: &str) -> Self {
        let rc = html5ever::parse_document(RcDom::default(), Default::default()).one(html);
        let mut doc = Document::default();
        let handles: Vec<_> = rc.document.children.borrow().iter().cloned().collect();
        for h in handles {
            let id = doc.absorb(&h, None);
            doc.roots.push(id);
        }
        doc
    }

    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    pub fn node(&self, id: NodeId) -> &NodeEntry {
        &self.nodes[id as usize]
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn walk(&self, start: Option<NodeId>, f: &mut dyn FnMut(WalkEvent, &NodeEntry)) {
        let starts: Vec<NodeId> = match start {
            Some(id) => vec![id],
            None => self.roots.clone(),
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

    fn absorb(&mut self, h: &markup5ever_rcdom::Handle, parent: Option<NodeId>) -> NodeId {
        let kind = node_data_to_kind(&h.data);
        let id = self.nodes.len() as NodeId;
        self.nodes.push(NodeEntry {
            kind,
            children: Vec::new(),
            parent,
        });
        let kids: Vec<_> = h.children.borrow().iter().cloned().collect();
        for k in kids {
            let cid = self.absorb(&k, Some(id));
            self.nodes[id as usize].children.push(cid);
        }
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
        NodeData::Doctype { .. } => NodeKind::Doctype,
        // html5ever in HTML mode never emits Document/PI as a child; we map
        // them to empty comments rather than carry an `Option` everywhere.
        NodeData::Document | NodeData::ProcessingInstruction { .. } => {
            NodeKind::Comment(String::new())
        }
    }
}

mod conceal;
mod mutate;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod concealed_tests;
