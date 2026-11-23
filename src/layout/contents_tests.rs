//! `display: contents` generates no box and its children are laid out in its
//! parent's; a table part stacks as a block (`layout.md` §2, `bl-e3a0`).

use super::*;
use crate::dom::Document;

fn laid(html: &str) -> (Document, Layout) {
    let doc = Document::parse(html);
    let styles = crate::css::compute(&doc);
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    (doc, layout)
}

fn rect(doc: &Document, layout: &Layout, tag: &str) -> Option<Rect> {
    layout.rect(doc.find_by_tag(tag)[0])
}

#[test]
fn contents_has_no_box_and_its_blocks_stack_in_the_parent() {
    let (doc, l) = laid(
        "<section><h1>a</h1><article style='display:contents'>\
         <h2>b</h2><h3>c</h3></article><h4>d</h4></section>",
    );
    assert_eq!(rect(&doc, &l, "article"), None);
    let y = |tag: &str| rect(&doc, &l, tag).unwrap().y;
    assert_eq!(
        [y("h1"), y("h2"), y("h3"), y("h4")],
        [0, LINE_HEIGHT, 2 * LINE_HEIGHT, 3 * LINE_HEIGHT]
    );
    assert_eq!(rect(&doc, &l, "section").unwrap().h, 4 * LINE_HEIGHT);
    assert_eq!(rect(&doc, &l, "h2").unwrap().w, VIEWPORT_WIDTH);
}

#[test]
fn a_block_through_contents_makes_its_parent_stack() {
    // The only block-level box in the section is inside the box-less element.
    let (doc, l) =
        laid("<section><article style='display:contents'><h2>b</h2></article></section>");
    assert_eq!(rect(&doc, &l, "article"), None);
    assert_eq!(rect(&doc, &l, "h2").unwrap().h, LINE_HEIGHT);
    assert_eq!(rect(&doc, &l, "section").unwrap().h, LINE_HEIGHT);
}

#[test]
fn contents_on_a_line_leaves_its_words_and_children_there() {
    let (doc, l) = laid("<p>ab <span style='display:contents'>cd <b>ef</b></span></p>");
    assert_eq!(rect(&doc, &l, "span"), None);
    let b = rect(&doc, &l, "b").unwrap();
    assert_eq!((b.x, b.w), (6 * GLYPH_ADVANCE, 2 * GLYPH_ADVANCE));
    assert_eq!(rect(&doc, &l, "p").unwrap().h, LINE_HEIGHT);
}

#[test]
fn a_table_part_stacks_as_a_block() {
    let (doc, l) = laid(
        "<section><i style='display:table-row'>a</i><b style='display:table-cell'>b</b></section>",
    );
    assert_eq!(rect(&doc, &l, "i").unwrap().w, VIEWPORT_WIDTH);
    assert_eq!(rect(&doc, &l, "b").unwrap().y, LINE_HEIGHT);
}
