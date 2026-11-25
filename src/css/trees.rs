//! Style scoping by tree (js.md §15): the document's sheets style the
//! document tree, and each shadow root's — its own `<style>` elements and the
//! sheets a script adopted onto it — style its tree, its host through `:host`
//! and the elements slotted into it through `::slotted()`. One [`Index`] per
//! tree; a page with no shadow root has the document's alone.

use std::collections::HashMap;

use super::index::Index;
use super::parse::Stylesheet;
use crate::dom::{Document, NodeId, NodeKind, WalkEvent};

/// Each connected shadow root's own sheets, in tree order then adoption
/// order: what [`Trees::build`] indexes.
pub fn gather(doc: &Document) -> Vec<(NodeId, Vec<Stylesheet>)> {
    let sheets = |root: NodeId| {
        let mut styles = Vec::new();
        doc.walk(Some(root), &mut |ev, e| {
            if let (WalkEvent::Enter(n), NodeKind::Element(el)) = (ev, &e.kind) {
                if el.name == "style" {
                    styles.push(n);
                }
            }
        });
        let own = styles.into_iter().map(|n| doc.text_content(n));
        own.chain(doc.adopted(root).iter().cloned())
            .map(|t| Stylesheet::parse(&t))
            .collect()
    };
    doc.shadow_roots()
        .into_iter()
        .map(|r| (r, sheets(r)))
        .collect()
}

/// The rules of every tree, indexed.
pub struct Trees<'a> {
    document: Index<'a>,
    shadow: HashMap<NodeId, Index<'a>>,
}

impl<'a> Trees<'a> {
    pub fn build(document: &'a [Stylesheet], shadow: &'a [(NodeId, Vec<Stylesheet>)]) -> Self {
        Trees {
            document: Index::build(document),
            shadow: shadow.iter().map(|(r, s)| (*r, Index::build(s))).collect(),
        }
    }

    /// The rules of the tree whose root is `root`: the document's for the
    /// document or a detached subtree, a shadow root's own, else none.
    pub fn of(&self, doc: &Document, root: NodeId) -> Option<&Index<'a>> {
        match doc.shadow(root) {
            Some(_) => self.shadow.get(&root),
            None => Some(&self.document),
        }
    }

    /// Total rules of the document's sheets — inline `style=`'s source-order
    /// slot ([`Index::rule_count`]).
    pub fn rule_count(&self) -> usize {
        self.document.rule_count
    }
}
