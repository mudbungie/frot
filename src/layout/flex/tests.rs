use super::super::{compute, Layout, Rect, VIEWPORT_WIDTH};
use crate::dom::{Document, NodeId};

/// Parse `html` and compute UA-implicit + inline-`style=` styles — the pair
/// `compute` takes. Inline `style=` carries `display`/`order`/`flex-direction`.
fn styled(html: &str) -> (Document, crate::css::Styles) {
    let doc = Document::parse(html);
    let styles = crate::css::compute(&doc);
    (doc, styles)
}

fn id(doc: &Document, tag: &str) -> NodeId {
    doc.find_by_tag(tag).first().copied().unwrap()
}

fn lay(html: &str) -> (Document, Layout) {
    let (doc, styles) = styled(html);
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    (doc, layout)
}

#[test]
fn child_order_sorts_by_order_then_source_index() {
    // order: a=2, b=1, c=0(default) → reading order [c, b, a].
    let (doc, layout) = lay(
        "<div style=\"display:flex\"><a style=\"order:2\">A</a>\
         <b style=\"order:1\">B</b><c>C</c></div>",
    );
    assert_eq!(
        layout.child_order(id(&doc, "div")),
        vec![id(&doc, "c"), id(&doc, "b"), id(&doc, "a")]
    );
}

#[test]
fn equal_order_items_keep_source_order() {
    // Both default order 0 → the stable sort preserves markup order.
    let (doc, layout) = lay("<div style=\"display:flex\"><a>A</a><b>B</b></div>");
    assert_eq!(
        layout.child_order(id(&doc, "div")),
        vec![id(&doc, "a"), id(&doc, "b")]
    );
}

#[test]
fn negative_order_sorts_ahead_of_default() {
    // b's -1 < a's 0 → b first.
    let (doc, layout) =
        lay("<div style=\"display:flex\"><a>A</a><b style=\"order:-1\">B</b></div>");
    assert_eq!(
        layout.child_order(id(&doc, "div")),
        vec![id(&doc, "b"), id(&doc, "a")]
    );
}

#[test]
fn row_reverse_reverses_child_order() {
    let (doc, layout) = lay(
        "<div style=\"display:flex;flex-direction:row-reverse\">\
         <a>A</a><b>B</b></div>",
    );
    assert_eq!(
        layout.child_order(id(&doc, "div")),
        vec![id(&doc, "b"), id(&doc, "a")]
    );
}

#[test]
fn column_reverse_reverses_child_order() {
    let (doc, layout) = lay(
        "<div style=\"display:flex;flex-direction:column-reverse\">\
         <a>A</a><b>B</b></div>",
    );
    assert_eq!(
        layout.child_order(id(&doc, "div")),
        vec![id(&doc, "b"), id(&doc, "a")]
    );
}

#[test]
fn child_order_is_empty_for_non_flex_and_non_element_ids() {
    let (doc, layout) = lay("<div><p>x</p></div>");
    // A block container has no stored flex order.
    assert_eq!(layout.child_order(id(&doc, "div")), Vec::<NodeId>::new());
    // A text node id also has none.
    let text = doc.node(id(&doc, "p")).children[0];
    assert_eq!(layout.child_order(text), Vec::<NodeId>::new());
}

#[test]
fn row_places_items_at_increasing_x_by_intrinsic_width() {
    // span1 "aa bb": 2 words × 2 glyphs (32) + 1 inter-word gap (8) = 40 wide.
    // span2 "cc": 1 word × 2 glyphs = 16 wide, placed at x=40.
    let (doc, layout) =
        lay("<div style=\"display:flex\"><span>aa bb</span><span>cc</span></div>");
    let spans = doc.find_by_tag("span");
    assert_eq!(layout.rect(spans[0]).unwrap(), Rect { x: 0, y: 0, w: 40, h: 20 });
    assert_eq!(layout.rect(spans[1]).unwrap(), Rect { x: 40, y: 0, w: 16, h: 20 });
    // Container spans the viewport width; height = the tallest item.
    let div = layout.rect(id(&doc, "div")).unwrap();
    assert_eq!(div, Rect { x: 0, y: 0, w: 1280, h: 20 });
}

#[test]
fn row_empty_item_is_zero_width_and_next_starts_at_zero() {
    let (doc, layout) =
        lay("<div style=\"display:flex\"><span></span><span>x</span></div>");
    let spans = doc.find_by_tag("span");
    // 0 words → 0 wide, 0 tall.
    assert_eq!(layout.rect(spans[0]).unwrap(), Rect { x: 0, y: 0, w: 0, h: 0 });
    // The next item still starts at x=0 (the empty item added no width).
    assert_eq!(layout.rect(spans[1]).unwrap(), Rect { x: 0, y: 0, w: 8, h: 20 });
}

#[test]
fn column_stacks_items_full_width_and_sums_height() {
    let (doc, layout) = lay(
        "<div style=\"display:flex;flex-direction:column\"><p>a</p><p>b</p></div>",
    );
    let ps = doc.find_by_tag("p");
    assert_eq!(layout.rect(ps[0]).unwrap(), Rect { x: 0, y: 0, w: 1280, h: 20 });
    assert_eq!(layout.rect(ps[1]).unwrap(), Rect { x: 0, y: 20, w: 1280, h: 20 });
    // Container height = sum of stacked items.
    assert_eq!(layout.rect(id(&doc, "div")).unwrap(), Rect { x: 0, y: 0, w: 1280, h: 40 });
}

#[test]
fn row_reverse_places_last_source_item_first() {
    // Source: span1 "aa"(16 wide), span2 "bbbb"(32 wide). Reversed reading order
    // [span2, span1] → span2 at x=0, span1 at x=32.
    let (doc, layout) = lay(
        "<div style=\"display:flex;flex-direction:row-reverse\">\
         <span>aa</span><span>bbbb</span></div>",
    );
    let spans = doc.find_by_tag("span");
    assert_eq!(layout.rect(spans[1]).unwrap(), Rect { x: 0, y: 0, w: 32, h: 20 });
    assert_eq!(layout.rect(spans[0]).unwrap(), Rect { x: 32, y: 0, w: 16, h: 20 });
}

#[test]
fn inline_flex_gets_child_order_but_is_not_block_placed() {
    let (doc, layout) =
        lay("<span style=\"display:inline-flex\"><a>1</a><b>2</b></span>");
    let span = id(&doc, "span");
    // child_order works for an inline-flex container (feeds the 3.7 refinement).
    assert_eq!(layout.child_order(span), vec![id(&doc, "a"), id(&doc, "b")]);
    // But inline-flex atomic sizing is out of scope: its box is the crude inline
    // text union ("1" at x0, "2" at x16), not a block-placed flex container.
    assert_eq!(layout.rect(span).unwrap(), Rect { x: 0, y: 0, w: 24, h: 20 });
}
