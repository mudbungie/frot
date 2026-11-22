//! `querySelector` / `querySelectorAll` over the arena (js.md §3).
//!
//! The `--js` syscall layer routes selector matching here so the CSS subset
//! engine is the single source of selector semantics for both the cascade and
//! `querySelector`. Matching is [`super::selector::matches`], the call the
//! cascade makes; an unsupported selector is an [`UnsupportedSelector`] error
//! (the caller throws), never a silent miss.

use super::selector::{matches, Selector};
use super::selparse::parse_query;
use crate::dom::{Document, NodeId};

/// The selector list used grammar the css subset engine cannot evaluate
/// (js.md §3 — a counted error, not a silent empty match).
#[derive(Debug, PartialEq, Eq)]
pub struct UnsupportedSelector;

/// Elements matching `list`, in document order. `root` scopes the result:
/// `None` matches over the whole document (every element); `Some(id)` matches
/// only strict descendants of `id` (as `element.querySelectorAll` does).
/// A selector is matched against the element where it stands in the document,
/// so combinators reaching above or beside `root` still resolve.
pub fn query_all(
    doc: &Document,
    root: Option<NodeId>,
    list: &str,
) -> Result<Vec<NodeId>, UnsupportedSelector> {
    let sels = parse_query(list).ok_or(UnsupportedSelector)?;
    let mut out = Vec::new();
    for &r in doc.roots() {
        walk(doc, r, root, root.is_none(), &sels, &mut out);
    }
    Ok(out)
}

/// Recurse `id`'s subtree. `collecting` says whether nodes at this depth are in
/// scope: it flips true at `root`'s children (`Some` scope) or is true
/// throughout (`None` scope, whole-document query). Only an element matches
/// ([`matches`]), and only elements carry children.
fn walk(
    doc: &Document,
    id: NodeId,
    root: Option<NodeId>,
    collecting: bool,
    sels: &[Selector],
    out: &mut Vec<NodeId>,
) {
    if collecting && sels.iter().any(|s| matches(s, doc, id)) {
        out.push(id);
    }
    let child_collecting = collecting || root == Some(id);
    for &c in &doc.node(id).children {
        walk(doc, c, root, child_collecting, sels, out);
    }
}

#[cfg(test)]
mod tests;
