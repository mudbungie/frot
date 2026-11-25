//! Custom elements against the real binary (js.md §15): `customel/probe.js`
//! defines, upgrades, creates, clones, moves and misuses custom elements and
//! logs every reaction in the order it ran; `customel/firefox153.json` is what
//! Firefox 153.0esr answered (identity.md §3.19 rig, over `customel/page.html`).
//! frot must answer every key the same but the two declared below. The
//! capture, not this file, is the oracle.

use serde_json::Value;

use super::sess;

const PROBE: &str = include_str!("customel/probe.js");
const PAGE: &str = include_str!("customel/page.html");
const CAPTURE: &str = include_str!("customel/firefox153.json");

/// Firefox never upgrades an element `innerHTML` made, whether the definition
/// came first (once any earlier upgrade in the realm has failed) or later and
/// `customElements.upgrade` asks; Chrome and the HTML standard upgrade both,
/// and so does frot — these are Chrome 158's answers (js.md §15.3).
const DECLARED: [(&str, &str); 2] = [
    ("upgradeFn", "vctor ||  true"),
    ("upgradeLate", "wctor true"),
];

#[test]
fn every_custom_element_answer_is_firefox_153s() {
    let s = sess(PAGE);
    s.eval(&format!(
        "var __r;(function(){{var resolve=function(o){{__r=o}};{PROBE}\n}})()"
    ))
    .unwrap();
    let got: Value = serde_json::from_str(&s.eval("JSON.stringify(__r)").unwrap()).unwrap();
    let want: Value = serde_json::from_str(CAPTURE).unwrap();
    let want = want.as_object().unwrap();
    assert_eq!(want.len(), 46);
    for (key, value) in want {
        let chrome = DECLARED.iter().find(|d| d.0 == key).map(|d| d.1);
        assert_eq!(got[key], chrome.map_or(value.clone(), Value::from), "{key}");
    }
}
