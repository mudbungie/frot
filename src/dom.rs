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

    /// Whether `id` is concealed by its **parent** — the UA rules that withhold
    /// a node for what its parent is, rather than for anything the node itself
    /// carries ([`Element::hidden`] is the attribute-local half of the same
    /// question). Answered for text nodes as much as elements: the box the UA
    /// skips is the parent's, so everything inside it goes, markup or not.
    ///
    /// Two rules, both of them "the parent's box swallows the child":
    ///
    /// - A **`<details>` without `open`** renders only its first `<summary>`
    ///   element child — the disclosure control. Every other child is
    ///   disclosure content, which HTML Rendering ("The `details` and `summary`
    ///   elements") puts in a `::details-content` box that is
    ///   `content-visibility: hidden` while closed. A closed `<details>` with
    ///   no `<summary>` conceals every child — the UA supplies its own
    ///   disclosure control, which is not in the document.
    /// - A **media element** renders *none* of its children
    ///   ([`crate::tags::renders_children`], which owns that fact and is also
    ///   read by the recipe-independent subtree skips in `views::text` and
    ///   `ax::tree`).
    ///
    /// So, unlike `[hidden]`, no author declaration on the child reveals it:
    /// the skipped box is not the child's, and frot has no anonymous boxes to
    /// give it one.
    ///
    /// Two consumers keep it honest, the same pair [`Element::hidden`] has: the
    /// cascade (`css::cascade`), which routes it on to `--css` text, the AX
    /// tree and geometry, and the needs detector (`needs::not_rendered`), which
    /// has no cascade to consult without `--css`.
    pub fn concealed(&self, id: NodeId) -> bool {
        let Some(parent) = self.node(id).parent else {
            return false;
        };
        self.withholds_children(parent)
            || (self.closed_details(parent) && self.first_summary(parent) != Some(id))
    }

    /// Whether `id` is an element that renders none of its children —
    /// [`crate::tags::renders_children`], the one home of the fallback-content
    /// fact. `<source>`/`<track>` are covered by it like any other child: they
    /// configure the box, they are never rendered beside it.
    pub fn withholds_children(&self, id: NodeId) -> bool {
        matches!(&self.node(id).kind,
            NodeKind::Element(el) if !crate::tags::renders_children(&el.name))
    }

    /// Whether `id` is a `<details>` element carrying no `open` attribute.
    /// `open` is a boolean attribute, so presence is the fact.
    fn closed_details(&self, id: NodeId) -> bool {
        matches!(&self.node(id).kind,
            NodeKind::Element(el) if el.name == "details" && el.attr("open").is_none())
    }

    /// The first `<summary>` element child of `id`, if any — the one child a
    /// closed `<details>` renders (`details > summary:first-of-type`); a later
    /// `<summary>` is disclosure content like any other child.
    fn first_summary(&self, id: NodeId) -> Option<NodeId> {
        self.node(id)
            .children
            .iter()
            .copied()
            .find(|&c| matches!(&self.node(c).kind, NodeKind::Element(el) if el.name == "summary"))
    }

    pub fn text_content(&self, id: NodeId) -> String {
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

mod mutate;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod concealed_tests;
