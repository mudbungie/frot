//! The Window interface and named properties (`bl-226d`): `window/probe.js`
//! run over `window/page.html` in frot, against Firefox 153.0esr's answers
//! to the same probe (`window/firefox153.json`, the identity.md §3.19 rig, 2026-11-24).

use serde_json::Value;

use super::drive_at;
use crate::dom::NodeKind;

const PAGE: &str = include_str!("window/page.html");
const PROBE: &str = include_str!("window/probe.js");
const CAPTURE: &str = include_str!("window/firefox153.json");

/// Keys frot answers differently, each a declared residual, with what frot
/// answers: the window's named properties are not built — a Proxy there makes
/// quickjs drop every ReferenceError (`window.js`; `bl-de60`) — so every
/// named lookup misses, `shadow`'s element-id `alert` among them (frot has no
/// `alert` either); the inert named properties object refuses a definition
/// in quickjs's words; and the window's own `window`/`self`/`document`/
/// `location`/`top` are data properties where Firefox's are accessors
/// (`bl-065f`).
const DECLARED: [(&str, &str); 9] = [
    (
        "named",
        "undefined,undefined,undefined,undefined,undefined,undefined,undefined,undefined,undefined",
    ),
    ("namedDesc", "none,none,false,-1,-1,false"),
    ("namedBare", "THROW ReferenceError:f is not defined"),
    (
        "namedDup",
        "THROW TypeError:cannot read property 'length' of undefined",
    ),
    ("namedLive", "undefined,false,undefined"),
    ("shadow", "undefined,5,false"),
    ("iframeNamed", "false,undefined"),
    ("defPP", "TypeError:object is not extensible"),
    (
        "globalThisSame",
        "true,true,true,true,vecw,vecw,vecw,gsc,ge",
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
fn every_captured_window_answer_matches_firefox_153() {
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
