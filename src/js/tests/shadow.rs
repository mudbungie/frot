//! Shadow roots and slots against the real binary (js.md §15):
//! `shadow/probe.js` reads declarative roots the parser made (and the ones it
//! refused), attaches roots, assigns slots, serializes, clones and misuses
//! them; `shadow/firefox153.json` is what Firefox 153.0esr answered
//! (identity.md §3.19 rig, over `shadow/page.html`). frot must answer every key
//! the same but the two prototype listings, where frot builds a subset of
//! Firefox's members in Firefox's order (js.md §15.3). The capture, not this
//! file, is the oracle.

use serde_json::Value;

use super::sess;

const PROBE: &str = include_str!("shadow/probe.js");
const PAGE: &str = include_str!("shadow/page.html");
const CAPTURE: &str = include_str!("shadow/firefox153.json");

const DECLARED: [(&str, &str); 2] = [
    (
        "protoSR",
        "getElementById setHTMLUnsafe getHTML mode delegatesFocus slotAssignment clonable serializable host \
         onslotchange innerHTML activeElement adoptedStyleSheets constructor | true",
    ),
    (
        "protoSlot",
        "assignedNodes assignedElements name constructor",
    ),
];

#[test]
fn every_shadow_answer_is_firefox_153s() {
    let got = sess(PAGE)
        .eval(&format!(
            "(function(){{var r;var resolve=function(o){{r=o}};{PROBE}\nreturn JSON.stringify(r)}})()"
        ))
        .unwrap();
    let got: Value = serde_json::from_str(&got).unwrap();
    let want: Value = serde_json::from_str(CAPTURE).unwrap();
    let want = want.as_object().unwrap();
    assert_eq!(want.len(), 49);
    for (key, value) in want {
        let declared = DECLARED.iter().find(|d| d.0 == key).map(|d| d.1);
        assert_eq!(
            got[key],
            declared.map_or(value.clone(), Value::from),
            "{key}"
        );
    }
}

/// A script inserted into a shadow tree is connected and runs once; one in a
/// detached host's tree does not (Firefox 153.0esr answers `[1,null]`).
#[test]
fn a_script_inserted_into_a_connected_shadow_tree_runs() {
    let s = sess("<html><body></body></html>");
    let got = s
        .eval(
            "var d=document.createElement('div');document.body.appendChild(d);\
             var sr=d.attachShadow({mode:'open'});var s=document.createElement('script');\
             s.textContent='window.ran=(window.ran||0)+1';sr.appendChild(s);\
             var e=document.createElement('div');var r2=e.attachShadow({mode:'open'});\
             var s2=document.createElement('script');s2.textContent='window.det=1';\
             r2.appendChild(s2);JSON.stringify([window.ran,window.det])",
        )
        .unwrap();
    assert_eq!(got, "[1,null]");
}
