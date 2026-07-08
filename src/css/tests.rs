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
