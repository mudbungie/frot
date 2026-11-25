//! Receiver checks (`bl-a1a1`): `brandck/probe.js` run over `brandck/page.html`
//! in frot, against Firefox 153.0esr's answers to the same probe
//! (`brandck/firefox153.json`, the identity.md §3.19 rig, 2026-11-24).

use serde_json::Value;

use super::drive;
use crate::dom::NodeKind;

const PAGE: &str = include_str!("brandck/page.html");
const PROBE: &str = include_str!("brandck/probe.js");
const CAPTURE: &str = include_str!("brandck/firefox153.json");

/// Keys frot answers differently, each a declared residual, with what frot
/// answers: the arena keeps no namespace, so a created SVG `g` is an HTML
/// element (identity.md §11); and the interfaces outside the node tree do not
/// check their receiver yet (`bl-9df0`).
const DECLARED: [(&str, &str); 10] = [
    ("htmlOnSvg", "0"),
    ("urlHref", "undef"),
    ("styleProto", ""),
    ("navProto", "frot-test/1"),
    ("storage", "THROW TypeError:invalid 'in' operand"),
    ("evType", "undef"),
    ("histLen", "undef"),
    ("screenW", "1920"),
    ("perfNow", "0"),
    ("headersGet", "THROW TypeError:invalid 'in' operand"),
];

#[test]
fn every_captured_receiver_check_matches_firefox_153() {
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
