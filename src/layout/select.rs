//! Select modes — the one layout fact a `<select>` carries (`layout.md` §2.4,
//! `bl-2932`).
//!
//! A `<select>` is either a **list box**, which lays its contents out in flow
//! like any block, or a **menu list** (the collapsed drop-down), which paints
//! only its own box: the popup holding the options is UA widgetry, so nothing
//! inside the element generates a box. The contents stay in `text` and in the
//! accessibility tree either way — `innerText` lists every option and so does
//! Chrome's AX tree — which is why this lives in layout and not beside
//! [`Document::unpainted`], whose readers include both of those.
//!
//! Which mode a select is in is [`Element::list_box`], shared with the
//! accessibility role (`listbox` or `combobox`), so the two cannot disagree.

use crate::dom::{Document, NodeId, NodeKind};

/// Whether `id` is a child of a menu-list `<select>` and so generates no box.
/// Asked of the child only: every layout walk is top-down and stops at a node
/// that does not render, so nothing deeper is reached.
pub(super) fn in_menu_list(doc: &Document, id: NodeId) -> bool {
    doc.node(id).parent.is_some_and(|parent| {
        matches!(&doc.node(parent).kind,
            NodeKind::Element(el) if el.name == "select" && !el.list_box())
    })
}

#[cfg(test)]
#[path = "select_tests.rs"]
mod tests;
