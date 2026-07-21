use super::*;
use crate::dom::Document;

/// Parse `html` and compute UA-implicit + inline-`style=` display for it — the
/// pair every `compute` call takes.
fn styled(html: &str) -> (Document, Styles) {
    let doc = Document::parse(html);
    let styles = crate::css::compute(&doc);
    (doc, styles)
}

fn id(doc: &Document, tag: &str) -> NodeId {
    doc.find_by_tag(tag).first().copied().unwrap()
}

/// The rect of the first `tag` element, unwrapped.
fn rect(doc: &Document, layout: &Layout, tag: &str) -> Rect {
    layout.rect(id(doc, tag)).unwrap()
}

#[test]
fn viewport_width_is_the_1280_constant() {
    assert_eq!(VIEWPORT_WIDTH, 1280);
}

#[test]
fn line_height_is_font_size_times_line_height() {
    assert_eq!(LINE_HEIGHT, 20);
}

#[test]
fn text_leaf_block_gets_one_line_high_full_width_box_at_origin() {
    let (doc, styles) = styled("<p>hi</p>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    assert_eq!(
        rect(&doc, &layout, "p"),
        Rect {
            x: 0,
            y: 0,
            w: 1280,
            h: 20
        }
    );
}

#[test]
fn html_and_body_are_block_and_full_width_at_origin() {
    // html/body were added to the block set; the page root lays out as block.
    let (doc, styles) = styled("<p>hi</p>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    assert_eq!(
        rect(&doc, &layout, "html"),
        Rect {
            x: 0,
            y: 0,
            w: 1280,
            h: 20
        }
    );
    assert_eq!(
        rect(&doc, &layout, "body"),
        Rect {
            x: 0,
            y: 0,
            w: 1280,
            h: 20
        }
    );
}

#[test]
fn stacked_block_siblings_get_increasing_y() {
    let (doc, styles) = styled("<p>a</p><p>b</p>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    let ps = doc.find_by_tag("p");
    let first = layout.rect(ps[0]).unwrap();
    let second = layout.rect(ps[1]).unwrap();
    assert_eq!(
        first,
        Rect {
            x: 0,
            y: 0,
            w: 1280,
            h: 20
        }
    );
    // The second sibling starts exactly where the first ends.
    assert_eq!(
        second,
        Rect {
            x: 0,
            y: 20,
            w: 1280,
            h: 20
        }
    );
    assert_eq!(second.y, first.h);
}

#[test]
fn container_height_is_the_sum_of_its_block_children() {
    // div stacks two 20px paragraphs → 40px tall; children keep x=0 w=1280.
    let (doc, styles) = styled("<div><p>a</p><p>b</p></div>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    assert_eq!(
        rect(&doc, &layout, "div"),
        Rect {
            x: 0,
            y: 0,
            w: 1280,
            h: 40
        }
    );
    let ps = doc.find_by_tag("p");
    assert_eq!(layout.rect(ps[0]).unwrap().h, 20);
    assert_eq!(layout.rect(ps[1]).unwrap().y, 20);
}

#[test]
fn nested_blocks_all_get_full_width_at_left_edge() {
    let (doc, styles) = styled("<div><section><p>x</p></section></div>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    for tag in ["div", "section", "p"] {
        let r = rect(&doc, &layout, tag);
        assert_eq!((r.x, r.w), (0, 1280), "tag {tag}");
    }
}

#[test]
fn empty_and_whitespace_only_blocks_are_zero_height() {
    let (doc, styles) = styled("<p>   </p>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    // Whitespace-only text is not rendered content.
    assert_eq!(
        rect(&doc, &layout, "p"),
        Rect {
            x: 0,
            y: 0,
            w: 1280,
            h: 0
        }
    );
}

#[test]
fn comment_only_block_is_zero_height() {
    // A block whose sole inline content is a comment has no rendered text.
    let (doc, styles) = styled("<p><!--note--></p>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    assert_eq!(rect(&doc, &layout, "p").h, 0);
}

#[test]
fn text_hidden_by_display_none_does_not_give_a_block_height() {
    // The only text lives under display:none, so the block measures as empty.
    let (doc, styles) = styled("<p><span style=\"display:none\">hidden</span></p>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    assert_eq!(rect(&doc, &layout, "p").h, 0);
    assert_eq!(layout.rect(id(&doc, "span")), None);
}

#[test]
fn display_none_block_generates_no_box_and_no_height() {
    let (doc, styles) = styled("<div style=\"display:none\"><p>x</p></div>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    assert_eq!(layout.rect(id(&doc, "div")), None);
    assert_eq!(layout.rect(id(&doc, "p")), None);
}

#[test]
fn hidden_block_child_is_skipped_but_real_siblings_stack() {
    // is_block_box rejects the display:none <p>; only the real one has height.
    let (doc, styles) = styled("<div><p style=\"display:none\">x</p><p>real</p></div>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    let ps = doc.find_by_tag("p");
    assert_eq!(layout.rect(ps[0]), None);
    assert_eq!(
        layout.rect(ps[1]).unwrap(),
        Rect {
            x: 0,
            y: 0,
            w: 1280,
            h: 20
        }
    );
    // The hidden child contributes nothing to the container height.
    assert_eq!(rect(&doc, &layout, "div").h, 20);
}

#[test]
fn non_rendered_tags_and_their_subtrees_generate_no_box() {
    let (doc, styles) = styled("<body><script>var x=1;</script><p>ok</p></body>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    assert_eq!(layout.rect(id(&doc, "script")), None);
    assert_eq!(layout.rect(id(&doc, "head")), None);
    assert_eq!(
        rect(&doc, &layout, "p"),
        Rect {
            x: 0,
            y: 0,
            w: 1280,
            h: 20
        }
    );
}

#[test]
fn inline_child_of_a_block_container_keeps_a_zero_placeholder() {
    // A rendered non-block child (span) gets Rect::ZERO and adds no height; the
    // block sibling still lays out from y=0.
    let (doc, styles) = styled("<div><span>hi</span><p>x</p></div>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    assert_eq!(layout.rect(id(&doc, "span")), Some(Rect::ZERO));
    assert_eq!(
        rect(&doc, &layout, "p"),
        Rect {
            x: 0,
            y: 0,
            w: 1280,
            h: 20
        }
    );
    assert_eq!(rect(&doc, &layout, "div").h, 20);
}

#[test]
fn text_node_between_block_children_is_ignored_for_flow() {
    // A bare text node among block children generates no box and no height.
    let (doc, styles) = styled("<div>loose text<p>x</p></div>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    let text_id = doc.node(id(&doc, "div")).children[0];
    assert!(matches!(doc.node(text_id).kind, NodeKind::Text(_)));
    assert_eq!(layout.rect(text_id), None);
    assert_eq!(rect(&doc, &layout, "div").h, 20);
}

#[test]
fn nested_inline_descendant_gets_a_zero_placeholder() {
    // walk recurses into an inline element's children, giving each Rect::ZERO.
    let (doc, styles) = styled("<div><span><b>x</b></span><p>y</p></div>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    assert_eq!(layout.rect(id(&doc, "b")), Some(Rect::ZERO));
}

#[test]
fn doctype_root_is_skipped_and_html_starts_at_origin() {
    // The doctype is a non-element root: it contributes no box and no height.
    let (doc, styles) = styled("<!doctype html><html><body><p>x</p></body></html>");
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    assert_eq!(rect(&doc, &layout, "html").y, 0);
    assert_eq!(
        rect(&doc, &layout, "p"),
        Rect {
            x: 0,
            y: 0,
            w: 1280,
            h: 20
        }
    );
}
