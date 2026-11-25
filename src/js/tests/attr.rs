//! An element's attribute list beyond get/set (js.md §15): `getAttributeNames`,
//! `hasAttributes`, `toggleAttribute`, `localName`, and `attributes` with its
//! `Attr` handles, against `attr/firefox153.json` — what Firefox 153.0esr
//! answered to `attr/probe.js` over `attr/page.html` (identity.md §3.19 rig).
//! frot must answer every key the same but `nnmProto`: frot builds the read
//! half of `NamedNodeMap` only (js.md §15.3).

use serde_json::Value;

use super::sess;

const PROBE: &str = include_str!("attr/probe.js");
const PAGE: &str = include_str!("attr/page.html");
const CAPTURE: &str = include_str!("attr/firefox153.json");

#[test]
fn every_attribute_list_answer_is_firefox_153s() {
    let got = sess(PAGE)
        .eval(&format!(
            "(function(){{var r;var resolve=function(o){{r=o}};{PROBE}\nreturn JSON.stringify(r)}})()"
        ))
        .unwrap();
    let got: Value = serde_json::from_str(&got).unwrap();
    let want: Value = serde_json::from_str(CAPTURE).unwrap();
    let want = want.as_object().unwrap();
    assert_eq!(want.len(), 17);
    for (key, value) in want {
        if key == "nnmProto" {
            assert_eq!(got[key], "getNamedItem item length constructor");
        } else {
            assert_eq!(&got[key], value, "{key}");
        }
    }
}
