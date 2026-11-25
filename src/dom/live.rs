//! Page state a script sets without leaving a mark in the tree (`bl-bc6d`):
//! where focus is, which popovers and modal dialogs are shown, which
//! elements a custom element definition has upgraded and the stylesheets a
//! script adopted onto the document or a shadow root (js.md §15), and the
//! checkedness, selectedness and `indeterminate` a script gave a control. It lives
//! in the arena beside the nodes, so the one document the views, the cascade
//! and the JS host share carries it — nothing global, nothing cached in JS.
//! The JS host writes it (`focus()`, `blur()`, `showModal()`, `close()`,
//! `showPopover()`, `hidePopover()`); the selector engine and
//! `document.activeElement` read it.
//!
//! It also holds each `<script>`'s "already started" flag (HTML, the script element,
//! `bl-a86b`): a script runs at most once, whenever and however it reaches
//! the document, and the fragment parser starts every script it makes so an
//! `innerHTML` script never runs. A flag of the element, so it is here, not
//! in the JS host — as is its "force async" flag (`bl-f9f2`).

use std::collections::BTreeSet;

use super::{Document, NodeId, NodeKind, WalkEvent};

/// Where a script has put focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    /// No script has moved it: the `autofocus` rule decides.
    #[default]
    Auto,
    /// `el.focus()` put it on this element.
    On(NodeId),
    /// `blur()` took it away: nothing is focused.
    Off,
}

#[derive(Debug, Clone, Default)]
pub struct Live {
    focus: Focus,
    /// Open popovers and modal dialogs, in the order they were shown.
    popovers: Vec<NodeId>,
    modals: Vec<NodeId>,
    /// `<script>` elements that are "already started".
    started: Vec<NodeId>,
    /// Elements whose "force async" flag is unset (`bl-f9f2`): the parser's
    /// own scripts, and any a script gave an `async` (attribute or property).
    unforced: Vec<NodeId>,
    /// Elements whose custom element state is "custom": constructed by
    /// their definition (`:defined`).
    custom: BTreeSet<NodeId>,
    /// The text of each constructed sheet in a root's `adoptedStyleSheets`,
    /// in order, keyed by the root (the document or a shadow root).
    adopted: Vec<(NodeId, Vec<String>)>,
    /// The checkedness of an `<input>` or selectedness of an `<option>` a
    /// script set — HTML's dirty flag (`bl-ff74`); absent, the attribute
    /// rules (`css/checked.rs`).
    picked: Vec<(NodeId, bool)>,
    /// Checkboxes a script made `indeterminate`.
    indeterminate: Vec<NodeId>,
    /// What elements' `ElementInternals` set ([`internals`]).
    internals: Vec<(NodeId, internals::Internals)>,
}

impl Document {
    pub fn focus(&self) -> Focus {
        self.live.focus
    }

    /// Every write bumps [`Document::generation`]: what is focused or shown
    /// changes the cascade, so per-generation caches must not outlive it.
    pub fn set_focus(&mut self, focus: Focus) {
        self.live.focus = focus;
        self.generation += 1;
    }

    /// Whether `id` is a shown popover (`:popover-open`).
    pub fn popover_open(&self, id: NodeId) -> bool {
        self.live.popovers.contains(&id)
    }

    /// Whether `id` is a dialog shown with `showModal()` (`:modal`).
    pub fn modal(&self, id: NodeId) -> bool {
        self.live.modals.contains(&id)
    }

    /// Show or hide the popover `id`.
    pub fn set_popover(&mut self, id: NodeId, open: bool) {
        toggle(&mut self.live.popovers, id, open);
        self.generation += 1;
    }

    /// Whether the script `id` is "already started" — it ran, or never will.
    pub fn started(&self, id: NodeId) -> bool {
        self.live.started.contains(&id)
    }

    /// Mark the script `id` started. Not a rendering fact: no generation bump.
    pub fn start(&mut self, id: NodeId) {
        if !self.started(id) {
            self.live.started.push(id);
        }
    }

    /// Whether the script `id`'s "force async" flag is set — true from its
    /// creation, so a script a script makes is async (`bl-f9f2`, measured).
    pub fn force_async(&self, id: NodeId) -> bool {
        !self.live.unforced.contains(&id)
    }

    /// Unset the script `id`'s "force async" flag. Not a rendering fact.
    pub fn unforce(&mut self, id: NodeId) {
        if self.force_async(id) {
            self.live.unforced.push(id);
        }
    }

    /// The `<script>` elements in the subtree at `root`, itself included, in
    /// tree order.
    pub fn scripts_in(&self, root: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        self.walk(Some(root), &mut |ev, entry| {
            if let (WalkEvent::Enter(id), NodeKind::Element(e)) = (ev, &entry.kind) {
                if e.name == "script" {
                    out.push(id);
                }
            }
        });
        out
    }

    /// Whether a custom element definition has constructed `id`.
    pub fn custom(&self, id: NodeId) -> bool {
        self.live.custom.contains(&id)
    }

    /// Record that `id`'s definition has constructed it.
    pub fn set_custom(&mut self, id: NodeId) {
        self.live.custom.insert(id);
        self.generation += 1;
    }

    /// The sheets adopted onto `root` (`adoptedStyleSheets`), as text.
    pub fn adopted(&self, root: NodeId) -> &[String] {
        let hit = self.live.adopted.iter().find(|(r, _)| *r == root);
        hit.map_or(&[], |(_, sheets)| sheets)
    }

    /// Replace the sheets adopted onto `root`.
    pub fn set_adopted(&mut self, root: NodeId, sheets: Vec<String>) {
        self.live.adopted.retain(|(r, _)| *r != root);
        self.live.adopted.push((root, sheets));
        self.generation += 1;
    }

    /// Mark the dialog `id` modal or not.
    pub fn set_modal(&mut self, id: NodeId, modal: bool) {
        toggle(&mut self.live.modals, id, modal);
        self.generation += 1;
    }
}

impl Document {
    /// The checkedness or selectedness a script set on `id`, if it did.
    pub fn picked(&self, id: NodeId) -> Option<bool> {
        self.live.picked.iter().find(|p| p.0 == id).map(|p| p.1)
    }

    pub fn set_picked(&mut self, id: NodeId, on: bool) {
        self.live.picked.retain(|p| p.0 != id);
        self.live.picked.push((id, on));
        self.generation += 1;
    }

    /// Whether a script set `indeterminate` on `id`.
    pub fn indeterminate(&self, id: NodeId) -> bool {
        self.live.indeterminate.contains(&id)
    }

    pub fn set_indeterminate(&mut self, id: NodeId, on: bool) {
        toggle(&mut self.live.indeterminate, id, on);
        self.generation += 1;
    }
}

fn toggle(list: &mut Vec<NodeId>, id: NodeId, on: bool) {
    list.retain(|&n| n != id);
    if on {
        list.push(id);
    }
}

pub mod internals;
mod top;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod top_tests;
