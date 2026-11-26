//! The window's everyday members (`bl-d8f7`): `winmembers/probe.js`
//! run over `winmembers/page.html` in frot, against Firefox 153.0esr's answers
//! to the same probe (`winmembers/firefox153.json`, the identity.md §3.19 rig, 2026-11-24).

use serde_json::Value;

use super::drive_at;
use crate::dom::NodeKind;

const PAGE: &str = include_str!("winmembers/page.html");
const PROBE: &str = include_str!("winmembers/probe.js");
const CAPTURE: &str = include_str!("winmembers/firefox153.json");

/// Keys frot answers differently, each a declared residual, with what frot
/// answers: a popup is blocked (the rig's Marionette session allows them, so
/// Firefox opened one); `scrollX`/`scrollY`/`pageXOffset` are data properties
/// where Firefox's are accessors (`bl-065f`); and Selection carries only the
/// members an always-empty selection can answer (no Range is built).
const DECLARED: [(&str, &str); 5] = [
    ("openNoGesture", "null"),
    ("d_scrollX", "vecw"),
    ("d_scrollY", "vecw"),
    ("d_pageXOffset", "vecw"),
    (
        "selProto",
        "getRangeAt removeAllRanges empty containsNode toString anchorNode anchorOffset focusNode \
         focusOffset isCollapsed rangeCount type direction constructor",
    ),
];

fn run() -> Value {
    let page = PAGE.replace(
        "</body>",
        &format!(
            "<script>var resolve=function(o){{document.documentElement\
             .setAttribute('data-out',JSON.stringify(o))}};\n(function(){{{PROBE}\n}})()</script></body>"
        ),
    );
    let (doc, _) = drive_at(&page, "https://example.com/page.html");
    let html = doc.find_by_tag("html")[0];
    let NodeKind::Element(el) = &doc.node(html).kind else {
        unreachable!("html is an element")
    };
    serde_json::from_str(el.attr("data-out").unwrap()).unwrap()
}

#[test]
fn every_captured_window_member_answer_matches_firefox_153() {
    let got = run();
    let want: Value = serde_json::from_str(CAPTURE).unwrap();
    let mut wrong = Vec::new();
    for (key, value) in want.as_object().unwrap() {
        match DECLARED.iter().find(|d| d.0 == key) {
            Some((_, frot)) => assert_eq!(got[key], *frot, "{key}"),
            None if &got[key] != value => {
                wrong.push(format!("{key}: got {} want {value}", got[key]))
            }
            None => {}
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
