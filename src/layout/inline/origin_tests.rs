//! Coordinate-composition regression tests (bl-2161).
//!
//! The bug: inline line-breaking runs in block-local coordinates, and only the
//! `y` origin was composed back on, so every inline descendant of a block placed
//! at a non-zero `x` (in Phase 3 that is any flex row item past the first, and
//! anything nested under it) was emitted at `x = 0` — outside its own parent in
//! the flat, page-coordinate `bboxes` output.
//!
//! These tests pin both halves: exact fixture geometry, and the invariant that
//! makes the class impossible — **a box never starts before its containing box's
//! origin on either axis**. Widths stay estimates (`layout.md` §6, 8px glyph
//! advance); the origin is not an estimate.

use super::*;
use crate::layout::{compute, Layout, VIEWPORT_WIDTH};

fn styled(html: &str) -> (Document, Styles) {
    let doc = Document::parse(html);
    let styles = crate::css::compute(&doc);
    (doc, styles)
}

fn id(doc: &Document, tag: &str) -> NodeId {
    doc.find_by_tag(tag).first().copied().unwrap()
}

/// Lay the whole document out at the real viewport width, then assert the
/// origin invariant over it — every fixture below is also an invariant fixture.
fn lay(html: &str) -> (Document, Layout) {
    let (doc, styles) = styled(html);
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    assert_origins_composed(&doc, &layout);
    (doc, layout)
}

fn rect_of(doc: &Document, layout: &Layout, tag: &str) -> Rect {
    layout.rect(id(doc, tag)).unwrap()
}

/// The invariant the bug broke: composing offsets down the tree only ever moves
/// a box right and down, so every box that has a boxed ancestor starts at or
/// after that ancestor's origin on both axes. (Boxes may still extend *past* an
/// ancestor's far edge — an over-long word overflows its block, `layout.md` §6 —
/// so this is an origin invariant, not a containment one.)
fn assert_origins_composed(doc: &Document, layout: &Layout) {
    for &root in doc.roots() {
        check_origin(doc, layout, root, None);
    }
}

fn check_origin(doc: &Document, layout: &Layout, node: NodeId, container: Option<Rect>) {
    let here = layout.rect(node);
    if let (Some(r), Some(c)) = (here, container) {
        assert!(
            r.x >= c.x && r.y >= c.y,
            "node {node} at {r:?} starts before its container {c:?}"
        );
    }
    // A node with no box (text, comment, `display:none`) passes its container
    // through to its descendants unchanged.
    let inherited = here.or(container);
    for &child in &doc.node(node).children {
        check_origin(doc, layout, child, inherited);
    }
}

/// Run the flow pass over the first `<p>` as if its containing block's content
/// box began at `(x, y)` — the seam `layout_block` uses, exercised directly so
/// wrapping at a non-zero origin is testable without a fixture contrived to wrap.
fn flow_p_at(html: &str, x: i32, y: i32, width_px: i32) -> (Document, Vec<Option<Rect>>, i32) {
    let (doc, styles) = styled(html);
    let block = id(&doc, "p");
    let mut boxes = vec![Some(Rect { x, y, w: 0, h: 0 }); doc.len()];
    let h = flow(&doc, &styles, block, x, y, width_px, &mut boxes);
    (doc, boxes, h)
}

#[test]
fn block_flex_inline_nesting_composes_every_origin() {
    // main(flex) → [span "aaaa" (32 wide, x=0), section (x=32, w=48)]
    //   → p(x=32,w=48) → "hi"(x=32,w=16), cursor 24 → <a>"one"(x=56,w=24).
    let (doc, layout) = lay("<main style=\"display:flex\"><span>aaaa</span>\
         <section><p>hi <a>one</a></p></section></main>");
    assert_eq!(
        rect_of(&doc, &layout, "section"),
        Rect {
            x: 32,
            y: 0,
            w: 48,
            h: 20
        }
    );
    // The bug emitted this at x=0 — two levels of containing origin dropped.
    assert_eq!(
        rect_of(&doc, &layout, "a"),
        Rect {
            x: 56,
            y: 0,
            w: 24,
            h: 20
        }
    );
}

#[test]
fn a_link_in_a_list_keeps_its_list_items_origin() {
    // The reported page shape: an offset container whose links all read x=0.
    // main(flex) → [span "xx" (16 wide), ul(x=16,w=32) → li → a "link"(32)].
    let (doc, layout) = lay("<main style=\"display:flex\"><span>xx</span>\
         <ul><li><a>link</a></li></ul></main>");
    let li = rect_of(&doc, &layout, "li");
    let a = rect_of(&doc, &layout, "a");
    assert_eq!(
        li,
        Rect {
            x: 16,
            y: 0,
            w: 32,
            h: 20
        }
    );
    assert_eq!(a, li);
}

#[test]
fn sibling_inline_runs_shift_together_and_keep_their_spacing() {
    // p is the second flex item (x=16): <a>"aa" at 16, gap 8, <b>"bb" at 40.
    let (doc, layout) = lay("<main style=\"display:flex\"><span>xx</span>\
         <p><a>aa</a> <b>bb</b></p></main>");
    let a = rect_of(&doc, &layout, "a");
    let b = rect_of(&doc, &layout, "b");
    assert_eq!(
        a,
        Rect {
            x: 16,
            y: 0,
            w: 16,
            h: 20
        }
    );
    assert_eq!(
        b,
        Rect {
            x: 40,
            y: 0,
            w: 16,
            h: 20
        }
    );
    // Composing an origin translates the run; it does not restretch it.
    assert_eq!(b.x - a.x, 24);
}

#[test]
fn every_wrapped_line_restarts_at_the_blocks_left_edge() {
    // 40px content box at x=100: "aaaa"(32) fills line 0, "bbbb" wraps to line 1
    // — and line 1 starts at 100, not 0, so the union stays 32 wide.
    let (doc, boxes, h) = flow_p_at("<p><a>aaaa bbbb</a></p>", 100, 40, 40);
    assert_eq!(h, 40);
    assert_eq!(
        boxes[id(&doc, "a") as usize],
        Some(Rect {
            x: 100,
            y: 40,
            w: 32,
            h: 40
        })
    );
}

#[test]
fn an_overflowing_word_overflows_from_the_blocks_origin() {
    // 10 glyphs = 80px in a 50px box: it overflows to the right of x=100,
    // never leftwards to x=0.
    let (doc, boxes, _) = flow_p_at("<p><span>aaaaaaaaaa</span></p>", 100, 0, 50);
    assert_eq!(
        boxes[id(&doc, "span") as usize],
        Some(Rect {
            x: 100,
            y: 0,
            w: 80,
            h: 20
        })
    );
}

#[test]
fn a_reordered_flex_items_descendants_follow_the_item() {
    // row-reverse puts the second <p> first: its <b> at x=0, and the first <p>'s
    // <a> at x=32 — descendants track visual placement, not source order.
    let (doc, layout) = lay("<main style=\"display:flex;flex-direction:row-reverse\">\
         <p><a>aa</a></p><p><b>bbbb</b></p></main>");
    assert_eq!(rect_of(&doc, &layout, "b").x, 0);
    assert_eq!(rect_of(&doc, &layout, "a").x, 32);
}

#[test]
fn an_order_shuffled_flex_items_descendants_follow_the_item() {
    // Same geometry via `order:` instead of a reversed direction.
    let (doc, layout) = lay("<main style=\"display:flex\">\
         <p style=\"order:2\"><a>aa</a></p><p><b>bbbb</b></p></main>");
    assert_eq!(rect_of(&doc, &layout, "b").x, 0);
    assert_eq!(rect_of(&doc, &layout, "a").x, 32);
}

#[test]
fn a_word_less_inline_gets_an_empty_box_at_its_containers_origin() {
    // <i/> renders no word, so it keeps the placeholder box — which is empty at
    // the containing block's origin (16,0), not a zero-size box at the viewport
    // origin. It contributes no advance either: "there" sits where it would
    // without it.
    let (doc, layout) = lay("<main style=\"display:flex\"><span>xx</span>\
         <p>hi <i></i>there</p></main>");
    assert_eq!(
        rect_of(&doc, &layout, "i"),
        Rect {
            x: 16,
            y: 0,
            w: 0,
            h: 0
        }
    );
    assert_eq!(rect_of(&doc, &layout, "p").x, 16);
}

#[test]
fn an_inline_level_block_child_is_anchored_at_the_running_cursor() {
    // The no-anonymous-box gap (`layout.md` §1): a bare inline among block
    // children gets no flow geometry — but its placeholder still carries the
    // container's origin, here the flex item's (x=16) at the y cursor (20).
    let (doc, layout) = lay("<main style=\"display:flex\"><span>xx</span>\
         <section><p>one</p><em><b>x</b></em></section></main>");
    let expected = Rect {
        x: 16,
        y: 20,
        w: 0,
        h: 0,
    };
    assert_eq!(rect_of(&doc, &layout, "em"), expected);
    // walk recurses, so the whole scaffold subtree shares the anchor.
    assert_eq!(rect_of(&doc, &layout, "b"), expected);
}
