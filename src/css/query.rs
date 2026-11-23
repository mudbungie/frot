//! `querySelector` / `querySelectorAll` over the arena (js.md §3).
//!
//! The `--js` syscall layer routes selector matching here so the CSS subset
//! engine is the single source of selector semantics for both the cascade and
//! `querySelector`. Matching is [`super::selector::matches`], the call the
//! cascade makes; an unsupported selector is an [`UnsupportedSelector`] error
//! (the caller throws), never a silent miss.

use super::selector::{matches_in, Context, Selector};
use super::selparse::parse_query;
use crate::dom::{Document, NodeId};

/// The selector list used grammar the css subset engine cannot evaluate
/// (js.md §3 — a counted error, not a silent empty match).
#[derive(Debug, PartialEq, Eq)]
pub struct UnsupportedSelector;

/// Elements matching `list` among the strict descendants of `root`, in tree
/// order: `root` is the document node for a whole-document query, an element
/// for `element.querySelectorAll`. A selector is matched against the element
/// where it stands in its tree, so combinators reaching above or beside
/// `root` still resolve — and a subtree that is in no document is queried like
/// any other. `:scope` is `root` when it is an element, else the root element.
pub fn query_all(
    doc: &Document,
    root: NodeId,
    list: &str,
) -> Result<Vec<NodeId>, UnsupportedSelector> {
    let sels = parse_query(list).ok_or(UnsupportedSelector)?;
    let scope = (root != Document::ROOT).then_some(root);
    let mut out = Vec::new();
    walk(Context { doc, scope }, root, &sels, &mut out);
    Ok(out)
}

/// Whether the node `id` itself matches `list` where it stands
/// (`element.matches`), in a document or out of one. `:scope` is `id`.
pub fn matches_any(doc: &Document, id: NodeId, list: &str) -> Result<bool, UnsupportedSelector> {
    let sels = parse_query(list).ok_or(UnsupportedSelector)?;
    let cx = Context {
        doc,
        scope: Some(id),
    };
    Ok(sels.iter().any(|s| matches_in(s, cx, id)))
}

/// Collect the matches below `id`. Only an element matches ([`matches`]), and
/// only the document and elements carry children.
fn walk(cx: Context, id: NodeId, sels: &[Selector], out: &mut Vec<NodeId>) {
    for &c in &cx.doc.node(id).children {
        if sels.iter().any(|s| matches_in(s, cx, c)) {
            out.push(c);
        }
        walk(cx, c, sels, out);
    }
}

#[cfg(test)]
mod tests;
