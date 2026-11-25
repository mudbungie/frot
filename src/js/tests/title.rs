//! `document.title` and `document.referrer` (`bl-dc63`) against the real
//! binary: `title/probe.js` reads, writes and removes titles — two of them,
//! none, one in the body, one inside SVG, no head, an iframe's — and reads the
//! referrer of a typed navigation; `title/firefox153.json` is what Firefox
//! 153.0esr answered (identity.md §3.19 rig, over `title/page.html`). frot must
//! answer every key identically.

use serde_json::Value;

use super::sess;

const PROBE: &str = include_str!("title/probe.js");
const PAGE: &str = include_str!("title/page.html");
const CAPTURE: &str = include_str!("title/firefox153.json");

#[test]
fn every_captured_title_answers_as_firefox_153_does() {
    let s = sess(PAGE);
    let got = s
        .eval(&format!(
            "(function(){{var r;var resolve=function(o){{r=o}};{PROBE}\nreturn JSON.stringify(r)}})()"
        ))
        .unwrap();
    let got: Value = serde_json::from_str(&got).unwrap();
    let want: Value = serde_json::from_str(CAPTURE).unwrap();
    let want = want.as_object().unwrap();
    assert_eq!(want.len(), 14);
    let wrong: Vec<String> = want
        .iter()
        .filter(|(k, v)| &got[k.as_str()] != *v)
        .map(|(k, v)| format!("{k}: got {} want {v}", got[k.as_str()]))
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
