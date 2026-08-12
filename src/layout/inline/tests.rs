use super::*;
use crate::dom::Document;

/// Parse `html` and compute UA-implicit + inline-`style=` display for it.
fn styled(html: &str) -> (Document, Styles) {
    let doc = Document::parse(html);
    let styles = crate::css::compute(&doc);
    (doc, styles)
}

fn id(doc: &Document, tag: &str) -> NodeId {
    doc.find_by_tag(tag).first().copied().unwrap()
}

/// Run the inline flow pass over the first `<p>` of `html` at width `width_px`,
/// with every box pre-seeded to `Rect::ZERO` (the post-`walk` state the block
/// pipeline hands the pass). Returns the doc, the filled boxes, and the height.
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

#[test]
fn short_paragraph_is_one_line_high() {
    let (_, _, h) = flow_p("<p>hi there</p>", 1280);
    assert_eq!(h, 20);
}

#[test]
fn long_paragraph_wraps_to_three_lines() {
    // Six 4-glyph words (w=32, advance 40) pack two per 100px line → 3 lines.
    let (_, _, h) = flow_p("<p>aaaa bbbb cccc dddd eeee ffff</p>", 100);
    assert_eq!(h, 60); // 3 lines × LINE_HEIGHT
}

#[test]
fn over_long_word_takes_one_overflowing_line() {
    // 10 glyphs = 80px, wider than the 50px block: its own line, overflowing.
    let (doc, boxes, h) = flow_p("<p><span>aaaaaaaaaa</span></p>", 50);
    assert_eq!(h, 20);
    assert_eq!(
        box_of(&doc, &boxes, "span"),
        Some(Rect {
            x: 0,
            y: 0,
            w: 80,
            h: 20
        })
    );
}

#[test]
fn inline_element_rect_unions_its_words_on_one_line() {
    // "hi "→x0..16 cursor24; <a>"one"(x24,w24) "two"(x56,w24)</a>; "bye"(x88).
    let (doc, boxes, h) = flow_p("<p>hi <a>one two</a> bye</p>", 1280);
    assert_eq!(h, 20);
    assert_eq!(
        box_of(&doc, &boxes, "a"),
        Some(Rect {
            x: 24,
            y: 0,
            w: 56,
            h: 20
        })
    );
}

#[test]
fn nested_inline_boxes_union_own_and_descendant_words() {
    // <a><b>xx</b>yy</a>: b sees "xx"(x0,w16); a sees "xx"+"yy"(x24,w16).
    let (doc, boxes, _) = flow_p("<p><a><b>xx</b>yy</a></p>", 1280);
    assert_eq!(
        box_of(&doc, &boxes, "b"),
        Some(Rect {
            x: 0,
            y: 0,
            w: 16,
            h: 20
        })
    );
    assert_eq!(
        box_of(&doc, &boxes, "a"),
        Some(Rect {
            x: 0,
            y: 0,
            w: 40,
            h: 20
        })
    );
}

#[test]
fn inline_element_wrapping_two_lines_gives_a_taller_union() {
    // <a>aaaa bbbb cccc</a> at 100px: (0,0,32,20)(40,0,32,20)(0,20,32,20).
    let (doc, boxes, h) = flow_p("<p><a>aaaa bbbb cccc</a></p>", 100);
    assert_eq!(h, 40);
    assert_eq!(
        box_of(&doc, &boxes, "a"),
        Some(Rect {
            x: 0,
            y: 0,
            w: 72,
            h: 40
        })
    );
}

#[test]
fn whitespace_only_block_is_zero_height() {
    let (_, _, h) = flow_p("<p>   </p>", 1280);
    assert_eq!(h, 0);
}

#[test]
fn display_none_inline_child_is_skipped_and_keeps_zero() {
    // The hidden span contributes no word (one line) and keeps its ZERO box.
    let (doc, boxes, h) = flow_p(
        "<p>hi <span style=\"display:none\">no no no</span> bye</p>",
        1280,
    );
    assert_eq!(h, 20);
    assert_eq!(box_of(&doc, &boxes, "span"), Some(Rect::ZERO));
}

#[test]
fn comment_in_inline_content_contributes_no_word() {
    // The comment node hits the non-text/non-element arm; "hi"+"bye" fit a line.
    let (_, _, h) = flow_p("<p>hi<!--c-->bye</p>", 1280);
    assert_eq!(h, 20);
}

#[test]
fn compute_fills_inline_rects_through_the_block_pipeline() {
    // End-to-end: block flow seeds ZERO via walk, then flow unions <a>'s word.
    let (doc, styles) = styled("<p>hi <a>link</a></p>");
    let layout = crate::layout::compute(&doc, &styles, crate::layout::VIEWPORT_WIDTH);
    // "hi "→cursor x24; "link"(4 glyphs=32) → a rect {24,0,32,20}.
    assert_eq!(
        layout.rect(id(&doc, "a")),
        Some(Rect {
            x: 24,
            y: 0,
            w: 32,
            h: 20
        })
    );
    assert_eq!(layout.rect(id(&doc, "p")).unwrap().h, 20);
}

#[test]
fn an_inline_non_rendered_element_measures_nothing() {
    // `<script>`/`<style>` are UA-implicit `inline`, so they reach the inline
    // collector as ordinary inline elements; `NON_RENDERED_TAGS` is what keeps
    // their source out of the line box. Without it the `<p>` would be sized
    // from JavaScript nobody sees.
    let (doc, styles) = styled("<p>ab<script>xxxxxxxxxxxxxxxx</script>cd</p>");
    let layout = crate::layout::compute(&doc, &styles, crate::layout::VIEWPORT_WIDTH);
    let words = super::content_of(&doc, &styles, id(&doc, "p"));
    assert_eq!(words.len(), 2);
    assert!(layout.rect(id(&doc, "script")).is_none());
}
