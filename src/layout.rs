//! Layout side table (Phase 3) — a [`NodeId`]-keyed table of per-element
//! boxes, mirroring the [`css::Styles`](crate::css::Styles) side-table
//! pattern: a `Vec` parallel to the arena, consulted by geometry-hungry views,
//! never mutating the DOM (`docs/design/layout.md` §1).
//!
//! This module computes **block flow geometry** (design §8.2 subtask 3, §§1–6):
//! normal-flow block boxes are stacked vertically, each the full width of its
//! containing block at its container's left edge, with height flowing bottom-up
//! from content. Inline fragment rects and multi-line wrapping (subtask 3.4)
//! live in the [`inline`] submodule, which this module calls to size a block's
//! inline formatting context; flex is 3.5. The box *kind* is never stored: it is
//! [`Styles::display`](crate::css::Styles::display), the single source of
//! truth (grid/table/… coercion to the seven
//! [`Display`](crate::css::Display) variants already happened in the cascade),
//! and geometry branches on it.
//!
//! frot parses no CSS `width`/`height`/margin/padding (design §6), so in normal
//! flow every block box is the containing-block width at x = its container's
//! left; for the whole in-flow tree that is `w = viewport_w`, `x = 0`. The real
//! work is therefore vertical stacking by content height.

use crate::css::{Display, Styles};
use crate::dom::{Document, NodeId, NodeKind};

mod flex;
mod inline;

/// Viewport width in px — hard-coded, not configurable (design §4). Load-bearing
/// once geometry lands (inline line-breaks and block widths derive from it);
/// the eventual `compute` call site passes it as `viewport_w`. There is no
/// `--viewport` flag (design §4, OQ-2).
pub const VIEWPORT_WIDTH: i32 = 1280;

/// Viewport height in px — the fixed 1280×720 viewport, paired with
/// [`VIEWPORT_WIDTH`]; a constant, not a flag (design §4, OQ-2). Block flow does
/// not consume it (height flows from content), but the JS `env.js` shims report
/// it as `innerHeight`/`outerHeight` and `documentElement.clientHeight`
/// (js.md §7/§8), so pages that read the viewport height get an honest fact.
pub const VIEWPORT_HEIGHT: i32 = 720;

/// One line box's height in px: font-size 16px × line-height 1.25 (design §6).
/// The single line-box unit; the [`inline`] flow pass stacks lines by it and
/// every inline fragment is this tall. A structural estimate, not pixel truth.
const LINE_HEIGHT: i32 = 20;

/// Average glyph advance in px: ~0.5em at the 16px default font-size (design §6).
/// No font is loaded, so a word's width is `glyphs * GLYPH_ADVANCE` and a single
/// inter-word space advances by the same. Used by the [`inline`] flow pass; a
/// deliberate approximation (design §6), consumed only there.
const GLYPH_ADVANCE: i32 = 8;

/// Element tags that generate no visual box; their whole subtree is skipped in
/// the box tree. This is a layout-specific concern, deliberately distinct from
/// the view-specific skip sets in `views::text`/`ax` (which classify
/// differently — e.g. `text` also skips `svg`/`math`, while layout also skips
/// the non-painting document metadata `head`/`title`/`meta`/`link`/`base`).
const NON_RENDERED_TAGS: &[&str] = &[
    "head", "title", "meta", "link", "base", "style", "script", "template", "noscript",
];

/// A box, in px, border-box, viewport coordinates with y increasing downward
/// (design §1). Integer px: sub-pixel precision is meaningless under our
/// approximate metrics, and machine-first output wants no float noise.
/// `Serialize` so the `bboxes` view (3.6) emits it as `{"x","y","w","h"}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    /// The origin-anchored zero-size box — the placeholder every scaffold box
    /// carries until geometry lands (3.3/3.4/3.5).
    pub const ZERO: Rect = Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    };
}

/// Per-node layout, indexed by [`NodeId`] parallel to the arena
/// (`boxes.len() == doc.len()`). A rendered element carries `Some(Rect)`; a
/// `display:none` subtree, a non-rendered element, a non-element node, and a
/// document node carry `None`.
///
/// `orders` is the flex layout's other output (design §5): for every flex
/// container ([`Display::Flex`]/[`Display::InlineFlex`]) it holds that
/// container's in-flow children in reading order; every other node holds `None`.
/// It is the single source for the [`child_order`](Layout::child_order) query.
pub struct Layout {
    boxes: Vec<Option<Rect>>,
    orders: Vec<Option<Vec<NodeId>>>,
}

impl Layout {
    /// The box generated for `id`, or `None` when the node generates none (a
    /// `display:none`/non-rendered/non-element node — see the type docs).
    pub fn rect(&self, id: NodeId) -> Option<Rect> {
        self.boxes[id as usize]
    }

    /// A flex container's in-flow child elements in flex reading order
    /// (design §5): its rendered element children sorted by `(order, source
    /// index)`, reversed for a `row-reverse`/`column-reverse` `flex-direction`.
    ///
    /// Defined only for a flex container (`Display::Flex`/`InlineFlex`); the AX
    /// refinement (3.7) calls it only for such nodes. For **any other id** — a
    /// non-flex element, a `display:none`/non-element/document node — it returns
    /// an empty `Vec` (no stored order).
    pub fn child_order(&self, id: NodeId) -> Vec<NodeId> {
        self.orders[id as usize].clone().unwrap_or_default()
    }
}

/// Compute the layout table. The initial containing block is the viewport
/// (width `viewport_w`, origin `(0, 0)`, design §4); [`Document::roots`] are its
/// in-flow block children, stacked from `y = 0`. Every rendered block-level
/// element gets real geometry ([`place`]/[`layout_block`]); every other
/// rendered element keeps a placeholder [`Rect::ZERO`] (inline fragments are
/// subtask 3.4, flex 3.5); non-rendered/`display:none`/non-element nodes get
/// `None`.
pub fn compute(doc: &Document, styles: &Styles, viewport_w: i32) -> Layout {
    let mut boxes = vec![None; doc.len()];
    let orders = flex::child_orders(doc, styles);
    let mut cursor = 0;
    for &root in doc.roots() {
        cursor += place(doc, root, styles, 0, cursor, viewport_w, &mut boxes);
    }
    Layout { boxes, orders }
}

/// Place node `id` as an in-flow child of a block container whose content box
/// begins at `(x, y)` with width `w`, returning the block-flow height it
/// contributes. A rendered **block-level** child lays out via [`layout_block`]
/// (its height); a rendered **flex container** (`display:flex`) via
/// [`flex::place_container`] (design §5/§6). Any other node — non-element,
/// non-rendered, `display:none`, or an inline-level box (inline/inline-block/
/// inline-flex; §1 anonymous-box promotion is "transient", out of scope here) —
/// contributes `0`; a rendered inline-level element still gets its scaffold
/// [`Rect::ZERO`] subtree via [`walk`].
fn place(
    doc: &Document,
    id: NodeId,
    styles: &Styles,
    x: i32,
    y: i32,
    w: i32,
    boxes: &mut [Option<Rect>],
) -> i32 {
    if !is_rendered_element(doc, id, styles) {
        return 0;
    }
    match styles.display(id) {
        Display::Block | Display::ListItem => layout_block(doc, id, styles, x, y, w, boxes),
        Display::Flex => flex::place_container(doc, id, styles, x, y, w, boxes),
        _ => {
            walk(doc, id, styles, boxes);
            0
        }
    }
}

/// Lay out block box `id` at `(x, y)` with width `w`, store its rect, and
/// return its content height. A container **with block-level children** stacks
/// them: each at `x`/`w` and the running cursor, height = the cursor advance
/// (sum of child heights). A container establishing an **inline formatting
/// context** (no block-level child box) takes its height from [`inline::flow`],
/// which also fills its inline elements' fragment-union rects; inline elements
/// with no rendered word, and text nodes, keep the [`Rect::ZERO`] seeded by
/// [`walk`].
fn layout_block(
    doc: &Document,
    id: NodeId,
    styles: &Styles,
    x: i32,
    y: i32,
    w: i32,
    boxes: &mut [Option<Rect>],
) -> i32 {
    let entry = doc.node(id);
    let has_block = entry.children.iter().any(|&c| is_block_box(doc, c, styles));
    let h = if has_block {
        let mut cursor = y;
        for &c in &entry.children {
            cursor += place(doc, c, styles, x, cursor, w, boxes);
        }
        cursor - y
    } else {
        for &c in &entry.children {
            walk(doc, c, styles, boxes);
        }
        inline::flow(doc, styles, id, y, w, boxes)
    };
    boxes[id as usize] = Some(Rect { x, y, w, h });
    h
}

/// Whether `id` is a rendered block-level box: a rendered element whose computed
/// display is `Block`, `ListItem`, or `Flex` (a flex container is block-level and
/// establishes its own formatting context, so it stacks like a block box).
fn is_block_box(doc: &Document, id: NodeId, styles: &Styles) -> bool {
    is_rendered_element(doc, id, styles)
        && matches!(
            styles.display(id),
            Display::Block | Display::ListItem | Display::Flex
        )
}

/// Whether `id` is an element that generates a box: not `display:none` and not
/// a [`NON_RENDERED_TAGS`] element. The shared render-tree predicate.
fn is_rendered_element(doc: &Document, id: NodeId, styles: &Styles) -> bool {
    let NodeKind::Element(el) = &doc.node(id).kind else {
        return false;
    };
    !styles.display_none(id) && !NON_RENDERED_TAGS.contains(&el.name.as_str())
}

/// Give rendered element `id` a placeholder [`Rect::ZERO`] and recurse. Used for
/// non-block subtrees whose geometry is filled by 3.4/3.5; skipped elements'
/// subtrees are not walked, so they stay `None`.
fn walk(doc: &Document, id: NodeId, styles: &Styles, boxes: &mut [Option<Rect>]) {
    if !is_rendered_element(doc, id, styles) {
        return;
    }
    boxes[id as usize] = Some(Rect::ZERO);
    for &c in &doc.node(id).children {
        walk(doc, c, styles, boxes);
    }
}

#[cfg(test)]
mod tests;
