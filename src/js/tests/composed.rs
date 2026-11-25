//! Events across shadow boundaries against the real binary (js.md §15.7):
//! `composed/probe.js` dispatches composed and uncomposed, bubbling and
//! non-bubbling events from open, closed, nested and slotted positions and
//! logs every listener's phase, target and `composedPath()`;
//! `composed/firefox153.json` is what Firefox 153.0esr answered (identity.md
//! §3.19 rig, over `composed/page.html`). frot must answer every key the same.

use serde_json::Value;

use super::sess;

const PROBE: &str = include_str!("composed/probe.js");
const PAGE: &str = include_str!("composed/page.html");
const CAPTURE: &str = include_str!("composed/firefox153.json");

#[test]
fn every_composed_answer_is_firefox_153s() {
    let got = sess(PAGE)
        .eval(&format!(
            "(function(){{var r;var resolve=function(o){{r=o}};{PROBE}\nreturn JSON.stringify(r)}})()"
        ))
        .unwrap();
    let got: Value = serde_json::from_str(&got).unwrap();
    let want: Value = serde_json::from_str(CAPTURE).unwrap();
    let want = want.as_object().unwrap();
    assert_eq!(want.len(), 16);
    let wrong: Vec<String> = want
        .iter()
        .filter(|(k, v)| got[k.as_str()] != **v)
        .map(|(k, v)| format!("{k}:\n got {}\nwant {v}", got[k.as_str()]))
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
