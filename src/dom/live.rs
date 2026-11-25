//! Page state a script sets without leaving a mark in the tree (`bl-bc6d`):
//! where focus is, which popovers and modal dialogs are shown, and which
//! elements a custom element definition has upgraded (js.md §15). It lives
//! in the arena beside the nodes, so the one document the views, the cascade
//! and the JS host share carries it — nothing global, nothing cached in JS.
//! The JS host writes it (`focus()`, `blur()`, `showModal()`, `close()`,
//! `showPopover()`, `hidePopover()`); the selector engine and
//! `document.activeElement` read it.

use std::collections::BTreeSet;

use super::{Document, NodeId};

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
    /// Elements whose custom element state is "custom": constructed by
    /// their definition (`:defined`).
    custom: BTreeSet<NodeId>,
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

    /// Whether a custom element definition has constructed `id`.
    pub fn custom(&self, id: NodeId) -> bool {
        self.live.custom.contains(&id)
    }

    /// Record that `id`'s definition has constructed it.
    pub fn set_custom(&mut self, id: NodeId) {
        self.live.custom.insert(id);
        self.generation += 1;
    }

    /// Mark the dialog `id` modal or not.
    pub fn set_modal(&mut self, id: NodeId, modal: bool) {
        toggle(&mut self.live.modals, id, modal);
        self.generation += 1;
    }
}

fn toggle(list: &mut Vec<NodeId>, id: NodeId, on: bool) {
    list.retain(|&n| n != id);
    if on {
        list.push(id);
    }
}

#[cfg(test)]
mod tests;
