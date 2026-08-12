//! §2 arena mutation ops — the `--js` capability's exclusive DOM write path.
//!
//! The arena stays the one document (no mirror DOM). Every op is append +
//! relink: the `nodes` `Vec` is append-only, so [`NodeId`]s are stable forever
//! and `NodeId`-keyed side tables stay parallel; `detach` merely unlinks, and a
//! detached subtree is simply never reached by `walk`. Each op bumps the
//! [`Document::generation`] counter so per-generation caches invalidate.

use super::*;
use html5ever::tendril::TendrilSink;
use html5ever::{local_name, ns, QualName};
use markup5ever_rcdom::RcDom;

impl Document {
    /// Current mutation generation; a parsed document starts at 0.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Append a detached element node (no parent, not a root) and return its id.
    pub fn create_element(&mut self, name: &str) -> NodeId {
        self.append_node(NodeKind::Element(Element {
            name: name.to_ascii_lowercase(),
            attrs: Vec::new(),
        }))
    }

    /// Append a detached text node and return its id.
    pub fn create_text(&mut self, text: &str) -> NodeId {
        self.append_node(NodeKind::Text(text.to_string()))
    }

    /// Append a detached comment node and return its id. Comments are real
    /// arena nodes, not host-side fakes: frameworks use them as *anchors* —
    /// placeholders whose `parentNode`/`nextSibling` are read back to position
    /// later content (Vue's RouterView/`v-if`, `bl-79db`) — so a comment that
    /// never enters the tree strands every patch that navigates from it. The
    /// parser already stores `NodeKind::Comment`; this is creation parity.
    pub fn create_comment(&mut self, text: &str) -> NodeId {
        self.append_node(NodeKind::Comment(text.to_string()))
    }

    /// Set (or overwrite) an attribute on an element node; a no-op on any other
    /// kind. Totality keeps the caller from having to pre-check the node kind.
    pub fn set_attr(&mut self, id: NodeId, name: &str, value: &str) {
        if let NodeKind::Element(el) = &mut self.nodes[id as usize].kind {
            let name = name.to_ascii_lowercase();
            match el.attrs.iter_mut().find(|a| a.name == name) {
                Some(a) => a.value = value.to_string(),
                None => el.attrs.push(Attr {
                    name,
                    value: value.to_string(),
                }),
            }
        }
        self.generation += 1;
    }

    /// Remove an attribute from an element node, if present; a no-op on any
    /// other kind.
    pub fn remove_attr(&mut self, id: NodeId, name: &str) {
        if let NodeKind::Element(el) = &mut self.nodes[id as usize].kind {
            let name = name.to_ascii_lowercase();
            el.attrs.retain(|a| a.name != name);
        }
        self.generation += 1;
    }

    /// Replace the data of a text node; a no-op on any other kind.
    pub fn set_text(&mut self, id: NodeId, text: &str) {
        if let NodeKind::Text(t) = &mut self.nodes[id as usize].kind {
            *t = text.to_string();
        }
        self.generation += 1;
    }

    /// Link `child` under `parent` immediately before the sibling `before`;
    /// `None` appends. This is the DOM's own `insertBefore(child, ref)`, and the
    /// position is a **node, never an index**: `child` is unlinked from its old
    /// parent first, so an index computed against the pre-move sibling list
    /// addresses the wrong slot the moment the move is within one parent — and
    /// off the end of it for `appendChild` of a node already last (bl-ae88).
    /// Naming the sibling instead makes that whole class arithmetic-free.
    ///
    /// Two degenerate references stay browser-shaped by falling out of the same
    /// rule: `before == child` resolves to `child`'s next sibling (the spec's
    /// own step), so self-insertion is a no-op; a `before` that is not a child
    /// of `parent` appends, which is the one deliberate mercy — a browser throws
    /// `NotFoundError`, but a misplaced node still yields an impression where a
    /// thrown exception kills the script that was building one.
    pub fn insert_child(&mut self, parent: NodeId, child: NodeId, before: Option<NodeId>) {
        let kids = &self.nodes[parent as usize].children;
        let before = match before {
            Some(r) if r == child => kids
                .iter()
                .position(|&c| c == r)
                .and_then(|at| kids.get(at + 1).copied()),
            r => r,
        };
        self.unlink(child);
        self.nodes[child as usize].parent = Some(parent);
        let kids = &mut self.nodes[parent as usize].children;
        let at = before
            .and_then(|r| kids.iter().position(|&c| c == r))
            .unwrap_or(kids.len());
        kids.insert(at, child);
        self.generation += 1;
    }

    /// Unlink a node from its parent (or the root list). The entry stays in the
    /// arena; its subtree becomes unreachable by `walk`.
    pub fn detach(&mut self, id: NodeId) {
        self.unlink(id);
        self.generation += 1;
    }

    /// Parse an `innerHTML` fragment into the arena, returning the detached
    /// top-level node ids for the caller to splice in via [`insert_child`].
    ///
    /// [`insert_child`]: Document::insert_child
    pub fn parse_fragment(&mut self, html: &str) -> Vec<NodeId> {
        let ctx = QualName::new(None, ns!(html), local_name!("body"));
        let rc =
            html5ever::parse_fragment(RcDom::default(), Default::default(), ctx, vec![], false)
                .one(html);
        // Fragment parsing nests the result under a synthetic <html> wrapper
        // (`#document > <html> > fragment nodes`); absorb the wrapper's children.
        let wrapper = rc.document.children.borrow()[0].clone();
        let kids: Vec<_> = wrapper.children.borrow().iter().cloned().collect();
        let ids = kids.iter().map(|k| self.absorb(k, None)).collect();
        self.generation += 1;
        ids
    }

    fn append_node(&mut self, kind: NodeKind) -> NodeId {
        let id = self.nodes.len() as NodeId;
        self.nodes.push(NodeEntry {
            kind,
            children: Vec::new(),
            parent: None,
        });
        self.generation += 1;
        id
    }

    /// Remove `id` from wherever it is currently linked (parent's children, or
    /// the root list when it has no parent) without touching the generation.
    fn unlink(&mut self, id: NodeId) {
        match self.nodes[id as usize].parent.take() {
            Some(p) => self.nodes[p as usize].children.retain(|&c| c != id),
            None => self.roots.retain(|&r| r != id),
        }
    }
}

#[cfg(test)]
mod tests;
