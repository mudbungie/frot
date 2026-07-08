//! `--out bboxes`: a flat, reading-order array of the boxes the layout engine
//! generated — geometry + order, not structure (`layout.md` §7).
//!
//! One entry per **rendered** element (`Layout::rect` is `Some`; `display:none`
//! and non-rendered subtrees are box-less and omitted, `visibility:hidden`
//! elements keep their box and are included):
//!
//! ```json
//! { "i": 12, "tag": "h1", "rect": { "x": 0, "y": 0, "w": 1280, "h": 20 },
//!   "text": "Welcome" }
//! ```
//!
//! - **`i`** — the element's source-order index: its 0-based ordinal in a
//!   document pre-order walk of elements. A reproducible, view-agnostic handle
//!   back to `dom`/`ax`; the arena `NodeId` is an internal detail and never
//!   exposed. Because the array is in *reading* order (a flex container's
//!   children come out in `Layout::child_order`, so `order`/`*-reverse` shuffles
//!   them), `i` values are not monotonic — that is intended.
//! - **`tag`** — the element's tag name.
//! - **`rect`** — `Layout::rect(id)` (`layout.md` §4: px, 1280 viewport,
//!   structural estimates).
//! - **`text`** — the element's own **direct** text (immediate text-node
//!   children only, whitespace-normalized), or `null` when it has none.

use crate::css::{Display, Styles};
use crate::dom::{Document, NodeEntry, NodeId, NodeKind, WalkEvent};
use crate::layout::Layout;
use serde_json::{json, Value};

/// Build the reading-order box array. `layout`/`styles` are the same tables the
/// engine computed; `styles` decides which containers reorder their children
/// (`Display::Flex`/`InlineFlex` → [`Layout::child_order`], else source order).
pub fn bboxes(doc: &Document, layout: &Layout, styles: &Styles) -> Value {
    let ordinals = source_ordinals(doc);
    let mut out = Vec::new();
    for &root in doc.roots() {
        emit(doc, root, layout, styles, &ordinals, &mut out);
    }
    Value::Array(out)
}

/// Each element's source-order index, keyed by [`NodeId`] parallel to the arena:
/// the 0-based ordinal at which a document pre-order walk enters that element.
/// Non-element nodes are never looked up (only rendered elements are emitted),
/// so their slot stays `0`.
fn source_ordinals(doc: &Document) -> Vec<usize> {
    let mut ord = vec![0usize; doc.len()];
    let mut next = 0usize;
    doc.walk(None, &mut |ev, entry| {
        if let (WalkEvent::Enter(id), NodeKind::Element(_)) = (ev, &entry.kind) {
            ord[id as usize] = next;
            next += 1;
        }
    });
    ord
}

/// Emit `id` (when it generates a box) then recurse in reading order. A
/// non-element node is skipped; a box-less element — `display:none` or a
/// non-rendered tag — has `rect(id) == None` and an all-box-less subtree, so it
/// and its whole branch are pruned. A rendered flex container recurses via
/// [`Layout::child_order`] (reading order); every other rendered element
/// recurses over its source-order children.
fn emit(
    doc: &Document,
    id: NodeId,
    layout: &Layout,
    styles: &Styles,
    ordinals: &[usize],
    out: &mut Vec<Value>,
) {
    let entry = doc.node(id);
    let NodeKind::Element(el) = &entry.kind else {
        return;
    };
    let Some(rect) = layout.rect(id) else {
        return;
    };
    out.push(json!({
        "i": ordinals[id as usize],
        "tag": el.name,
        "rect": rect,
        "text": direct_text(doc, entry),
    }));
    let children = if matches!(styles.display(id), Display::Flex | Display::InlineFlex) {
        layout.child_order(id)
    } else {
        entry.children.clone()
    };
    for c in children {
        emit(doc, c, layout, styles, ordinals, out);
    }
}

/// The element's own direct text: its immediate text-node children concatenated
/// and whitespace-normalized (runs collapse to one space, ends trimmed), or
/// [`Value::Null`] when that yields nothing. Descendant text belongs to the
/// descendants' own entries, so only direct children count.
fn direct_text(doc: &Document, entry: &NodeEntry) -> Value {
    let mut buf = String::new();
    for &c in &entry.children {
        if let NodeKind::Text(t) = &doc.node(c).kind {
            buf.push_str(t);
        }
    }
    let norm = buf.split_whitespace().collect::<Vec<_>>().join(" ");
    if norm.is_empty() {
        Value::Null
    } else {
        Value::String(norm)
    }
}

#[cfg(test)]
mod tests;
