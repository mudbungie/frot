//! ElementInternals against the real binary (js.md §15.6):
//! `internals/probe.js` attaches internals to form-associated and plain custom
//! elements, logs their form reactions, sets validity, states and ARIA
//! defaults; `internals/firefox153.json` is what Firefox 153.0esr answered
//! (identity.md §3.19 rig, over `internals/page.html`). frot must answer every
//! key the same but `reset`: frot has no `HTMLFormElement.reset()`
//! (js.md §15.6). The capture, not this file, is the oracle.

use serde_json::Value;

use super::sess;

const PROBE: &str = include_str!("internals/probe.js");
const PAGE: &str = include_str!("internals/page.html");
const CAPTURE: &str = include_str!("internals/firefox153.json");

const DECLARED: [(&str, &str); 1] = [("reset", "THROW TypeError: not a function")];

#[test]
fn every_internals_answer_is_firefox_153s() {
    let got = sess(PAGE)
        .eval(&format!(
            "(function(){{var r;var resolve=function(o){{r=o}};{PROBE}\nreturn JSON.stringify(r)}})()"
        ))
        .unwrap();
    let got: Value = serde_json::from_str(&got).unwrap();
    let want: Value = serde_json::from_str(CAPTURE).unwrap();
    let want = want.as_object().unwrap();
    assert_eq!(want.len(), 53);
    let mut wrong = Vec::new();
    for (key, value) in want {
        let declared = DECLARED.iter().find(|d| d.0 == key).map(|d| d.1);
        let want = declared.map_or(value.clone(), Value::from);
        if got[key] != want {
            wrong.push(format!("{key}: got {} want {want}", got[key]));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
