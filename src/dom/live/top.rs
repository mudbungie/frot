//! The top layer's rules beyond showing and hiding (`bl-2dcd`), measured in
//! Chrome 158 (`tests/fixtures/fidelity/topjs.html`):
//!
//! - **Light dismiss.** Showing an `auto` popover (the attribute empty or
//!   `auto`, any case) hides every open `auto` and `hint` popover that is not
//!   its ancestor; showing a `hint` popover hides every open `hint` one that
//!   is not its ancestor. A `manual` popover — that value, or any value HTML
//!   does not know — is never dismissed.
//! - **A modal dialog.** `showModal()` hides every open `auto` and `hint`
//!   popover, and while it is open everything outside the topmost modal
//!   dialog is inert: no node of the accessibility tree (its ancestors are
//!   passed through), and nothing there takes focus.

use crate::dom::{Document, NodeId};

#[derive(PartialEq, Eq)]
enum Kind {
    Auto,
    Hint,
    Manual,
}

impl Document {
    /// Show the popover `id`, dismissing the popovers it light-dismisses.
    pub fn show_popover(&mut self, id: NodeId) {
        let kind = self.kind(id);
        self.dismiss(|doc, p| match kind {
            Kind::Auto => doc.kind(p) != Kind::Manual && !doc.inside(id, p),
            Kind::Hint => doc.kind(p) == Kind::Hint && !doc.inside(id, p),
            Kind::Manual => false,
        });
        self.set_popover(id, true);
    }

    /// Show the dialog `id` modally, dismissing every `auto` and `hint`
    /// popover.
    pub fn show_modal(&mut self, id: NodeId) {
        self.dismiss(|doc, p| doc.kind(p) != Kind::Manual);
        self.set_modal(id, true);
    }

    /// Whether `id` is inert behind the topmost modal dialog: one is open and
    /// `id` is not inside it.
    pub fn blocked(&self, id: NodeId) -> bool {
        self.top_modal().is_some_and(|m| !self.inside(id, m))
    }

    /// The modal dialog shown last and still open.
    pub fn top_modal(&self) -> Option<NodeId> {
        self.live.modals.last().copied()
    }

    /// Whether `id` is `root` or inside it.
    pub fn inside(&self, id: NodeId, root: NodeId) -> bool {
        std::iter::successors(Some(id), |&n| self.node(n).parent).any(|n| n == root)
    }

    fn dismiss(&mut self, hides: impl Fn(&Document, NodeId) -> bool) {
        let gone: Vec<NodeId> = self
            .live
            .popovers
            .iter()
            .copied()
            .filter(|&p| hides(self, p))
            .collect();
        for p in gone {
            self.set_popover(p, false);
        }
    }

    fn kind(&self, id: NodeId) -> Kind {
        let value = self.element(id).and_then(|e| e.attr("popover"));
        match value.map(str::to_ascii_lowercase).as_deref() {
            Some("" | "auto") => Kind::Auto,
            Some("hint") => Kind::Hint,
            _ => Kind::Manual,
        }
    }
}
