//! Forms and the document's address (`bl-09c8`): `form/probe.js` run over
//! `form/page.html` in frot, against Firefox 153.0esr's answers to the same
//! probe (`form/firefox153.json`, the identity.md §3.19 rig, 2026-11-24).

use serde_json::Value;

use super::drive_at;
use crate::dom::NodeKind;

const PAGE: &str = include_str!("form/page.html");
const PROBE: &str = include_str!("form/probe.js");
const CAPTURE: &str = include_str!("form/firefox153.json");

/// Keys frot answers differently, each a declared residual, with what frot
/// answers: a collection is a snapshot, so `document.forms` and `elements` are
/// a new object per read and `forms` does not grow (js.md §3, "DOM
/// collections"); named access on a form and the document is not built
/// (`bl-de60`), nor on the window (`bl-226d`).
const DECLARED: [(&str, &str); 6] = [
    (
        "elements",
        "8|[object HTMLFormControlsCollection]|qq/fs/s/ta/b/o/ob/os|false|8",
    ),
    ("forms", "4|[object HTMLCollection]|false|f|f|g"),
    ("formsLive", "4>4"),
    (
        "formNamed",
        "undefined|undefined|undefined|undefined|undefined",
    ),
    ("docNamed", "undefined|undefined|undefined|undefined"),
    (
        "winNamed",
        "undefined|undefined|undefined|undefined|undefined|undefined",
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
fn every_captured_form_answer_matches_firefox_153() {
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
