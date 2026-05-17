//! CSS application (`--css`): a small, dependency-free engine that parses
//! author CSS, matches a selector subset, cascades it, and exposes a
//! per-node [`Styles`] side table.
//!
//! Scope is deliberately narrow — only what visibility and generated content
//! need: `display:none`, `visibility:hidden|collapse` (inherited), and
//! `::before`/`::after` `content`. There is no layout, no media-query
//! evaluation, and no `cssparser`/`selectors` dependency; see `VISION.md`.

mod cascade;
mod parse;
mod selector;
mod selparse;

use crate::dom::NodeId;

pub use cascade::compute;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Visible,
    Hidden,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComputedStyle {
    /// `display:none` — the element and its subtree are not rendered.
    pub display_none: bool,
    /// Computed (inherited) visibility.
    pub visibility: Visibility,
    /// `::before` generated content, if a generated box is produced.
    pub before: Option<String>,
    /// `::after` generated content, if a generated box is produced.
    pub after: Option<String>,
}

/// Per-node computed styles, indexed by [`NodeId`]. Non-element nodes and
/// elements no rule touched carry [`ComputedStyle::default`] (rendered,
/// visible, no generated content).
#[derive(Debug, Clone)]
pub struct Styles {
    nodes: Vec<ComputedStyle>,
}

impl Styles {
    fn from_nodes(nodes: Vec<ComputedStyle>) -> Self {
        Self { nodes }
    }

    pub fn get(&self, id: NodeId) -> &ComputedStyle {
        &self.nodes[id as usize]
    }

    /// Whether `id`'s subtree is removed from rendered output.
    pub fn display_none(&self, id: NodeId) -> bool {
        self.get(id).display_none
    }

    pub fn visibility(&self, id: NodeId) -> Visibility {
        self.get(id).visibility
    }

    pub fn before(&self, id: NodeId) -> Option<&str> {
        self.get(id).before.as_deref()
    }

    pub fn after(&self, id: NodeId) -> Option<&str> {
        self.get(id).after.as_deref()
    }
}

#[cfg(test)]
mod tests;
