//! Manual slot assignment (`slot.assign()`, js.md §15.9): each slot's
//! manually assigned nodes, which a shadow root made with
//! `slotAssignment: "manual"` assigns by ([`crate::dom::shadow`]). A script
//! sets them and nothing in the tree records them, so they are live state.

use crate::dom::{Document, NodeId};

impl Document {
    /// The nodes `slot.assign()` last gave `slot`, in the order given.
    pub fn manually_assigned(&self, slot: NodeId) -> &[NodeId] {
        let hit = self.live.manual.iter().find(|(s, _)| *s == slot);
        hit.map_or(&[], |(_, nodes)| nodes)
    }

    /// Give `slot` the manually assigned `nodes` (each once, first place
    /// kept), taking each from any other slot that had it. Answers the slots
    /// whose list changed: the ones that lost a node, then `slot`.
    pub fn assign(&mut self, slot: NodeId, nodes: &[NodeId]) -> Vec<NodeId> {
        let mut list: Vec<NodeId> = Vec::new();
        for &n in nodes {
            if !list.contains(&n) {
                list.push(n);
            }
        }
        let mut changed = Vec::new();
        for (s, had) in self.live.manual.iter_mut().filter(|(s, _)| *s != slot) {
            let before = had.len();
            had.retain(|n| !list.contains(n));
            if had.len() != before {
                changed.push(*s);
            }
        }
        if self.manually_assigned(slot) != list.as_slice() {
            changed.push(slot);
        }
        self.live.manual.retain(|(s, _)| *s != slot);
        self.live.manual.push((slot, list));
        self.generation += 1;
        changed
    }
}

#[cfg(test)]
mod tests {
    use crate::dom::Document;

    #[test]
    fn assigning_takes_nodes_from_other_slots_and_reports_the_change() {
        let mut doc = Document::parse("<p>x</p>");
        let (s, t, a, b) = (10, 11, 12, 13);
        assert!(doc.manually_assigned(s).is_empty());
        assert_eq!(doc.assign(s, &[b, a, b]), [s]);
        assert_eq!(doc.manually_assigned(s), [b, a]);
        assert_eq!(doc.assign(s, &[b, a]), Vec::<u32>::new());
        assert_eq!(doc.assign(t, &[a]), [s, t]);
        assert_eq!(doc.manually_assigned(s), [b]);
        assert_eq!(doc.assign(t, &[]), [t]);
    }
}
