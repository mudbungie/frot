//! Layout side table (Phase 3) — a [`NodeId`]-keyed table of per-element
//! boxes, mirroring the [`css::Styles`](crate::css::Styles) side-table
//! pattern: a `Vec` parallel to the arena, consulted by geometry-hungry views,
//! never mutating the DOM (`docs/design/layout.md` §1).
//!
//! This module is the box-tree *scaffold* (design §8.2 subtask 2): it derives
//! which elements generate a box and stores placeholder rects. Real geometry
//! is filled by later subtasks — block flow (3.3), inline flow (3.4), flex
//! (3.5). The box *kind* is never stored: it is
//! [`Styles::display`](crate::css::Styles::display), the single source of
//! truth (grid/table/… coercion to the seven
//! [`Display`](crate::css::Display) variants already happened in the cascade),
//! and later geometry subtasks branch on it.

use crate::css::Styles;
use crate::dom::{Document, NodeId, NodeKind};

/// Viewport width in px — hard-coded, not configurable (design §4). Load-bearing
/// once geometry lands (inline line-breaks and block widths derive from it);
/// the eventual `compute` call site passes it as `viewport_w`. There is no
/// `--viewport` flag (design §4, OQ-2).
pub const VIEWPORT_WIDTH: i32 = 1280;

/// Element tags that generate no visual box; their whole subtree is skipped in
/// the box tree. This is a layout-specific concern, deliberately distinct from
/// the view-specific skip sets in `views::text`/`ax` (which classify
/// differently — e.g. `text` also skips `svg`/`math`, while layout also skips
/// the non-painting document metadata `head`/`title`/`meta`/`link`/`base`).
const NON_RENDERED_TAGS: &[&str] = &[
    "head", "title", "meta", "link", "base", "style", "script", "template",
    "noscript",
];

/// A box, in px, border-box, viewport coordinates with y increasing downward
/// (design §1). Integer px: sub-pixel precision is meaningless under our
/// approximate metrics, and machine-first output wants no float noise. Not
/// `Serialize` yet — nothing serializes a `Rect` until the `bboxes` view (3.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    /// The origin-anchored zero-size box — the placeholder every scaffold box
    /// carries until geometry lands (3.3/3.4/3.5).
    pub const ZERO: Rect = Rect { x: 0, y: 0, w: 0, h: 0 };
}

/// Per-node layout, indexed by [`NodeId`] parallel to the arena
/// (`boxes.len() == doc.len()`). A rendered element carries `Some(Rect)`; a
/// `display:none` subtree, a non-rendered element, a non-element node, and a
/// document node carry `None`.
pub struct Layout {
    boxes: Vec<Option<Rect>>,
}

impl Layout {
    /// The box generated for `id`, or `None` when the node generates none (a
    /// `display:none`/non-rendered/non-element node — see the type docs).
    pub fn rect(&self, id: NodeId) -> Option<Rect> {
        self.boxes[id as usize]
    }
}

/// Build the box-tree scaffold: walk the render tree from [`Document::roots`],
/// giving every rendered element a placeholder [`Rect::ZERO`] and every other
/// node `None`. **Geometry is filled by later subtasks** — block flow (3.3),
/// inline flow (3.4), flex (3.5); `viewport_w` is threaded through for them and
/// is not read here. The box *kind* is not stored — it is
/// [`Styles::display`](Styles::display), consulted by those subtasks.
pub fn compute(doc: &Document, styles: &Styles, viewport_w: i32) -> Layout {
    let _ = viewport_w; // threaded for geometry (3.3/3.4/3.5); unused in the scaffold
    let mut boxes = vec![None; doc.len()];
    for &root in doc.roots() {
        walk(doc, root, styles, &mut boxes);
    }
    Layout { boxes }
}

/// An element generates a box iff it is rendered: not `display:none` and not a
/// [`NON_RENDERED_TAGS`] element. A skipped element's subtree is not walked, so
/// it stays `None`; non-element nodes generate nothing and are not recursed.
fn walk(doc: &Document, id: NodeId, styles: &Styles, boxes: &mut [Option<Rect>]) {
    let entry = doc.node(id);
    let NodeKind::Element(el) = &entry.kind else {
        return;
    };
    if styles.display_none(id) || NON_RENDERED_TAGS.contains(&el.name.as_str()) {
        return;
    }
    boxes[id as usize] = Some(Rect::ZERO);
    for &c in &entry.children {
        walk(doc, c, styles, boxes);
    }
}

#[cfg(test)]
mod tests;
