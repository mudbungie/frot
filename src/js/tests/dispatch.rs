//! Event dispatch against the real binary (`bl-810b`): `dispatch/probe.js`
//! listens at both phases on every object from the window down to a nested
//! element and dispatches through it — bubbling or not, stopped at each hop,
//! canceled, passive, `once`, re-entered, on a detached subtree, a fragment and
//! a text node — then pokes the Event and EventTarget shapes and argument
//! errors; `dispatch/firefox153.json` is what Firefox 153.0esr answered
//! (identity.md §3.19 rig, over `dispatch/page.html`). frot must answer every
//! key identically but the declared one below. The capture is the oracle.

use serde_json::Value;

use super::{drive, sess};
use crate::dom::NodeKind;

const PROBE: &str = include_str!("dispatch/probe.js");
const PAGE: &str = include_str!("dispatch/page.html");
const CAPTURE: &str = include_str!("dispatch/firefox153.json");

/// `throwing`: the reported exception's `filename` is the throwing script's
/// URL in Firefox; frot reports it as `''` (and `lineno`/`colno` 0) — its
/// engine errors carry no script URL. `timeStamp`: this test's session runs on
/// a frozen clock (`sess`), so `performance.now()`, and the stamp, read 0.
const DECLARED: [&str; 2] = ["throwing", "timeStamp"];

#[test]
fn every_captured_dispatch_answers_as_firefox_153_does() {
    let s = sess(PAGE);
    let got = s
        .eval(&format!(
            "(function(){{var r;var resolve=function(o){{r=o}};{PROBE}\nreturn JSON.stringify(r)}})()"
        ))
        .unwrap();
    let got: Value = serde_json::from_str(&got).unwrap();
    let want: Value = serde_json::from_str(CAPTURE).unwrap();
    let want = want.as_object().unwrap();
    assert_eq!(want.len(), 63);
    let mut wrong = Vec::new();
    for (key, value) in want {
        if !DECLARED.contains(&key.as_str()) && &got[key] != value {
            wrong.push(format!("{key}: got {} want {value}", got[key]));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert_eq!(
        got["throwing"],
        "1,2;true;ErrorEvent/Error: boom/boom//true/true"
    );
    assert_eq!(got["timeStamp"], "numberfalsetrue");
}

/// What frot itself fires takes the same path (`dispatch/life.html`, read on
/// the same rig): readystatechange at the document (its path reaching the
/// window, which hears only the capture pass), DOMContentLoaded bubbling back
/// up to the window, each <img>'s load/error captured on the document — and
/// on the window for `error` alone — and the window's `load` dispatched at the
/// window with the document as its target. All trusted.
#[test]
fn the_host_fires_through_the_same_path() {
    let (doc, report) = drive(include_str!("dispatch/life.html"));
    assert_eq!(report.errors, 0);
    let html = doc.find_by_tag("html")[0];
    let NodeKind::Element(el) = &doc.node(html).kind else {
        unreachable!("html is an element")
    };
    let got: Value = serde_json::from_str(el.attr("data-out").unwrap()).unwrap();
    let want = [
        "winC:readystatechange:1:doc:true:false:false:doc/win",
        "docC:readystatechange:2:doc:true:false:false:doc/win",
        "docB:readystatechange:2:doc:true:false:false:doc/win",
        "winC:DOMContentLoaded:1:doc:true:true:false:doc/win",
        "docC:DOMContentLoaded:2:doc:true:true:false:doc/win",
        "docB:DOMContentLoaded:2:doc:true:true:false:doc/win",
        "winB:DOMContentLoaded:3:doc:true:true:false:doc/win",
        "docC:load:1:g:true:false:false:g///doc",
        "winC:error:1:x:true:false:false:x///doc/win",
        "docC:error:1:x:true:false:false:x///doc/win",
        "winC:readystatechange:1:doc:true:false:false:doc/win",
        "docC:readystatechange:2:doc:true:false:false:doc/win",
        "docB:readystatechange:2:doc:true:false:false:doc/win",
        "winC:load:2:doc:true:false:false:win",
        "winB:load:2:doc:true:false:false:win",
    ];
    assert_eq!(got, serde_json::json!(want));
}
