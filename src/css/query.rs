//! `querySelector` / `querySelectorAll` over the arena (js.md §3).
//!
//! The `--js` syscall layer routes selector matching here so the CSS subset
//! engine is the single source of selector semantics for both the cascade and
//! `querySelector`. Matching reuses [`super::selector::matches`] with the same
//! nearest-first ancestor chain the cascade builds; an unsupported selector is
//! an [`UnsupportedSelector`] error (the caller throws), never a silent miss.

use super::selector::{matches, Selector};
use super::selparse::parse_query;
use crate::dom::{Document, Element, NodeId, NodeKind};

/// The selector list used grammar the css subset engine cannot evaluate
/// (js.md §3 — a counted error, not a silent empty match).
#[derive(Debug, PartialEq, Eq)]
pub struct UnsupportedSelector;

/// Elements matching `list`, in document order. `root` scopes the result:
/// `None` matches over the whole document (every element); `Some(id)` matches
/// only strict descendants of `id` (as `element.querySelectorAll` does).
/// Ancestor context is always the full chain from the document roots, so
/// descendant/child combinators reaching above `root` still resolve.
pub fn query_all(
    doc: &Document,
    root: Option<NodeId>,
    list: &str,
) -> Result<Vec<NodeId>, UnsupportedSelector> {
    let sels = parse_query(list).ok_or(UnsupportedSelector)?;
    let mut out = Vec::new();
    let mut ancestors: Vec<&Element> = Vec::new();
    for &r in doc.roots() {
        walk(doc, r, root, root.is_none(), &mut ancestors, &sels, &mut out);
    }
    Ok(out)
}

/// Recurse `id`'s subtree with `ancestors` (outer-first) accumulated from the
/// document roots. `collecting` says whether elements at this depth are in
/// scope: it flips true at `root`'s children (`Some` scope) or is true
/// throughout (`None` scope, whole-document query). Non-element nodes are arena
/// leaves (only elements carry children), so they neither match nor recurse.
fn walk<'a>(
    doc: &'a Document,
    id: NodeId,
    root: Option<NodeId>,
    collecting: bool,
    ancestors: &mut Vec<&'a Element>,
    sels: &[Selector],
    out: &mut Vec<NodeId>,
) {
    let NodeKind::Element(el) = &doc.node(id).kind else {
        return;
    };
    if collecting {
        let near: Vec<&Element> = ancestors.iter().rev().copied().collect();
        if sels.iter().any(|s| matches(s, el, &near)) {
            out.push(id);
        }
    }
    let child_collecting = collecting || root == Some(id);
    ancestors.push(el);
    for &c in &doc.node(id).children {
        walk(doc, c, root, child_collecting, ancestors, sels, out);
    }
    ancestors.pop();
}

#[cfg(test)]
mod tests;
