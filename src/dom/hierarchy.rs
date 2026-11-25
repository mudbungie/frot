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
//! 1. Only a document, an element or a fragment takes children.
//! 2. A node is never inserted under itself or its own descendant — the rule
//!    that keeps the arena a tree, so every walk terminates.
//! 3. A document is nobody's child; a doctype is a child of a document only;
//!    text is not a child of a document.
//! 4. A document holds at most one element and at most one doctype, the
//!    doctype first. A node already there counts: re-appending the document
//!    element is refused, as it is in the binary.
//!
//! Inserting a fragment inserts its children (`bl-de06`), so rules 3 and 4 are
//! asked of them: a fragment holding text, or two elements, is refused by a
//! document as that text or that second element would be.
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
        let in_document = matches!(into, NodeKind::Document);
        if !in_document
            && !matches!(
                into,
                NodeKind::Element(_) | NodeKind::Fragment | NodeKind::Shadow(_)
            )
        {
            return Some(format!("Cannot add children to a {}", interface(into)));
        }
        // Host-including: a host is never put inside its own shadow tree.
        let up = |&a: &NodeId| match self.node(a).kind {
            NodeKind::Shadow(s) => Some(s.host),
            _ => self.node(a).parent,
        };
        if std::iter::successors(Some(parent), up).any(|a| a == child) {
            return Some("The new child is an ancestor of the parent".into());
        }
        let kind = &self.node(child).kind;
        match kind {
            NodeKind::Document => return Some("May not add a Document as a child".into()),
            NodeKind::Doctype(_) if !in_document => return Some(misplaced(kind, into)),
            _ if !in_document => return None,
            _ => {}
        }
        // What the insertion adds to the document: the node, or a fragment's
        // children in its place.
        let single = [child];
        let moved: &[NodeId] = match kind {
            NodeKind::Fragment | NodeKind::Shadow(_) => &self.node(child).children,
            _ => &single,
        };
        let is = |k: NodeId, element: bool| match self.node(k).kind {
            NodeKind::Element(_) => element,
            NodeKind::Doctype(_) => !element,
            _ => false,
        };
        if let Some(&text) = moved
            .iter()
            .find(|&&m| matches!(self.node(m).kind, NodeKind::Text(_)))
        {
            return Some(misplaced(&self.node(text).kind, into));
        }
        let kids = &self.node(parent).children;
        let at = replacing
            .or(before)
            .and_then(|r| kids.iter().position(|&k| k == r))
            .unwrap_or(kids.len());
        // Whether `range` already holds an element (`element`) or a doctype.
        let has = |range: &[NodeId], element: bool| {
            range
                .iter()
                .any(|&k| Some(k) != replacing && is(k, element))
        };
        let elements = moved.iter().filter(|&&m| is(m, true)).count();
        let doctype = moved.iter().any(|&m| is(m, false));
        if elements > 1 || (elements == 1 && has(kids, true)) {
            Some("Cannot have more than one Element child of a Document".into())
        } else if elements == 1 && has(&kids[at..], false) {
            Some("Cannot insert a root element before the doctype".into())
        } else if doctype && has(kids, false) {
            Some("Cannot have more than one DocumentType child of a Document".into())
        } else if doctype && has(&kids[..at], true) {
            Some("Cannot have a DocumentType node after the root element".into())
        } else {
            None
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
        NodeKind::Fragment | NodeKind::Shadow(_) => "DocumentFragment",
        NodeKind::Element(_) => "Element",
        NodeKind::Text(_) => "Text",
        NodeKind::Comment(_) => "Comment",
        NodeKind::Doctype(_) => "DocumentType",
    }
}

#[cfg(test)]
#[path = "hierarchy_tests.rs"]
mod tests;
