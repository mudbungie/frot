//! Basic single-line flex (Phase 3 subtask 3.5) — the reading-order query plus
//! crude main-axis placement (`layout.md` §5/§6).
//!
//! Two outputs, kept deliberately apart:
//!
//! - **[`child_order`] is exact and deterministic** (design §5): a flex
//!   container's in-flow child elements sorted by `(order value, source index)`,
//!   then reversed when `flex-direction` is `row-reverse`/`column-reverse`. It is
//!   the whole reorder surface in Phase 3 and feeds the AX refinement (3.7).
//!   [`child_orders`] precomputes it for every flex container into the layout's
//!   `orders` side table.
//! - **Placement is crude** (design §6: "crude main-axis placement, not exact
//!   item sizing"). A `row`/`row-reverse` container lays its items left-to-right,
//!   each sized to a crude intrinsic content width (its unwrapped word run, via
//!   [`inline::max_content_width`]); a `column`/`column-reverse` container stacks
//!   them full-width top-to-bottom. Each item's height and subtree come from the
//!   ordinary block layout ([`layout_block`]) at its assigned width — flex items
//!   are blockified. **These rects are structural estimates, not pixel truth.**
//!
//! Out of scope (design §6): `flex-wrap` (single line only), `flex-grow`/
//! `shrink`/`basis`, `justify-content`/`align-*`, and `gap`. A nested flex item
//! lays out as a plain block (its own `child_order` is still computed).

use super::inline;
use super::{is_rendered_element, layout_block, Rect};
use crate::css::{Display, FlexDirection, Styles};
use crate::dom::{Document, NodeId};

/// Precompute [`child_order`] for every flex container into a table parallel to
/// the arena: a rendered `Display::Flex`/`InlineFlex` element gets `Some(order)`,
/// every other node `None`. This is the layout's `orders` output.
pub(super) fn child_orders(doc: &Document, styles: &Styles) -> Vec<Option<Vec<NodeId>>> {
    (0..doc.len() as NodeId)
        .map(|id| {
            let flex = is_rendered_element(doc, id, styles)
                && matches!(styles.display(id), Display::Flex | Display::InlineFlex);
            flex.then(|| child_order(doc, styles, id))
        })
        .collect()
}

/// Flex container `id`'s in-flow child elements in reading order (design §5):
/// its rendered element children (`display:none`/non-rendered and every
/// non-element child excluded) sorted by `(order, source index)` — a stable sort
/// by `order` keeps equal-`order` items in source order — then reversed for a
/// `row-reverse`/`column-reverse` `flex-direction`.
pub(super) fn child_order(doc: &Document, styles: &Styles, id: NodeId) -> Vec<NodeId> {
    let mut items: Vec<NodeId> = doc
        .node(id)
        .children
        .iter()
        .copied()
        .filter(|&c| is_rendered_element(doc, c, styles))
        .collect();
    items.sort_by_key(|&c| styles.order(c));
    if matches!(
        styles.flex_direction(id),
        FlexDirection::RowReverse | FlexDirection::ColumnReverse
    ) {
        items.reverse();
    }
    items
}

/// Lay out flex container `id` at `(x, y)` with main-axis extent `w`, store its
/// rect, and return its content height. Items are placed in [`child_order`]
/// sequence: `row`/`row-reverse` left-to-right at crude intrinsic widths (height
/// = tallest item); `column`/`column-reverse` stacked full-width (height = sum).
pub(super) fn place_container(
    doc: &Document,
    id: NodeId,
    styles: &Styles,
    x: i32,
    y: i32,
    w: i32,
    boxes: &mut [Option<Rect>],
) -> i32 {
    let items = child_order(doc, styles, id);
    let h = match styles.flex_direction(id) {
        FlexDirection::Column | FlexDirection::ColumnReverse => {
            column(doc, styles, &items, x, y, w, boxes)
        }
        FlexDirection::Row | FlexDirection::RowReverse => row(doc, styles, &items, x, y, boxes),
    };
    boxes[id as usize] = Some(Rect { x, y, w, h });
    h
}

/// Row main axis: place each item left-to-right from `x`, width = its crude
/// intrinsic content width, height from [`layout_block`] at that width. Returns
/// the tallest item height (the container's cross-axis extent).
fn row(
    doc: &Document,
    styles: &Styles,
    items: &[NodeId],
    x: i32,
    y: i32,
    boxes: &mut [Option<Rect>],
) -> i32 {
    let mut cx = x;
    let mut max_h = 0;
    for &item in items {
        let iw = inline::max_content_width(doc, styles, item);
        let ih = layout_block(doc, item, styles, cx, y, iw, boxes);
        cx += iw;
        max_h = max_h.max(ih);
    }
    max_h
}

/// Column main axis: stack each item top-to-bottom from `y`, each the full
/// container width `w`, height from [`layout_block`]. Returns the summed height.
fn column(
    doc: &Document,
    styles: &Styles,
    items: &[NodeId],
    x: i32,
    y: i32,
    w: i32,
    boxes: &mut [Option<Rect>],
) -> i32 {
    let mut cy = y;
    for &item in items {
        cy += layout_block(doc, item, styles, x, cy, w, boxes);
    }
    cy - y
}

#[cfg(test)]
mod tests;
