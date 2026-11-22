//! Which insertions the tree refuses (`bl-196a`): the DOM's "ensure
//! pre-insertion validity", restricted to the node kinds the arena has.
//!
//! One pure question, [`Document::refusal`], with one caller in the arena —
//! [`Document::insert_child`], which asks it before it relinks anything — and
//! one in the syscall table, for `replaceChild`, whose validity is judged with
//! the node being replaced set aside. The answer is the reason in the words
//! Firefox 153.0esr puts after `Node.appendChild: ` in its
//! `HierarchyRequestError` (measured, identity.md §3.21), so the prelude adds
//! the method name and nothing else.
//!
//! The rules, in the order the binary applies them:
//!
//! 1. Only a document or an element takes children.
//! 2. A node is never inserted under itself or its own descendant — the rule
//!    that keeps the arena a tree, so every walk terminates.
//! 3. A document is nobody's child; a doctype is a child of a document only;
//!    text is not a child of a document.
//! 4. A document holds at most one element and at most one doctype, the
//!    doctype first. A node already there counts: re-appending the document
//!    element is refused, as it is in the binary.
//!
//! A reference sibling that is not a child of the parent is *not* refused
//! (`insert_child` appends, its documented mercy).

use super::{Document, NodeId, NodeKind};

impl Document {
    /// Why `child` may not become a child of `parent` before `before` (`None`
    /// appends), or `None` when it may. `replacing` names a child of `parent`
    /// that the insertion takes the place of: it is not counted, and its
    /// position is the insertion point.
    pub fn refusal(
        &self,
        parent: NodeId,
        child: NodeId,
        before: Option<NodeId>,
        replacing: Option<NodeId>,
    ) -> Option<String> {
        let into = &self.node(parent).kind;
        if !matches!(into, NodeKind::Document | NodeKind::Element(_)) {
            return Some(format!("Cannot add children to a {}", interface(into)));
        }
        if std::iter::successors(Some(parent), |&a| self.node(a).parent).any(|a| a == child) {
            return Some("The new child is an ancestor of the parent".into());
        }
        let kind = &self.node(child).kind;
        let in_document = matches!(into, NodeKind::Document);
        match kind {
            NodeKind::Document => return Some("May not add a Document as a child".into()),
            NodeKind::Doctype if !in_document => return Some(misplaced(kind, into)),
            NodeKind::Text(_) if in_document => return Some(misplaced(kind, into)),
            _ => {}
        }
        if !in_document {
            return None;
        }
        let kids = &self.node(parent).children;
        let at = replacing
            .or(before)
            .and_then(|r| kids.iter().position(|&k| k == r))
            .unwrap_or(kids.len());
        // Whether `range` holds an element (`element`) or a doctype (not).
        let has = |range: &[NodeId], element: bool| {
            range.iter().any(|&k| {
                Some(k) != replacing
                    && match self.node(k).kind {
                        NodeKind::Element(_) => element,
                        NodeKind::Doctype => !element,
                        _ => false,
                    }
            })
        };
        match kind {
            NodeKind::Element(_) if has(kids, true) => {
                Some("Cannot have more than one Element child of a Document".into())
            }
            NodeKind::Element(_) if has(&kids[at..], false) => {
                Some("Cannot insert a root element before the doctype".into())
            }
            NodeKind::Doctype if has(kids, false) => {
                Some("Cannot have more than one DocumentType child of a Document".into())
            }
            NodeKind::Doctype if has(&kids[..at], true) => {
                Some("Cannot have a DocumentType node after the root element".into())
            }
            _ => None,
        }
    }
}

fn misplaced(child: &NodeKind, parent: &NodeKind) -> String {
    let (child, parent) = (interface(child), interface(parent));
    format!(
        "Cannot insert a {child} as a child of a{} {parent}",
        article(parent)
    )
}

/// `a Document`, `an Element`.
fn article(name: &str) -> &'static str {
    if name.starts_with('E') {
        "n"
    } else {
        ""
    }
}

/// The interface name the binary's messages use for a node kind.
fn interface(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Document => "Document",
        NodeKind::Element(_) => "Element",
        NodeKind::Text(_) => "Text",
        NodeKind::Comment(_) => "Comment",
        NodeKind::Doctype => "DocumentType",
    }
}

#[cfg(test)]
#[path = "hierarchy_tests.rs"]
mod tests;
