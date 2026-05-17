use super::*;
use crate::dom::Document;

#[test]
fn defaults_are_rendered_visible_and_contentless() {
    let cs = ComputedStyle::default();
    assert!(!cs.display_none);
    assert_eq!(cs.visibility, Visibility::Visible);
    assert_eq!(Visibility::default(), Visibility::Visible);
    assert_eq!(cs.before, None);
    assert_eq!(cs.after, None);
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
