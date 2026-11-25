//! NodeIterator (`bl-33df`): `nodeiter/probe.js` run over `nodeiter/page.html`
//! in frot, against Firefox 153.0esr's answers to the same probe
//! (`nodeiter/firefox153.json`, the identity.md §3.19 rig, 2026-11-24).

use serde_json::Value;

use super::drive;
use crate::dom::NodeKind;

const PAGE: &str = include_str!("nodeiter/page.html");
const PROBE: &str = include_str!("nodeiter/probe.js");
const CAPTURE: &str = include_str!("nodeiter/firefox153.json");

#[test]
fn every_captured_iteration_matches_firefox_153() {
    let page = PAGE.replace(
        "</body>",
        &format!(
            "<script>var resolve=function(o){{document.documentElement\
             .setAttribute('data-out',JSON.stringify(o))}};\n(function(){{{PROBE}\n}})()</script></body>"
        ),
    );
    let (doc, _) = drive(&page);
    let html = doc.find_by_tag("html")[0];
    let NodeKind::Element(el) = &doc.node(html).kind else {
        unreachable!("html is an element")
    };
    let got: Value = serde_json::from_str(el.attr("data-out").unwrap()).unwrap();
    let want: Value = serde_json::from_str(CAPTURE).unwrap();
    let wrong: Vec<String> = want
        .as_object()
        .unwrap()
        .iter()
        .filter(|(k, v)| &got[k.as_str()] != *v)
        .map(|(k, v)| format!("{k}: got {} want {v}", got[k.as_str()]))
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
