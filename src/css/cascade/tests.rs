use crate::css::{compute, Display, FlexDirection, Styles, Visibility};
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
    let (doc, s) = styles("<style>p{display:none!important}</style><p style='display:block'>x</p>");
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
fn media_min_width_overrides_apply_at_the_desktop_viewport() {
    // bl-8ff4 minimal repro: mobile-first CSS hides desktop chrome in the base
    // rules and un-hides it (and hides the hamburger) inside
    // `@media (min-width: 768px)`. At the fixed 1280px viewport the desktop
    // nav must render and the hamburger must not — frot used to emit the
    // exact inverse because @media blocks were skipped wholesale.
    let (doc, s) = styles(
        "<style>\
         .desktop-nav { display: none; }\
         @media (min-width: 768px) { .desktop-nav { display: block; } }\
         .hamburger { display: block; }\
         @media (min-width: 768px) { .hamburger { display: none; } }\
         </style>\
         <nav class=desktop-nav><a href=/docs>Docs</a><a href=/about>About</a></nav>\
         <button class=hamburger>Mobile navigation</button>",
    );
    assert!(!s.display_none(first_tag(&doc, "nav")));
    assert!(s.display_none(first_tag(&doc, "button")));
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
        let _ = (cs.display, cs.visibility, &cs.before, &cs.after);
    }
    assert!(!s.display_none(first_tag(&doc, "p")));
}

#[test]
fn implicit_display_by_tag_when_no_rule_wins() {
    // No `display` rule: seed from the shared block-tag set. `li` → list-item,
    // other block tags → block, everything else → inline. Never `none`.
    let (doc, s) = styles("<ul><li>a</li></ul><div>b</div><span>c</span>");
    assert_eq!(s.display(first_tag(&doc, "li")), Display::ListItem);
    assert_eq!(s.display(first_tag(&doc, "ul")), Display::Block);
    assert_eq!(s.display(first_tag(&doc, "div")), Display::Block);
    assert_eq!(s.display(first_tag(&doc, "span")), Display::Inline);
    assert!(!s.display_none(first_tag(&doc, "span")));
}

#[test]
fn author_display_rule_overrides_implicit_and_is_coerced() {
    // A winning author rule is parsed (and coerced) by Display::parse,
    // overriding the element's UA-implicit display.
    let (doc, s) = styles(
        "<style>span{display:inline-flex}div{display:grid}li{display:block}</style>\
         <span>a</span><div>b</div><ul><li>c</li></ul>",
    );
    assert_eq!(s.display(first_tag(&doc, "span")), Display::InlineFlex);
    // `grid` coerces to Block (§2), applied once in the cascade.
    assert_eq!(s.display(first_tag(&doc, "div")), Display::Block);
    // An author rule on an `li` overrides its implicit ListItem.
    assert_eq!(s.display(first_tag(&doc, "li")), Display::Block);
}

#[test]
fn order_is_the_winning_integer_else_zero() {
    let (doc, s) = styles(
        "<a style='order:3'>x</a><b style='order:-2'>y</b>\
         <c style='order:nope'>z</c><d>w</d>",
    );
    // A winning integer declaration (positive and negative).
    assert_eq!(s.order(first_tag(&doc, "a")), 3);
    assert_eq!(s.order(first_tag(&doc, "b")), -2);
    // A non-integer value is dropped → 0.
    assert_eq!(s.order(first_tag(&doc, "c")), 0);
    // Absent → 0 (the initial value).
    assert_eq!(s.order(first_tag(&doc, "d")), 0);
}

#[test]
fn flex_direction_is_the_winning_keyword_else_row() {
    let (doc, s) = styles("<a style='flex-direction:column'>x</a><b>y</b>");
    // A winning declaration is parsed by FlexDirection::parse.
    assert_eq!(
        s.flex_direction(first_tag(&doc, "a")),
        FlexDirection::Column
    );
    // Absent → Row (the default main axis).
    assert_eq!(s.flex_direction(first_tag(&doc, "b")), FlexDirection::Row);
}

#[test]
fn noscript_is_hidden_only_when_js_ran() {
    use crate::css::compute_with;
    // js.md §4: scripting hides <noscript>. UA-implicit, so an author `display`
    // rule still overrides it.
    let doc = Document::parse("<noscript><p>fallback</p></noscript>");
    let ns = first_tag(&doc, "noscript");
    assert!(!compute_with(&doc, &[], false).display_none(ns));
    assert!(compute_with(&doc, &[], true).display_none(ns));
    let doc = Document::parse("<style>noscript{display:block}</style><noscript>x</noscript>");
    let ns = first_tag(&doc, "noscript");
    assert!(!compute_with(&doc, &[], true).display_none(ns));
}

#[test]
fn external_sheets_cascade_before_style_elements() {
    use crate::css::compute_with;
    // an external sheet alone hides the element.
    let doc = Document::parse("<p>x</p>");
    let s = compute_with(&doc, &["p{display:none}".to_string()], false);
    assert!(s.display_none(first_tag(&doc, "p")));
    // a later <style> rule of equal specificity wins over the external one.
    let doc = Document::parse("<style>p{display:block}</style><p>x</p>");
    let s = compute_with(&doc, &["p{display:none}".to_string()], false);
    assert!(!s.display_none(first_tag(&doc, "p")));
}
