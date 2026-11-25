//! `Node.isEqualNode` (`bl-d5dc`): `equal/probe.js` run over `equal/page.html`
//! in frot, against Firefox 153.0esr's answers to the same probe
//! (`equal/firefox153.json`, the identity.md §3.19 rig, 2026-11-24).

use serde_json::Value;

use super::drive;
use crate::dom::NodeKind;

const PAGE: &str = include_str!("equal/page.html");
const PROBE: &str = include_str!("equal/probe.js");
const CAPTURE: &str = include_str!("equal/firefox153.json");

/// Keys frot answers differently, each a declared residual, with what frot
/// answers: `document.implementation.createDocumentType` and
/// `document.createAttribute` are not built (identity.md §3.21; `doctypeClone`
/// and `attrHandle` cover the same comparisons), and the arena keeps no
/// namespace, so an SVG `g` equals an HTML one (identity.md §11).
const DECLARED: [(&str, &str); 3] = [
    ("doctype", "THROW TypeError:not a function"),
    ("attrNode", "THROW TypeError:not a function"),
    ("svg", "true"),
];

#[test]
fn every_captured_equality_matches_firefox_153() {
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
        .filter(|(k, v)| {
            let frot = DECLARED
                .iter()
                .find(|d| d.0 == k.as_str())
                .map(|d| Value::from(d.1));
            got[k.as_str()] != frot.unwrap_or_else(|| (*v).clone())
        })
        .map(|(k, v)| format!("{k}: got {} want {v}", got[k.as_str()]))
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
