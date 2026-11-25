//! The user agent's own `display: none` rules: the `hidden` attribute, a
//! hidden input, `<datalist>`, and what a `<select>` shows.

use super::tests::{first_tag, styles};
use crate::css::Display;

#[test]
fn hidden_attribute_is_ua_display_none_and_author_overridable() {
    // HTML's UA sheet: `[hidden] { display: none }` (`bl-eeb4`). Its value is
    // noise — `hidden` is a boolean attribute.
    let (doc, s) = styles("<div hidden>x</div>");
    assert!(s.display_none(first_tag(&doc, "div")));
    let (doc, s) = styles("<div hidden='until-found'>x</div>");
    assert!(s.display_none(first_tag(&doc, "div")));
    // UA origin loses to any author or inline declaration.
    let (doc, s) = styles("<style>div{display:block}</style><div hidden>x</div>");
    assert!(!s.display_none(first_tag(&doc, "div")));
    let (doc, s) = styles("<div hidden style='display:block'>x</div>");
    assert!(!s.display_none(first_tag(&doc, "div")));
    // The UA rule is HTML's: SVG and MathML ignore the attribute, roots
    // included, and HTML inside a `<foreignObject>` keeps it (`bl-7d99`).
    let (doc, s) = styles(
        "<svg hidden><text hidden>t</text><foreignObject><div hidden>d</div></foreignObject></svg>\
         <math><mi hidden>x</mi></math>",
    );
    for (tag, none) in [
        ("svg", false),
        ("text", false),
        ("mi", false),
        ("div", true),
    ] {
        assert_eq!(s.display_none(first_tag(&doc, tag)), none, "{tag}");
    }
}

#[test]
fn hidden_input_is_ua_display_none_and_not_author_overridable() {
    // The other half of the UA "hidden elements" block, and the `!important`
    // one (`bl-a189`): wordpress.org's six `type=hidden` inputs get no boxes in
    // Chrome, whatever the page's CSS says.
    let (doc, s) = styles("<input type=hidden>");
    assert!(s.display_none(first_tag(&doc, "input")));
    let (doc, s) = styles("<input type=HIDDEN>");
    assert!(s.display_none(first_tag(&doc, "input")));
    let (doc, s) = styles("<style>input{display:block}</style><input type=hidden>");
    assert!(s.display_none(first_tag(&doc, "input")));
    let (doc, s) = styles("<input type=hidden style='display:block'>");
    assert!(s.display_none(first_tag(&doc, "input")));
    // Every other input type still renders.
    let (doc, s) = styles("<input type=text>");
    assert!(!s.display_none(first_tag(&doc, "input")));
}

#[test]
fn datalist_is_ua_display_none_and_author_overridable() {
    // The one tag in the UA-implicit display map that computes `none`
    // (`datalist { display: none }`, `bl-66ed`). Measured as a *declaration*
    // rather than a structure: the author rule below makes Chrome paint the
    // suggestions, list them in `innerText`, and stop ignoring them in AX.
    let (doc, s) = styles("<datalist id=dl><option value=alpha>Alpha</option></datalist>");
    assert!(s.display_none(first_tag(&doc, "datalist")));
    // Its options are ordinary block boxes; only the parent's `none` hides them.
    assert_eq!(s.display(first_tag(&doc, "option")), Display::Block);
    let (doc, s) = styles("<style>datalist{display:block}</style><datalist id=dl>x</datalist>");
    assert!(!s.display_none(first_tag(&doc, "datalist")));
    let (doc, s) = styles("<datalist id=dl style='display:block'>x</datalist>");
    assert!(!s.display_none(first_tag(&doc, "datalist")));
}

#[test]
fn select_subtree_ua_display_matches_chrome() {
    // `<option>`/`<optgroup>` compute `block`, `<select>` `inline-block` —
    // which frot has no variant for, so it is the inline-level default
    // (`layout.md` §2.4; `bl-66ed`).
    let (doc, s) = styles("<select><optgroup label=G><option>a</option></optgroup></select>");
    assert_eq!(s.display(first_tag(&doc, "option")), Display::Block);
    assert_eq!(s.display(first_tag(&doc, "optgroup")), Display::Block);
    assert_eq!(s.display(first_tag(&doc, "select")), Display::Inline);
}
