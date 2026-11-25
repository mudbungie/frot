//! The flat tree (js.md §15): what renders, as Chrome's layout and
//! accessibility tree compose it. A host's children are its shadow root's; a
//! slot's are the nodes assigned to it, else its own (fallback); a light
//! child no slot takes is in no flat tree at all. Every *rendering* reader —
//! the cascade's walk, layout, `text`, `ax`, `bboxes`, `links`, `forms`, the
//! needs detector — descends through [`Document::flat_children`] and climbs
//! through [`Document::flat_parent`]; every *node-tree* reader (selectors,
//! `querySelector`, `textContent`, scripts) keeps `children`/`parent`. A page
//! with no shadow root composes to its node tree, node for node.

use std::borrow::Cow;

use super::{Document, NodeEntry, NodeId, NodeKind, WalkEvent};

impl Document {
    /// `id`'s children in the flat tree.
    pub fn flat_children(&self, id: NodeId) -> Cow<'_, [NodeId]> {
        if let Some(root) = self.shadow_root(id) {
            return Cow::Borrowed(&self.node(root).children);
        }
        match self.slot_assigned(id) {
            Some(nodes) if !nodes.is_empty() => Cow::Owned(nodes),
            _ => Cow::Borrowed(&self.node(id).children),
        }
    }

    /// `id`'s parent in the flat tree: a shadow root's child climbs to the
    /// host, a host's child to its slot, and a node nothing renders — a light
    /// child no slot takes, a slot's fallback while it has assigned nodes —
    /// has none.
    pub fn flat_parent(&self, id: NodeId) -> Option<NodeId> {
        let parent = self.node(id).parent?;
        if let NodeKind::Shadow(s) = self.node(parent).kind {
            return Some(s.host);
        }
        if self.shadow_root(parent).is_some() {
            return self.assigned_slot(id);
        }
        match self.slot_assigned(parent) {
            Some(nodes) if !nodes.is_empty() => None,
            _ => Some(parent),
        }
    }

    /// [`Document::walk`] over the flat tree.
    pub fn walk_flat(&self, start: Option<NodeId>, f: &mut dyn FnMut(WalkEvent, &NodeEntry)) {
        let starts = match start {
            Some(id) => vec![id],
            None => self.roots().to_vec(),
        };
        for id in starts {
            self.walk_flat_node(id, f);
        }
    }

    fn walk_flat_node(&self, id: NodeId, f: &mut dyn FnMut(WalkEvent, &NodeEntry)) {
        f(WalkEvent::Enter(id), self.node(id));
        for &c in self.flat_children(id).iter() {
            self.walk_flat_node(c, f);
        }
        f(WalkEvent::Exit(id), self.node(id));
    }

    /// The nodes assigned to `id` when it is a `<slot>` in a shadow tree;
    /// `None` for any other node.
    fn slot_assigned(&self, id: NodeId) -> Option<Vec<NodeId>> {
        let slot = matches!(&self.node(id).kind, NodeKind::Element(el) if el.name == "slot");
        slot.then(|| self.assigned(id))
    }
}

#[cfg(test)]
mod tests;
