use crate::css::{compute, Styles, Visibility};
use crate::dom::Document;

fn styles(html: &str) -> (Document, Styles) {
    let doc = Document::parse(html);
    let s = compute(&doc);
    (doc, s)
}

fn first_tag(doc: &Document, tag: &str) -> u32 {
    *doc.find_by_tag(tag).first().expect("tag present")
}

#[test]
fn display_none_via_style_element() {
    let (doc, s) = styles("<style>p{display:none}</style><p>x</p><div>y</div>");
    assert!(s.display_none(first_tag(&doc, "p")));
    assert!(!s.display_none(first_tag(&doc, "div")));
}

#[test]
fn non_matching_rule_is_inert() {
    let (doc, s) = styles("<style>div{display:none}</style><p>x</p>");
    assert!(!s.display_none(first_tag(&doc, "p")));
}

#[test]
fn inline_beats_sheet_but_important_sheet_wins() {
    let (doc, s) = styles("<style>p{display:none}</style><p style='display:block'>x</p>");
    assert!(!s.display_none(first_tag(&doc, "p")));
    let (doc, s) =
        styles("<style>p{display:none!important}</style><p style='display:block'>x</p>");
    assert!(s.display_none(first_tag(&doc, "p")));
}

#[test]
fn specificity_and_source_order_break_ties() {
    // equal specificity → later rule wins.
    let (doc, s) = styles("<style>p{display:none}p{display:block}</style><p>x</p>");
    assert!(!s.display_none(first_tag(&doc, "p")));
    // higher specificity wins regardless of order.
    let (doc, s) = styles("<style>.c{display:block}p{display:none}</style><p class=c>x</p>");
    assert!(!s.display_none(first_tag(&doc, "p")));
}

#[test]
fn visibility_inherits_and_is_overridable() {
    let (doc, s) = styles(
        "<div style='visibility:hidden'><span>a<i style='visibility:visible'>b</i></span></div>",
    );
    assert_eq!(s.visibility(first_tag(&doc, "div")), Visibility::Hidden);
    assert_eq!(s.visibility(first_tag(&doc, "span")), Visibility::Hidden);
    assert_eq!(s.visibility(first_tag(&doc, "i")), Visibility::Visible);
}

#[test]
fn visibility_collapse_is_hidden_and_visible_keyword_stays_visible() {
    let (doc, s) =
        styles("<p style='visibility:collapse'>a</p><q style='visibility:visible'>b</q>");
    assert_eq!(s.visibility(first_tag(&doc, "p")), Visibility::Hidden);
    assert_eq!(s.visibility(first_tag(&doc, "q")), Visibility::Visible);
}

#[test]
fn descendant_selector_needs_ancestor_context() {
    let (doc, s) = styles(
        "<style>.box p{display:none}</style>\
         <div class=box><section><p>x</p></section></div><p>keep</p>",
    );
    let ps = doc.find_by_tag("p");
    assert!(s.display_none(ps[0]));
    assert!(!s.display_none(ps[1]));
}

#[test]
fn generated_content_before_after_and_attr() {
    let (doc, s) = styles(
        r#"<style>
            a::before{content:"[" attr(data-tag) "] "}
            a::after{content:'!'}
            b::before{content:none}
            i::before{content:normal}
            u::before{display:none;content:"x"}
            c::before{content:attr(missing)}
        </style>
        <a data-tag="hot">link</a><b>x</b><i>y</i><u>z</u><c>w</c>"#,
    );
    let a = first_tag(&doc, "a");
    assert_eq!(s.before(a), Some("[hot] "));
    assert_eq!(s.after(a), Some("!"));
    assert_eq!(s.before(first_tag(&doc, "b")), None);
    assert_eq!(s.before(first_tag(&doc, "i")), None);
    assert_eq!(s.before(first_tag(&doc, "u")), None);
    // attr() of an absent attribute contributes the empty string.
    assert_eq!(s.before(first_tag(&doc, "c")), Some(""));
    // a plain element with no ::before rule has no generated box.
    assert_eq!(s.after(first_tag(&doc, "b")), None);
}

#[test]
fn content_token_edges() {
    // escaped quote; escaped backslash; unterminated string; an ignored
    // keyword token next to a quoted one; a stray ')'; and a trailing
    // backslash with nothing after it.
    let css = "<style>\
        p::before{content:\"a\\\"b\"}\
        q::before{content:\"tail\\\\\"}\
        r::before{content:\"open}\
        s::before{content:open-quote \"ok\"}\
        t::before{content:x)}\
        w::before{content:\"z\\\
        </style><p>1</p><q>2</q><r>3</r><s>4</s><t>5</t><w>6</w>";
    let (doc, st) = styles(css);
    assert_eq!(st.before(first_tag(&doc, "p")), Some(r#"a"b"#));
    assert_eq!(st.before(first_tag(&doc, "q")), Some("tail\\"));
    assert_eq!(st.before(first_tag(&doc, "r")), Some("open"));
    assert_eq!(st.before(first_tag(&doc, "s")), Some("ok"));
    assert_eq!(st.before(first_tag(&doc, "t")), Some(""));
    assert_eq!(st.before(first_tag(&doc, "w")), Some("z\\"));
}

#[test]
fn non_elements_get_default_style() {
    let (doc, s) = styles("<!doctype html><!--c--><p>text</p>");
    for id in 0..doc.len() as u32 {
        let cs = s.get(id);
        let _ = (cs.display_none, cs.visibility, &cs.before, &cs.after);
    }
    assert!(!s.display_none(first_tag(&doc, "p")));
}
