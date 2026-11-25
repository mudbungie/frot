//! `setHTMLUnsafe` against the real binary (js.md §15.10): `sethtml/probe.js`
//! parses markup with declarative shadow roots into elements, a shadow root
//! and a template, beside `innerHTML`'s refusal of them;
//! `sethtml/firefox153.json` is what Firefox 153.0esr answered (identity.md
//! §3.19 rig, over `sethtml/page.html`). frot must answer every key the same
//! but three, declared in js.md §15.10: `Document.parseHTMLUnsafe` is absent
//! (`shape`, `parse`), and frot upgrades a defined element `setHTMLUnsafe`
//! makes at once, as `innerHTML` does (`custom`). The capture is the oracle.

use serde_json::Value;

use super::sess;

const PROBE: &str = include_str!("sethtml/probe.js");
const PAGE: &str = include_str!("sethtml/page.html");
const CAPTURE: &str = include_str!("sethtml/firefox153.json");

const DECLARED: [(&str, &str); 3] = [
    ("shape", "function 1 function undefined true"),
    ("custom", "ctor:1:true,conn true"),
    ("parse", "THROW TypeError: not a function"),
];

#[test]
fn every_set_html_answer_is_firefox_153s() {
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
        .filter_map(|(k, v)| {
            let declared = DECLARED.iter().find(|d| d.0 == k).map(|d| Value::from(d.1));
            let v = declared.unwrap_or_else(|| v.clone());
            (got[k.as_str()] != v).then(|| format!("{k}:\n got {}\nwant {v}", got[k.as_str()]))
        })
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
