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

use crate::dom::{Document, NodeId, NodeKind};

pub use cascade::{compute, compute_with};

/// Concatenated text of `id`'s subtree as the accessible-name algorithm sees
/// it under `--css`: `display:none` subtrees are dropped and `::before` /
/// `::after` generated content is included. With no rule affecting the
/// subtree this is exactly [`Document::text_content`].
pub fn rendered_subtree_text(doc: &Document, id: NodeId, styles: &Styles) -> String {
    let mut out = String::new();
    collect_text(doc, id, styles, &mut out);
    out
}

fn collect_text(doc: &Document, id: NodeId, styles: &Styles, out: &mut String) {
    let entry = doc.node(id);
    match &entry.kind {
        NodeKind::Element(_) => {
            if styles.display_none(id) {
                return;
            }
            if let Some(b) = styles.before(id) {
                out.push_str(b);
            }
            for &c in &entry.children {
                collect_text(doc, c, styles, out);
            }
            if let Some(a) = styles.after(id) {
                out.push_str(a);
            }
        }
        NodeKind::Text(t) => out.push_str(t),
        NodeKind::Comment(_) | NodeKind::Doctype => {}
    }
}

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
