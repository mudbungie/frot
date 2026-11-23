use super::number;
use crate::css::query_all;
use crate::dom::{Document, NodeKind};

fn ids(html: &str, selector: &str) -> String {
    let doc = Document::parse(html);
    let hits = query_all(&doc, Document::ROOT, selector).unwrap();
    let id = |&n: &u32| match &doc.node(n).kind {
        NodeKind::Element(e) => e.attr("id").unwrap_or("-").to_string(),
        other => panic!("matched a non-element: {other:?}"),
    };
    let named = hits.iter().map(id).filter(|i| i != "-");
    named.collect::<Vec<_>>().join(" ")
}

// Measured in Chrome 158 (`bl-2a85`).
const PLACEHOLDERS: &str = "<input id=ph1 placeholder=x><input id=ph2 placeholder=''>\
    <input id=ph3 placeholder=x value=v><input id=ph4 placeholder=x value=''>\
    <input id=ph5 type=checkbox placeholder=x><input id=ph6 type=number placeholder=x>\
    <input id=ph7 type=date placeholder=x><input id=ph8 type=hidden placeholder=x>\
    <input id=ph9 type=PASSWORD placeholder=x><input id=ph10 placeholder=' '>\
    <textarea id=ta1 placeholder=x></textarea><textarea id=ta2 placeholder=x>t</textarea>\
    <textarea id=ta3 placeholder=x>\n</textarea><input id=ph12 type=range placeholder=x>\
    <input id=ph16 type=bogus placeholder=x><input id=ph18 type=submit placeholder=x>\
    <input id=ph19 disabled placeholder=x><select id=ph21 placeholder=x></select><input id=ph22>\
    <input id=s1 type=number placeholder=x value=abc><input id=s2 type=email placeholder=x value='  '>\
    <input id=s3 placeholder=x value='&#10;'><input id=s4 type=number placeholder=x value=1e3>\
    <input id=s5 type=url placeholder=x value=' u '><input id=s6 placeholder=x value=' '>";

#[test]
fn placeholder_shown_wants_a_placeholder_and_an_empty_value() {
    assert_eq!(
        ids(PLACEHOLDERS, ":placeholder-shown"),
        "ph1 ph2 ph4 ph6 ph9 ph10 ta1 ta3 ph16 ph19 s1 s2 s3"
    );
}

#[test]
fn a_valid_number_is_a_decimal_with_an_optional_exponent() {
    for yes in ["0", "-1", "1.5", ".5", "1e3", "1E-3", "-.5e+2"] {
        assert!(number(yes), "{yes}");
    }
    for no in [
        "", "-", "1.", ".", "+1", "1e", "1e+", "1ex", "abc", " 1", "inf", "1..2",
    ] {
        assert!(!number(no), "{no}");
    }
}

const DEFAULTS: &str = "<form id=f1><input id=d1 type=checkbox checked>\
    <input id=d2 type=radio name=r checked><input id=d3 type=radio name=r>\
    <select><option id=o1>a</option><option id=o2 selected>b</option></select>\
    <select><option id=o3>a</option></select><button id=b0 type=button>x</button>\
    <input id=b1 type=submit><button id=b2>y</button><input id=b3 type=image></form>\
    <form id=f2><button id=b4 type=reset>r</button><button id=b5 disabled>d</button>\
    <input id=b6 type=submit></form><button id=b7>outside</button>\
    <button id=b8 type=bogus form=f3>early</button><form id=f3><input id=b9 type=image></form>";

#[test]
fn default_is_the_checked_the_selected_and_each_forms_first_submit() {
    assert_eq!(ids(DEFAULTS, ":default"), "d1 d2 o2 b1 b5 b8");
}

const RADIOS: &str = "<form id=f1><input id=x type=radio name=g></form>\
    <progress id=p1></progress><progress id=p2 value=3></progress><progress id=p3 value=x></progress>\
    <input id=i1 type=radio name=g><input id=i2 type=radio name=g>\
    <input id=i3 type=radio name=h checked><input id=i4 type=radio name=h>\
    <input id=i5 type=radio><input id=i8 type=radio checked><input id=i6 type=checkbox>\
    <input id=i7 type=radio name=g form=f1><meter id=m1></meter><div id=i9 type=radio></div>";

#[test]
fn indeterminate_is_a_valueless_progress_or_a_group_nobody_checked() {
    assert_eq!(ids(RADIOS, ":indeterminate"), "x p1 i1 i2 i5 i7");
}

const CUSTOM: &str = "<my-el id=c1></my-el><x-y id=c2></x-y><font-face id=c3></font-face>\
    <div id=c5 is=x-y></div><div id=c6 is=''></div><x id=c7></x><a-b- id=c9></a-b->\
    <a-\u{e4} id=c11></a-\u{e4}><a-b.c id=c12></a-b.c><a-b_1 id=c13></a-b_1><x-y! id=c14></x-y!>\
    <svg><g id=s1 is=x-y></g><x-y id=s2></x-y></svg><math><x-y id=s3></x-y></math>";

#[test]
fn defined_is_every_element_but_an_unregistered_custom_one() {
    assert_eq!(
        ids(CUSTOM, ":not(:defined)"),
        "c1 c2 c5 c6 c9 c11 c12 c13 c14"
    );
    assert_eq!(ids(CUSTOM, "[id]:defined"), "c3 c7 s1 s2 s3");
}

#[test]
fn open_paused_and_muted_read_attributes() {
    let html = "<details id=o1 open></details><details id=o2></details><dialog id=o3 open=false>\
        </dialog><dialog id=o4></dialog><select id=o5></select><video id=v1></video>\
        <audio id=v2 muted></audio><div id=v3 muted></div>";
    assert_eq!(ids(html, ":open"), "o1 o3");
    assert_eq!(ids(html, ":paused"), "v1 v2");
    assert_eq!(ids(html, ":muted"), "v2");
    assert_eq!(
        ids(
            html,
            ":playing, :seeking, :buffering, :stalled, :volume-locked"
        ),
        ""
    );
    assert_eq!(ids(html, "video:not(:playing)"), "v1");
}
