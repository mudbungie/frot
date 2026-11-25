//! `slotchange` against the real binary (js.md §15.8): `slotchange/probe.js`
//! changes slot assignment every way a tree can — inserting, removing and
//! renaming slottables and slots, fallback edits, nested and detached hosts —
//! and logs each `slotchange` a task later; `slotchange/firefox153.json` is
//! what Firefox 153.0esr answered (identity.md §3.19 rig, over
//! `slotchange/page.html`). frot must answer every key the same.

use serde_json::Value;

use super::drive;

const PROBE: &str = include_str!("slotchange/probe.js");
const PAGE: &str = include_str!("slotchange/page.html");
const CAPTURE: &str = include_str!("slotchange/firefox153.json");

#[test]
fn every_slotchange_answer_is_firefox_153s() {
    let page = PAGE.replace(
        "</body>",
        &format!(
            "<script>var resolve=function(o){{document.documentElement\
             .setAttribute('data-out',JSON.stringify(o))}};\n{PROBE}</script></body>"
        ),
    );
    let (doc, report) = drive(&page);
    assert_eq!(report.errors, 0);
    let html = doc.find_by_tag("html")[0];
    let out = doc.element(html).unwrap().attr("data-out").unwrap();
    let got: Value = serde_json::from_str(out).unwrap();
    let want: Value = serde_json::from_str(CAPTURE).unwrap();
    let want = want.as_object().unwrap();
    assert_eq!(want.len(), 17);
    let wrong: Vec<String> = want
        .iter()
        .filter(|(k, v)| got[k.as_str()] != **v)
        .map(|(k, v)| format!("{k}:\n got {}\nwant {v}", got[k.as_str()]))
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
