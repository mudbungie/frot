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
        "getElementById getHTML mode delegatesFocus slotAssignment clonable serializable host \
         onslotchange innerHTML activeElement constructor | true",
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
