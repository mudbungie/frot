//! What custom elements need of the tree (js.md §15): whether a node is
//! connected, and the elements of a subtree that are or may become custom —
//! the two questions the prelude's reactions ask at every insertion and
//! removal. The registry itself is JS; the custom *state* of an upgraded
//! element lives in [`super::live`], where `:defined` reads it.

use super::{Document, NodeId, NodeKind, WalkEvent};

impl Document {
    /// Whether `id`'s root is the document.
    pub fn connected(&self, id: NodeId) -> bool {
        let mut n = id;
        while let Some(p) = self.node(n).parent {
            n = p;
        }
        n == Self::ROOT
    }

    /// `id` and its descendants, in tree order, that are custom or could be
    /// made so: an element with a hyphen in its name, an `is` attribute, or a
    /// custom state. Which of them a definition matches is the registry's
    /// question, not the tree's.
    pub fn custom_nodes(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        self.walk(Some(id), &mut |ev, entry| {
            if let (WalkEvent::Enter(n), NodeKind::Element(el)) = (ev, &entry.kind) {
                if el.name.contains('-') || el.attr("is").is_some() || self.custom(n) {
                    out.push(n);
                }
            }
        });
        out
    }
}

#[cfg(test)]
mod tests;
