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

    /// Append a detached element node (no parent) and return its id. A
    /// `<template>` comes with its empty contents, so every template owns a
    /// fragment however it was made (js.md §15).
    pub fn create_element(&mut self, name: &str) -> NodeId {
        let name = name.to_ascii_lowercase();
        let template = name == "template";
        let id = self.append_node(NodeKind::Element(Element {
            name,
            attrs: Vec::new(),
        }));
        if template {
            self.nodes[id as usize].content = Some(self.create_fragment());
        }
        id
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

    /// A detached copy of node `id` itself, without its children — what
    /// `cloneNode` makes of a doctype, whose three strings no other creation
    /// takes (bl-ad2d).
    pub fn copy_node(&mut self, id: NodeId) -> NodeId {
        let kind = self.nodes[id as usize].kind.clone();
        self.append_node(kind)
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

    /// Replace the data of a text or comment node (CharacterData); a no-op on
    /// any other kind.
    pub fn set_text(&mut self, id: NodeId, text: &str) {
        if let NodeKind::Text(t) | NodeKind::Comment(t) = &mut self.nodes[id as usize].kind {
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
    ///
    /// An insertion the tree cannot hold is refused before anything moves, and
    /// the reason returned ([`Document::refusal`]): that is what keeps the
    /// arena a tree and the document's children a document's.
    pub fn insert_child(
        &mut self,
        parent: NodeId,
        child: NodeId,
        before: Option<NodeId>,
    ) -> Result<(), String> {
        if let Some(why) = self.refusal(parent, child, before, None) {
            return Err(why);
        }
        let kids = &self.nodes[parent as usize].children;
        let before = match before {
            Some(r) if r == child => kids
                .iter()
                .position(|&c| c == r)
                .and_then(|at| kids.get(at + 1).copied()),
            r => r,
        };
        // DOM "insert": a fragment is never linked — its children move, in
        // order, to where it was aimed, and it is left empty.
        let moved = match self.nodes[child as usize].kind {
            NodeKind::Fragment | NodeKind::Shadow(_) => self.nodes[child as usize].children.clone(),
            _ => vec![child],
        };
        for node in moved {
            self.unlink(node);
            self.nodes[node as usize].parent = Some(parent);
            let kids = &mut self.nodes[parent as usize].children;
            let at = before
                .and_then(|r| kids.iter().position(|&c| c == r))
                .unwrap_or(kids.len());
            kids.insert(at, node);
        }
        self.generation += 1;
        Ok(())
    }

    /// Append an empty, detached fragment and return its id.
    pub fn create_fragment(&mut self) -> NodeId {
        self.append_node(NodeKind::Fragment)
    }

    /// Unlink a node from its parent. The entry stays in the
    /// arena; its subtree becomes unreachable by `walk`.
    pub fn detach(&mut self, id: NodeId) {
        self.unlink(id);
        self.generation += 1;
    }

    /// Parse an `innerHTML` fragment into the arena, returning the fragment
    /// that holds the parsed nodes, for the caller to splice in with one
    /// [`insert_child`].
    ///
    /// [`insert_child`]: Document::insert_child
    pub fn parse_fragment(&mut self, html: &str) -> NodeId {
        let ctx = QualName::new(None, ns!(html), local_name!("body"));
        let rc =
            html5ever::parse_fragment(RcDom::default(), Default::default(), ctx, vec![], false)
                .one(html);
        // Fragment parsing nests the result under a synthetic <html> wrapper
        // (`#document > <html> > fragment nodes`); absorb the wrapper's children.
        let wrapper = rc.document.children.borrow()[0].clone();
        let fragment = self.create_fragment();
        self.absorb_children(&wrapper, fragment, false);
        // The fragment parser starts every script it makes: an `innerHTML`
        // script never runs (HTML §13.4, `bl-a86b`; measured in Firefox).
        for id in self.scripts_in(fragment) {
            self.start(id);
        }
        fragment
    }

    /// Parse a whole document — an iframe's `srcdoc` (js.md §14, bl-7b86) —
    /// into the arena as a detached fragment holding its top-level nodes (the
    /// doctype, comments, the `<html>` element), for the caller to put in
    /// place of a document's own.
    pub fn parse_document(&mut self, html: &str) -> NodeId {
        let rc = html5ever::parse_document(RcDom::default(), Default::default()).one(html);
        let fragment = self.create_fragment();
        self.absorb_children(&rc.document, fragment, true);
        fragment
    }

    fn append_node(&mut self, kind: NodeKind) -> NodeId {
        self.generation += 1;
        self.push(kind, None)
    }

    /// Remove `id` from its parent's children, if it has a parent, without
    /// touching the generation. A top-level node's parent is the document node,
    /// so it needs no case of its own.
    fn unlink(&mut self, id: NodeId) {
        if let Some(p) = self.nodes[id as usize].parent.take() {
            self.nodes[p as usize].children.retain(|&c| c != id);
        }
    }
}

#[cfg(test)]
mod tests;
