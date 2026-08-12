use super::*;
use crate::dom::Document;

#[test]
fn defaults_are_rendered_visible_and_contentless() {
    let cs = ComputedStyle::default();
    // Default display is Inline (never None): a default node reports rendered.
    assert_eq!(cs.display, Display::Inline);
    assert_eq!(Display::default(), Display::Inline);
    assert_eq!(cs.visibility, Visibility::Visible);
    assert_eq!(Visibility::default(), Visibility::Visible);
    assert_eq!(cs.before, None);
    assert_eq!(cs.after, None);
    // Flex defaults: no reorder, row main axis.
    assert_eq!(cs.order, 0);
    assert_eq!(cs.flex_direction, FlexDirection::Row);
    assert_eq!(FlexDirection::default(), FlexDirection::Row);
}

#[test]
fn flex_direction_parse_maps_keywords_and_coerces_the_rest() {
    assert_eq!(FlexDirection::parse("row"), FlexDirection::Row);
    assert_eq!(
        FlexDirection::parse("row-reverse"),
        FlexDirection::RowReverse
    );
    assert_eq!(FlexDirection::parse("column"), FlexDirection::Column);
    assert_eq!(
        FlexDirection::parse("column-reverse"),
        FlexDirection::ColumnReverse
    );
    // Case-insensitive and trimmed.
    assert_eq!(
        FlexDirection::parse("  COLUMN-Reverse "),
        FlexDirection::ColumnReverse
    );
    // Any unknown value coerces to Row.
    assert_eq!(FlexDirection::parse("wat"), FlexDirection::Row);
}

#[test]
fn display_parse_maps_the_seven_keywords_and_coerces_the_rest() {
    assert_eq!(Display::parse("none"), Display::None);
    assert_eq!(Display::parse("block"), Display::Block);
    assert_eq!(Display::parse("inline"), Display::Inline);
    assert_eq!(Display::parse("inline-block"), Display::InlineBlock);
    assert_eq!(Display::parse("list-item"), Display::ListItem);
    assert_eq!(Display::parse("flex"), Display::Flex);
    assert_eq!(Display::parse("inline-flex"), Display::InlineFlex);
    // Case-insensitive and trimmed, like the visibility keyword parsing.
    assert_eq!(Display::parse("  INLINE-Block "), Display::InlineBlock);
    // Everything else — grid, table*, contents, unknown — coerces to Block.
    assert_eq!(Display::parse("grid"), Display::Block);
    assert_eq!(Display::parse("table-cell"), Display::Block);
    assert_eq!(Display::parse("contents"), Display::Block);
    assert_eq!(Display::parse("wat"), Display::Block);
}

#[test]
fn accessors_expose_the_computed_table() {
    let doc = Document::parse(
        "<style>p{display:none}q{visibility:hidden}r::before{content:'x'}</style>\
         <p>1</p><q>2</q><r>3</r><s>4</s>",
    );
    let s = compute(&doc);
    let id = |t| *doc.find_by_tag(t).first().unwrap();
    assert!(s.display_none(id("p")));
    // display_none is the derived `display == None` query.
    assert_eq!(s.display(id("p")), Display::None);
    assert_eq!(s.visibility(id("q")), Visibility::Hidden);
    assert_eq!(s.before(id("r")), Some("x"));
    assert_eq!(s.after(id("r")), None);
    // an untouched element carries the default ComputedStyle.
    assert_eq!(s.get(id("s")), &ComputedStyle::default());
}

#[test]
fn rendered_subtree_text_applies_display_none_and_generated_content() {
    let doc = Document::parse(
        "<style>p::before{content:'['}p::after{content:']'}q{display:none}</style>\
         <div><!--c--><p>hi</p><q>skip</q>tail</div>",
    );
    let s = compute(&doc);
    let div = *doc.find_by_tag("div").first().unwrap();
    assert_eq!(rendered_subtree_text(&doc, div, &s), "[hi]tail");
}

/// Generated content reaches the rendered text with its CSS escapes decoded —
/// the python.org icon-font case (`.icon-download:before{content:"\e609"}` is
/// one private-use glyph, not the literal text `e609`), one assertion per
/// escape class.
#[test]
fn generated_content_decodes_css_escapes() {
    let doc = Document::parse(
        r#"<style>
             .icon-download::before { content: "\e609" }
             .dash::before          { content: "\2014 " }
             .quote::before         { content: "\"q\"" }
             .cont::before          { content: "a\
b" }
             .nul::before           { content: "\0" }
           </style>
           <span class="icon-download">Download</span><span class="dash">D</span>
           <span class="quote">Q</span><span class="cont">C</span>
           <span class="nul">N</span>"#,
    );
    let s = compute(&doc);
    let text = |i| {
        let id = doc.find_by_tag("span")[i];
        rendered_subtree_text(&doc, id, &s)
    };
    assert_eq!(text(0), "\u{e609}Download");
    assert_eq!(text(1), "—D");
    assert_eq!(text(2), "\"q\"Q");
    assert_eq!(text(3), "abC");
    assert_eq!(text(4), "\u{fffd}N");
}
