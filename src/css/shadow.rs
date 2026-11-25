//! Selectors across a shadow boundary (js.md §15): `:host`, `:host(…)` and
//! `::slotted(…)`, the three ways a shadow tree's stylesheet reaches out of
//! its tree. A selector is matched in a [`Context`] that names the host of
//! the tree its sheet belongs to (`None` for the document's sheets and for
//! `querySelector`), and, when the cascade asks about a slotted element, that
//! element.
//!
//! - In its own shadow tree's context the host is *featureless*: only a
//!   compound of `:host` pseudo-classes matches it, and it is reached by a
//!   combinator climbing out of the tree — `:host(.dark) p`.
//! - `::slotted(X)` is matched on the slot an element is assigned to, with
//!   `X` matched on the element itself.

use super::selector::{matches_in, Compound, Context, Selector, Simple};
use crate::dom::{Document, NodeId, NodeKind};

impl<'a> Context<'a> {
    /// A match in `doc` outside any shadow tree's sheet: no `:scope` element,
    /// no host, no slotted element.
    pub fn of(doc: &'a Document) -> Self {
        Context {
            doc,
            scope: None,
            host: None,
            slotted: None,
        }
    }
}

/// `:host` / `:host(list)` on `id`: `id` is the context's host, and matches
/// `list` (if given) where it stands in its own tree.
pub(super) fn host(cx: Context, id: NodeId, list: &[Selector]) -> bool {
    let outside = Context { host: None, ..cx };
    cx.host == Some(id) && (list.is_empty() || list.iter().any(|s| matches_in(s, outside, id)))
}

/// `::slotted(list)` on the slot `id`: the element the cascade asks about is
/// assigned to it and matches `list`.
pub(super) fn slotted(cx: Context, id: NodeId, list: &[Selector]) -> bool {
    let outside = Context {
        host: None,
        slotted: None,
        ..cx
    };
    cx.slotted.is_some_and(|e| {
        cx.doc.assigned_slot(e) == Some(id) && list.iter().any(|s| matches_in(s, outside, e))
    })
}

/// Whether `c` cannot match `id` because `id` is the context's featureless
/// host and `c` is not made of `:host` pseudo-classes alone.
pub(super) fn featureless(cx: Context, id: NodeId, c: &Compound) -> bool {
    cx.host == Some(id) && !c.simples.iter().all(|s| matches!(s, Simple::Host(_)))
}

/// `id`'s element ancestors, nearest first — in a shadow tree's context going
/// on from the tree's top to its host, and no further.
pub(super) fn ancestors(cx: Context<'_>, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
    let doc = cx.doc;
    let up = move |&n: &NodeId| match doc.node(n).kind {
        NodeKind::Shadow(s) if cx.host == Some(s.host) => Some(s.host),
        _ if Some(n) == cx.host => None,
        _ => doc.node(n).parent,
    };
    std::iter::successors(doc.node(id).parent, up)
        .filter(move |&n| matches!(doc.node(n).kind, NodeKind::Element(_)))
}

#[cfg(test)]
mod tests;
