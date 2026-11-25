//! HTMLScriptElement's members (`bl-f9f2`) against Firefox 153.0esr:
//! `tests/fixtures/js/script-probe.js` was served to the binary as `/probe.js`
//! (identity.md §3.19 rig) and its answers captured in
//! `script-firefox153.json`; here a local server serves it to frot.

use mockito::Server;
use serde_json::{Map, Value};

use super::drive_at;
use crate::dom::NodeKind;

const PROBE: &str = include_str!("../../../tests/fixtures/js/script-probe.js");
const FIREFOX: &str = include_str!("../../../tests/fixtures/js/script-firefox153.json");
const PAGE: &str = "<!doctype html><html><head><meta charset=utf-8><script>\
    window.done=function(v){document.documentElement.setAttribute('data-out',JSON.stringify(v))};\
    </script><script id=pa async src=\"data:text/javascript,1\"></script>\
    <script id=pd defer src=\"data:text/javascript,2\"></script><script src=\"/probe.js\"></script>\
    </head><body>x</body></html>";

/// Where frot answers otherwise, each declared (js.md §4.7), pinned to frot's
/// answer.
const RESIDUAL: [(&str, &str); 2] = [
    // `innerText`/`textContent` are not redefined on the script interface.
    (
        "proto",
        "src,type,noModule,charset,async,defer,crossOrigin,referrerPolicy,text,fetchPriority,\
         event,htmlFor,integrity,constructor",
    ),
    // frot resolves no import map, so `supports('importmap')` is false.
    (
        "statics",
        "length,name,prototype,supports|function|true,true,false,false,false",
    ),
];

#[test]
fn script_members_answer_as_firefox_does() {
    let mut server = Server::new();
    let _m = server
        .mock("GET", "/probe.js")
        .with_header("content-type", "text/javascript")
        .with_body(PROBE)
        .create();
    let (doc, report) = drive_at(PAGE, &format!("{}/p.html", server.url()));
    assert_eq!(report.errors, 0);
    let html = doc.find_by_tag("html")[0];
    let NodeKind::Element(el) = &doc.node(html).kind else {
        unreachable!("html is an element")
    };
    let got: Map<String, Value> = serde_json::from_str(el.attr("data-out").unwrap()).unwrap();
    let mut want: Map<String, Value> = serde_json::from_str(FIREFOX).unwrap();
    for (k, v) in RESIDUAL {
        want.insert(k.into(), Value::from(v));
    }
    assert_eq!(got, want);
}
