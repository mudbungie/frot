//! Focus in shadow trees against the real binary (js.md §15.11):
//! `delegates/probe.js` focuses and blurs hosts whose shadow roots delegate
//! focus (open, closed, with `autofocus`, with nothing focusable) and plain
//! ones, and reads `activeElement` on the document and each root, `:focus`
//! and `:focus-within`; `delegates/firefox153.json` is what Firefox 153.0esr
//! answered (identity.md §3.19 rig, over `delegates/page.html`). frot must
//! answer every key the same.

use serde_json::Value;

use super::sess;

const PROBE: &str = include_str!("delegates/probe.js");
const PAGE: &str = include_str!("delegates/page.html");
const CAPTURE: &str = include_str!("delegates/firefox153.json");

#[test]
fn every_delegates_focus_answer_is_firefox_153s() {
    let got = sess(PAGE)
        .eval(&format!(
            "(function(){{var r;var resolve=function(o){{r=o}};{PROBE}\nreturn JSON.stringify(r)}})()"
        ))
        .unwrap();
    let got: Value = serde_json::from_str(&got).unwrap();
    let want: Value = serde_json::from_str(CAPTURE).unwrap();
    let want = want.as_object().unwrap();
    assert_eq!(want.len(), 12);
    let wrong: Vec<String> = want
        .iter()
        .filter(|(k, v)| got[k.as_str()] != **v)
        .map(|(k, v)| format!("{k}:\n got {}\nwant {v}", got[k.as_str()]))
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
