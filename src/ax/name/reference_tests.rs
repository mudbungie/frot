//! Names taken *from another element*: an `aria-labelledby` target, a
//! `<label>`, a `<legend>`. Each case is a Chrome 139 measurement
//! (`Accessibility.getFullAXTree`, `--headless=new`), taken in `bl-0482`.
//!
//! The probe shape is `<div role=button aria-labelledby=t>fallback</div>`: a
//! reference that resolves to nothing leaves the button named "fallback", so a
//! silent failure is never mistaken for a pass.

use super::*;

fn name(html: &str, tag: &str) -> Option<String> {
    let d = Document::parse(html);
    accessible_name(&d, *d.find_by_tag(tag).first().unwrap(), None)
}

fn name_css(html: &str, tag: &str) -> Option<String> {
    let d = Document::parse(html);
    let s = crate::css::compute(&d);
    accessible_name(&d, *d.find_by_tag(tag).first().unwrap(), Some(&s))
}

/// The defect: the target's own alternative was the one thing never read, so
/// the walk descended past a replaced element into nothing.
#[test]
fn a_target_speaks_its_own_alternative() {
    // Chrome: "ALT1".
    assert_eq!(
        name(
            "<div role=button aria-labelledby=t>fb</div><img id=t alt=ALT>",
            "div"
        ),
        Some("ALT".into())
    );
    // Chrome: "VAL9" — an embedded control speaks its value.
    assert_eq!(
        name(
            "<div role=button aria-labelledby=t>fb</div><input id=t value=VAL>",
            "div"
        ),
        Some("VAL".into())
    );
    // Chrome: "IMGIN30".
    assert_eq!(
        name(
            "<div role=button aria-labelledby=t>fb</div><input id=t type=image alt=SORT>",
            "div"
        ),
        Some("SORT".into())
    );
    // Chrome: "S10b" — a `<select>` speaks its selected option.
    assert_eq!(
        name(
            "<div role=button aria-labelledby=t>fb</div>\
             <select id=t><option>S</option><option selected>M</option></select>",
            "div"
        ),
        Some("M".into())
    );
    // Chrome: "AL4" — `aria-label` on the target is still step 2C.
    assert_eq!(
        name(
            "<div role=button aria-labelledby=t>fb</div><span id=t aria-label=AL>TXT</span>",
            "div"
        ),
        Some("AL".into())
    );
}

/// The filed case: the reference stopped *at* the group in Chrome and ran past
/// it in frot, naming the button after the option instead.
#[test]
fn a_target_is_not_descended_past_into_its_own_options() {
    let select = "<select><optgroup id=g label=GL><option id=o label=OL>ot</option>\
                  </optgroup></select>";
    // Chrome: "GL".
    assert_eq!(
        name(
            &format!("<div role=button aria-labelledby=g>fb</div>{select}"),
            "div"
        ),
        Some("GL".into())
    );
    // Chrome: "OL" — and frot used to emit no name at all here.
    assert_eq!(
        name(
            &format!("<div role=button aria-labelledby=o>fb</div>{select}"),
            "div"
        ),
        Some("OL".into())
    );
}

/// `alt=""` is the *empty* alternative, so the reference resolves to nothing
/// and the referring element falls back to its own contents (Chrome: "fb").
#[test]
fn an_empty_alternative_resolves_the_reference_to_nothing() {
    assert_eq!(
        name(
            "<div role=button aria-labelledby=t>fb</div><img id=t alt=''>",
            "div"
        ),
        Some("fb".into())
    );
}

/// §2B does not recurse: one hop is the whole rule. Chrome names the button
/// "TXT5" — the target's contents — not what the target's own reference says.
#[test]
fn a_reference_does_not_follow_a_second_reference() {
    let html = "<div role=button aria-labelledby=a>fb</div>\
                <span id=a aria-labelledby=b>TXT</span><span id=b>CHAIN</span>";
    assert_eq!(name(html, "div"), Some("TXT".into()));
    // Nor from *below* the target: one traversal is one traversal. Chrome
    // names this button "AINNER31B".
    let html = "<div role=button aria-labelledby=a>fb</div>\
                <span id=a>A<span aria-labelledby=b>IN</span>B</span><span id=b>REF</span>";
    assert_eq!(name(html, "div"), Some("AINB".into()));
    // A descendant's `aria-label` *is* read, though (Chrome: "P Q43").
    let html = "<div role=button aria-labelledby=a>fb</div>\
                <span id=a>P<span aria-label=Q>z</span></span>";
    assert_eq!(name(html, "div"), Some("P Q".into()));
}

/// Which is also why a self-reference terminates without needing the cycle
/// guard to be seeded with the target: Chrome names this button "SELF6".
#[test]
fn a_self_reference_names_from_contents() {
    assert_eq!(
        name("<div id=s role=button aria-labelledby=s>SELF</div>", "div"),
        Some("SELF".into())
    );
    // And a target that *contains* the element it names (Chrome: "OUT36 IN36").
    let html = "<span id=t>OUT <div role=button aria-labelledby=t>IN</div></span>";
    assert_eq!(name(html, "div"), Some("OUT IN".into()));
}

/// §2A excludes hidden nodes "unless directly referenced". Chrome names the
/// button after the target in all three flavours of hidden.
#[test]
fn a_directly_referenced_target_is_read_even_when_hidden() {
    let html = "<div role=button aria-labelledby=t>fb</div>\
                <span id=t style='display:none'>HID</span>";
    assert_eq!(name_css(html, "div"), Some("HID".into()));
    let html = "<div role=button aria-labelledby=t>fb</div>\
                <span id=t style='visibility:hidden'>VH</span>";
    assert_eq!(name_css(html, "div"), Some("VH".into()));
    // Chrome: "AH41" — even an `aria-hidden` replaced element.
    let html = "<div role=button aria-labelledby=t>fb</div><img id=t aria-hidden=true alt=AH>";
    assert_eq!(name(html, "div"), Some("AH".into()));
    // The exemption is spent on the target: a hidden *child* of it is still
    // dropped (Chrome: "K").
    let html = "<div role=button aria-labelledby=t>fb</div>\
                <span id=t>K<span aria-hidden=true>HID</span></span>";
    assert_eq!(name(html, "div"), Some("K".into()));
    // It applies wherever the reference is written, not only at the top: a
    // referring *descendant* gets it too (Chrome names this button "H50").
    let html = "<style>.h{display:none}</style><button><span aria-labelledby=t>x</span></button>\
                <span id=t class=h>H50</span>";
    assert_eq!(name_css(html, "button"), Some("H50".into()));
}

/// A `<legend>` and a `<label>` enter by the same door, so their own
/// alternative stops the walk exactly as a target's does.
#[test]
fn a_legend_and_a_label_speak_their_own_alternative() {
    // Chrome: fieldset "LA15".
    let html = "<fieldset><legend aria-label=LA>LT</legend><input></fieldset>";
    assert_eq!(name(html, "fieldset"), Some("LA".into()));
    // Chrome: fieldset "REF20" — a legend is *not* inside a reference, so its
    // own `aria-labelledby` is followed.
    let html = "<fieldset><legend aria-labelledby=t>LT</legend><input></fieldset>\
                <span id=t>REF</span>";
    assert_eq!(name(html, "fieldset"), Some("REF".into()));
    // Chrome: input "LB16".
    let html = "<label for=q aria-label=LB>LT</label><input id=q>";
    assert_eq!(name(html, "input"), Some("LB".into()));
    // Chrome: input "LB21" — the wrapping label too.
    let html = "<label aria-label=LB>LT <input value=V></label>";
    assert_eq!(name(html, "input"), Some("LB".into()));
    // Chrome: input "REF42".
    let html = "<label for=q aria-labelledby=t>LT</label><input id=q><span id=t>REF</span>";
    assert_eq!(name(html, "input"), Some("REF".into()));
}

/// …and, unlike a referenced target, a hidden one names nothing: Chrome gives
/// both the control and the fieldset an empty name.
#[test]
fn a_hidden_label_or_legend_names_nothing() {
    let html = "<style>.h{display:none}</style>\
                <label for=q class=h>LT</label><input id=q>";
    assert_eq!(name_css(html, "input"), None);
    let html = "<style>.h{display:none}</style>\
                <fieldset><legend class=h>LEG</legend><input></fieldset>";
    assert_eq!(name_css(html, "fieldset"), None);
}
