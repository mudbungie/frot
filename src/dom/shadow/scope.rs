//! What a tree mutation can change of slot assignment (js.md §15.8), asked
//! once per mutation by the prelude's `slotchange` seam — so a page with no
//! shadow tree pays one call that answers nothing.

use crate::dom::{Document, NodeId, NodeKind, WalkEvent};

impl Document {
    /// Moving the subtree at `node` under `parent` (`None`: removing it from
    /// its parent): first the shadow roots whose slots may gain or lose nodes
    /// — a host's own, and the tree a slot moves in or out of — then the
    /// unassigned slots whose own children change, which Firefox signals too.
    pub fn slot_scope(&self, parent: Option<NodeId>, node: NodeId) -> Vec<Vec<NodeId>> {
        let (mut roots, mut fallback) = (Vec::new(), Vec::new());
        for p in [parent, self.node(node).parent].into_iter().flatten() {
            roots.extend(self.shadow_root(p));
            let root = self.tree_root(p);
            if self.shadow(root).is_none() {
                continue;
            }
            if self.holds_slot(node) {
                roots.push(root);
            }
            if self.element(p).is_some_and(|e| e.name == "slot") && self.assigned(p).is_empty() {
                fallback.push(p);
            }
        }
        vec![roots, fallback]
    }

    fn holds_slot(&self, node: NodeId) -> bool {
        let mut found = false;
        self.walk(Some(node), &mut |ev, entry| {
            if let (WalkEvent::Enter(_), NodeKind::Element(el)) = (ev, &entry.kind) {
                found |= el.name == "slot";
            }
        });
        found
    }
}

#[cfg(test)]
mod tests {
    use crate::dom::Document;

    #[test]
    fn only_a_host_or_a_slot_moving_in_a_shadow_tree_is_in_scope() {
        let mut doc = Document::parse(
            "<div id=h><template shadowrootmode=open><slot id=s></slot><p id=p>x</p>\
             </template><b id=b>b</b></div><i id=i>i</i>",
        );
        let by = |d: &Document, s: &str| d.element_by_id(s);
        let (h, b, i) = (
            by(&doc, "h").unwrap(),
            by(&doc, "b").unwrap(),
            by(&doc, "i").unwrap(),
        );
        let root = doc.shadow_root(h).unwrap();
        let none: Vec<Vec<u32>> = vec![vec![], vec![]];
        assert_eq!(doc.slot_scope(Some(Document::ROOT), i), none);
        assert_eq!(doc.slot_scope(None, b), [vec![root], vec![]]);
        let kids = doc.node(root).children.clone();
        let (s, p) = (kids[0], kids[1]);
        assert_eq!(doc.slot_scope(Some(p), i), none);
        assert_eq!(doc.slot_scope(None, s), [vec![root], vec![]]);
        doc.detach(b);
        assert_eq!(doc.slot_scope(Some(s), i), [vec![], vec![s]]);
    }
}
