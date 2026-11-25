//! Whether an element is rendered, asked of one element at a time
//! (`bl-4248`): the cascade resolved down its flat ancestor chain only.

use super::{conceal, gather, resolve, trees, Walk};
use crate::css::trees::Trees;
use crate::css::{ComputedStyle, Display, Visibility};
use crate::dom::{Document, NodeId};

/// The first of `ids` that is rendered and visible — no `display:none` on it
/// or any flat ancestor, its own `visibility` not `hidden`, and in the flat
/// tree at all — under the author sheets (`Some(external)`, `--css`) or none
/// (`None`, bare). An on-demand query (`bl-4248`): each candidate resolves
/// only its own ancestor chain, so asking costs no full cascade.
pub fn first_rendered(
    doc: &Document,
    external: Option<&[String]>,
    js: bool,
    ids: impl IntoIterator<Item = NodeId>,
) -> Option<NodeId> {
    let sheets = external.map_or_else(Vec::new, |ext| gather(doc, ext));
    let shadow = external.map_or_else(Vec::new, |_| trees::gather(doc));
    let trees = Trees::build(&sheets, &shadow);
    let cx = Walk {
        doc,
        trees: &trees,
        js,
    };
    ids.into_iter().find(|&id| rendered(cx, id))
}

/// [`first_rendered`] for one element: resolve its flat ancestor chain top
/// down, as the [`walk`] would.
fn rendered(cx: Walk, id: NodeId) -> bool {
    let doc = cx.doc;
    let chain: Vec<NodeId> = std::iter::successors(Some(id), |&n| doc.flat_parent(n)).collect();
    if chain.last() != Some(&Document::ROOT) {
        return false;
    }
    let mut style = ComputedStyle::default();
    // Below the document, a flat chain is elements only.
    let elements = chain
        .iter()
        .rev()
        .filter_map(|&n| doc.element(n).map(|e| (n, e)));
    for (n, el) in elements {
        let concealed = doc.concealed(n) || style.display == Display::TableColumn;
        style = conceal(resolve(cx, n, el, doc.tree_root(n), &style), concealed);
        if style.display == Display::None {
            return false;
        }
    }
    style.visibility == Visibility::Visible
}
