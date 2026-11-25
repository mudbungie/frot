//! What custom elements need of the tree (js.md §15): whether a node is
//! connected, and the elements of a subtree that are or may become custom —
//! the two questions the prelude's reactions ask at every insertion and
//! removal. The registry itself is JS; the custom *state* of an upgraded
//! element lives in [`super::live`], where `:defined` reads it.

use super::{Document, NodeId, NodeKind};

impl Document {
    /// Whether `id`'s shadow-including root is the document: the top of its
    /// tree, continued through each shadow root to its host.
    pub fn connected(&self, id: NodeId) -> bool {
        let mut n = self.tree_root(id);
        while let NodeKind::Shadow(s) = self.node(n).kind {
            n = self.tree_root(s.host);
        }
        n == Self::ROOT
    }

    /// `id` and its shadow-including descendants, in shadow-including tree
    /// order (an element, then its shadow tree, then its children), that are
    /// custom or could be made so: an element with a hyphen in its name, an
    /// `is` attribute, or a custom state. Which of them a definition matches
    /// is the registry's question, not the tree's.
    pub fn custom_nodes(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        self.collect_custom(id, &mut out);
        out
    }

    fn collect_custom(&self, id: NodeId, out: &mut Vec<NodeId>) {
        if let NodeKind::Element(el) = &self.node(id).kind {
            if el.name.contains('-') || el.attr("is").is_some() || self.custom(id) {
                out.push(id);
            }
        }
        let shadow = self.shadow_root(id);
        for &c in shadow.iter().chain(&self.node(id).children) {
            self.collect_custom(c, out);
        }
    }
}

#[cfg(test)]
mod tests;
