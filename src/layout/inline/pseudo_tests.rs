//! Generated `::before`/`::after` content as inline content (`layout.md` §6):
//! it is tokenized exactly like a text child at the position a browser puts it,
//! so it gives its originating element a box and displaces what follows.

use super::*;
use crate::css::Styles;
use crate::dom::Document;
use crate::layout::{compute, VIEWPORT_WIDTH};

/// Parse `html` and cascade its `<style>` blocks — generated content needs the
/// author cascade, so this is `css::compute`, not the bare policy.
fn styled(html: &str) -> (Document, Styles) {
    let doc = Document::parse(html);
    let styles = crate::css::compute(&doc);
    (doc, styles)
}

fn id(doc: &Document, tag: &str) -> NodeId {
    doc.find_by_tag(tag).first().copied().unwrap()
}

/// [`flow`] over the first `<p>`, boxes pre-seeded to `Rect::ZERO` as the block
/// pipeline hands them over. Returns the doc, the filled boxes, and the height.
fn flow_p(html: &str, width_px: i32) -> (Document, Vec<Option<Rect>>, i32) {
    let (doc, styles) = styled(html);
    let block = id(&doc, "p");
    let mut boxes = vec![Some(Rect::ZERO); doc.len()];
    let h = flow(&doc, &styles, block, 0, 0, width_px, &mut boxes);
    (doc, boxes, h)
}

fn box_of(doc: &Document, boxes: &[Option<Rect>], tag: &str) -> Option<Rect> {
    boxes[id(doc, tag) as usize]
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> Option<Rect> {
    Some(Rect { x, y, w, h })
}

/// The python.org shape: a span with no text whose whole content is an icon
/// glyph from `::before`. It gets a real box, and the text after it shifts by
/// the glyph's advance instead of overlapping it.
#[test]
fn empty_inline_gets_a_box_from_its_generated_content() {
    let (doc, boxes, h) = flow_p(
        "<style>i::before{content:\"\\e609\"}</style><p>a<i></i><s>b</s></p>",
        1280,
    );
    assert_eq!(h, 20);
    // "a"(x0,w8) cursor 16; the glyph(x16,w8) cursor 32; "b"(x32,w8).
    assert_eq!(box_of(&doc, &boxes, "i"), rect(16, 0, 8, 20));
    assert_eq!(box_of(&doc, &boxes, "s"), rect(32, 0, 8, 20));
}

/// On a text-bearing inline the two pseudos bracket the children — `::before`
/// ahead, `::after` behind — so the union spans both and the next element sits
/// past the `::after`.
#[test]
fn before_and_after_bracket_the_children_of_a_text_bearing_inline() {
    let (doc, boxes, _) = flow_p(
        "<style>a::before{content:\"[[\"}a::after{content:\"]\"}</style>\
         <p><a>hi</a><s>t</s></p>",
        1280,
    );
    // "[["(x0,w16) "hi"(x24,w16) "]"(x48,w8) → union x0..56; "t"(x64,w8).
    assert_eq!(box_of(&doc, &boxes, "a"), rect(0, 0, 56, 20));
    assert_eq!(box_of(&doc, &boxes, "s"), rect(64, 0, 8, 20));
}

/// A block whose only content is generated is one line tall, not zero — its own
/// pseudo strings are inline content of the formatting context it establishes.
#[test]
fn block_with_only_generated_content_is_one_line_tall() {
    let (doc, styles) = styled("<style>div::after{content:\"xx\"}</style><div></div>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    assert_eq!(layout.rect(id(&doc, "div")), rect(0, 0, 1280, 20));
}

/// A pseudo with `display:none` computes no string in the cascade, so it reaches
/// layout as absent content and the element keeps its placeholder box.
#[test]
fn display_none_pseudo_contributes_no_geometry() {
    let (doc, boxes, h) = flow_p(
        "<style>i::before{content:\"xxxx\";display:none}</style><p>a<i></i></p>",
        1280,
    );
    assert_eq!(h, 20);
    assert_eq!(box_of(&doc, &boxes, "i"), Some(Rect::ZERO));
}

/// `visibility:hidden` still occupies space (`layout.md` §7), and so does the
/// generated content of a hidden element.
#[test]
fn hidden_element_keeps_its_generated_content_geometry() {
    let (doc, boxes, _) = flow_p(
        "<style>i{visibility:hidden}i::before{content:\"xxxx\"}</style>\
         <p><i></i><s>b</s></p>",
        1280,
    );
    assert_eq!(box_of(&doc, &boxes, "i"), rect(0, 0, 32, 20));
    assert_eq!(box_of(&doc, &boxes, "s"), rect(40, 0, 8, 20));
}

/// Generated content wraps on word boundaries like any other text, so a long
/// `::before` makes its element two lines tall.
#[test]
fn generated_content_wraps_at_word_boundaries() {
    let (doc, boxes, h) = flow_p(
        "<style>i::before{content:\"aaaa bbbb cccc\"}</style><p><i></i></p>",
        100,
    );
    assert_eq!(h, 40);
    assert_eq!(box_of(&doc, &boxes, "i"), rect(0, 0, 72, 40));
}

/// `attr()` content measures the attribute's value — the geometry follows the
/// DOM, with no separate path.
#[test]
fn attr_content_is_measured_from_the_attribute() {
    let (doc, boxes, _) = flow_p(
        "<style>i::before{content:attr(data-x)}</style><p><i data-x=\"abcd\"></i></p>",
        1280,
    );
    assert_eq!(box_of(&doc, &boxes, "i"), rect(0, 0, 32, 20));
}

/// A flex item's crude intrinsic width counts its own generated content, so an
/// icon-only item is not zero-wide and the item after it starts past it.
#[test]
fn flex_item_intrinsic_width_counts_generated_content() {
    let (doc, styles) = styled(
        "<style>div{display:flex}i::before{content:\"abcd\"}</style>\
         <div><i></i><i>zz</i></div>",
    );
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    let items = doc.find_by_tag("i");
    // item 0: "abcd" → 32px. item 1: "abcd"+"zz" → 48px + one 8px gap = 56px.
    assert_eq!(layout.rect(items[0]), rect(0, 0, 32, 20));
    assert_eq!(layout.rect(items[1]), rect(32, 0, 56, 20));
}

/// The anonymous-box gap is inherited, not invented: in a container that also
/// has block children, generated content is dropped exactly as a stray text
/// child there is (`layout.md` §1 — no anonymous block promotion).
#[test]
fn generated_content_beside_block_children_is_dropped_like_stray_text() {
    let (doc, styles) = styled(
        "<style>div::before{content:\"xx\"}</style><div>stray<p>a</p></div>\
         <div>stray<p>a</p></div>",
    );
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    let divs = doc.find_by_tag("div");
    // The pseudo-bearing div is the same one line tall as its plain twin: both
    // lost their inline content to the block child.
    assert_eq!(layout.rect(divs[0]).unwrap().h, 20);
    assert_eq!(layout.rect(divs[1]).unwrap().h, 20);
}
