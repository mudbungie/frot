use super::State;
use crate::css::query_all;
use crate::dom::{Document, NodeKind};

/// The ids `selector` matches in `html`, in tree order.
fn ids(html: &str, selector: &str) -> String {
    let doc = Document::parse(html);
    let hits = query_all(&doc, Document::ROOT, selector).unwrap();
    let id = |&n: &u32| match &doc.node(n).kind {
        NodeKind::Element(e) => e.attr("id").unwrap_or("-").to_string(),
        other => panic!("matched a non-element: {other:?}"),
    };
    hits.iter().map(id).collect::<Vec<_>>().join(" ")
}

#[test]
fn only_state_pseudo_classes_are_named() {
    assert_eq!(State::named("link"), Some(State::AnyLink));
    assert_eq!(State::named("-webkit-any-link"), Some(State::AnyLink));
    assert_eq!(State::named("focus-visible"), Some(State::Focus));
    assert_eq!(State::named("target"), None);
}

#[test]
fn root_is_the_document_element_only() {
    let html = "<html id=h><body id=b><p id=p>x</p></body></html>";
    assert_eq!(ids(html, ":root"), "h");
    assert_eq!(ids(html, ":not(:root)[id]"), "b p");
}

#[test]
fn empty_ignores_comments_and_counts_whitespace() {
    let html = "<body><i id=a></i><i id=b><!-- c --></i><i id=c> </i><i id=d>t</i>\
                <i id=e><b id=f></b></i></body>";
    assert_eq!(ids(html, "i:empty, b:empty"), "a b f");
}

#[test]
fn a_link_is_an_anchor_or_area_with_href() {
    let html = "<a id=a href=x>l</a><a id=b>l</a><a id=c href>l</a><map><area id=d href=y>\
                <area id=e></map><link id=f href=x><svg><a id=g xlink:href=z></a></svg>";
    assert_eq!(ids(html, ":link"), "a c d g");
    assert_eq!(ids(html, ":any-link"), "a c d g");
    assert_eq!(ids(html, "a:not(:link)"), "b");
}

#[test]
fn checked_reads_checkboxes_and_radio_groups() {
    let html = "<form><input id=a type=CHECKBOX checked><input id=b type=checkbox>\
                <input id=c type=text checked><p id=p checked>x</p>\
                <input id=r1 type=radio name=g checked><input id=r2 type=radio name=g checked>\
                <input id=r3 type=radio name=g><input id=r4 type=radio name=h checked>\
                <input id=r5 type=radio name=g form=other checked>\
                <input id=r6 type=radio checked><input id=r7 type=radio name='' checked>\
                <input id=r8 type=radio checked><button id=x type=radio name=g checked></button>\
                </form><input id=r9 type=radio name=g checked><input id=r10 type=radio name=g checked>";
    // r5 names a form that is not there: it is unowned, like r9 and r10.
    assert_eq!(ids(html, ":checked"), "a r2 r4 r6 r7 r8 r10");
    // One group across the `form` attribute and ancestry; a second form with
    // the same id is another form.
    let html = "<form id=f><input type=radio name=g checked id=a></form>\
                <input type=radio name=g checked form=f id=b>\
                <input type=radio name=g checked form=nope id=c><input type=radio name=g checked id=d>\
                <input type=radio name=g checked form='' id=e>\
                <form id=f><input type=radio name=g checked id=h></form>";
    assert_eq!(ids(html, ":checked"), "b e h");
}

#[test]
fn checked_follows_the_option_a_select_picks() {
    let one = |body: &str| ids(&format!("<select {body}</select>"), ":checked");
    // No `selected`: the first option that is not off, in a drop-down only.
    assert_eq!(one(">text<option id=a>one<option id=b>"), "a");
    assert_eq!(
        one("><option id=a disabled><optgroup disabled><option id=b></optgroup><optgroup><option id=c></optgroup>"),
        "c"
    );
    assert_eq!(one("><option id=a disabled>"), "");
    assert_eq!(one("disabled><option id=a>"), "a");
    assert_eq!(one("size=1><option id=a>"), "a");
    assert_eq!(one("size=x><option id=a>"), "a");
    assert_eq!(one("size=3><option id=a>"), "");
    // `selected`: the last one wins, disabled or not, list box or not.
    assert_eq!(
        one("><option id=a selected><option id=b selected disabled><option id=c>"),
        "b"
    );
    assert_eq!(
        one("size=2><option id=a selected><option id=b selected>"),
        "b"
    );
    // `multiple`, and an option outside any select: its own attribute.
    assert_eq!(
        one("multiple><option id=a><option id=b selected><option id=c selected>"),
        "b c"
    );
    assert_eq!(
        ids(
            "<option id=a selected></option><option id=b></option>",
            ":checked"
        ),
        "a"
    );
}

#[test]
fn disabled_follows_fieldsets_and_selects() {
    let html = "<input id=a disabled><input id=b><p id=p disabled>x</p>\
                <fieldset id=f disabled><legend id=l><span><input id=c></span></legend>\
                <legend><input id=d></legend>text<button id=e></button>\
                <fieldset id=g><textarea id=h></textarea></fieldset><a id=i href=x>l</a></fieldset>\
                <fieldset id=j><select id=k><option id=o1></select></fieldset>\
                <select id=m disabled><optgroup id=n><option id=o2></optgroup></select>\
                <select id=q><optgroup id=r disabled><option id=o3></optgroup><option id=o4 disabled>\
                <option id=o5></select><option id=o6></option>\
                <fieldset id=s disabled>text<i></i><input id=t></fieldset>";
    assert_eq!(ids(html, ":disabled"), "a f d e g h m n o2 r o3 o4 s t");
    assert_eq!(ids(html, ":enabled"), "b c j k o1 q o5 o6");
}

#[test]
fn required_applies_to_the_types_that_take_a_value() {
    let html =
        "<input id=a required><input id=b type=EMAIL required><input id=c type=hidden required>\
                <input id=d type=range required><input id=e type=button required><input id=f>\
                <select id=g required></select><textarea id=h required></textarea>\
                <button id=i required></button><p id=j required>x</p><select id=k></select>";
    assert_eq!(ids(html, ":required"), "a b g h");
    assert_eq!(ids(html, ":optional"), "c d e f i k");
}

#[test]
fn read_write_is_an_editable_text_control_or_editable_content() {
    let html = "<input id=a><input id=b readonly><input id=c disabled><input id=d type=checkbox>\
                <input id=e type=color><input id=f type=bogus><textarea id=g></textarea>\
                <textarea id=h readonly></textarea><p id=i>x</p>\
                <div id=j contenteditable><b id=k></b><i id=l contenteditable=FALSE><u id=m></u>\
                <s id=n contenteditable=plaintext-only></s></i><input id=o readonly></div>\
                <div id=p contenteditable=true></div><div id=q contenteditable=inherit></div>";
    assert_eq!(ids(html, ":read-write"), "a f g j k n p");
    assert_eq!(ids(html, "[id]:read-only"), "b c d e h i l m o q");
}
