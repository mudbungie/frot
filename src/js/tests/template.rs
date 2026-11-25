//! `<template>` contents, `importNode` and `adoptNode` against the real binary
//! (js.md §15): `template/probe.js` reads a parsed, a nested and a created
//! template, their clones and imports, and the refusals;
//! `template/firefox153.json` is what Firefox 153.0esr answered (identity.md
//! §3.19 rig, over `template/page.html`). frot must answer every key the same.
//! The capture, not this file, is the oracle.

use serde_json::Value;

use super::sess;

const PROBE: &str = include_str!("template/probe.js");
const PAGE: &str = include_str!("template/page.html");
const CAPTURE: &str = include_str!("template/firefox153.json");

#[test]
fn every_template_answer_is_firefox_153s() {
    let got = sess(PAGE)
        .eval(&format!(
            "(function(){{var r;var resolve=function(o){{r=o}};{PROBE}\nreturn JSON.stringify(r)}})()"
        ))
        .unwrap();
    let got: Value = serde_json::from_str(&got).unwrap();
    let want: Value = serde_json::from_str(CAPTURE).unwrap();
    let want = want.as_object().unwrap();
    assert_eq!(want.len(), 34);
    for (key, value) in want {
        assert_eq!(&got[key], value, "{key}");
    }
}

/// `--out dom` writes what script left in a template's contents, and — as
/// HTML serialization does — not a child appended to the template itself.
#[test]
fn dom_output_writes_a_templates_contents_after_script() {
    let s = sess("<template id=t><i>parsed</i></template>");
    s.eval(
        "var t=document.getElementById('t');t.content.appendChild(document.createElement('u'));\
         t.appendChild(document.createElement('s'))",
    )
    .unwrap();
    let out = crate::views::dom::dom_json(&s.into_document());
    let names: Vec<&str> = out[0]["children"][0]["children"][0]["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["i", "u"]);
}
