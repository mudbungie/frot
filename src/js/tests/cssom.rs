//! Constructable stylesheets and `adoptedStyleSheets` (js.md §15) against
//! `cssom/firefox153.json` — what Firefox 153.0esr answered to
//! `cssom/probe.js` over `cssom/page.html` (identity.md §3.19 rig), in a
//! session that applies author CSS as `--css` does. frot must answer every key
//! the same but the declared few: Firefox re-serializes each rule's
//! `cssText` and frot keeps it as written; the CSSOM's legacy members and
//! `CSSStyleRule` are not built; and frot has no `document.styleSheets`
//! (js.md §15.3).

use serde_json::Value;

use crate::dom::Document;
use crate::fetch::FetchSession;
use crate::js::{Session, StyleSource};

const PROBE: &str = include_str!("cssom/probe.js");
const PAGE: &str = include_str!("cssom/page.html");
const CAPTURE: &str = include_str!("cssom/firefox153.json");

const DECLARED: [(&str, &str); 5] = [
    (
        "replaceSync",
        "undefined 3 p { color: red } | .a, .b { display: none } | @media screen { i { x: y } }",
    ),
    ("insert", "0 3 a{} | b{} | d{}"),
    (
        "setNonConstructed",
        "THROW TypeError: cannot read property '0' of undefined",
    ),
    (
        "proto",
        "insertRule deleteRule replace replaceSync ownerRule cssRules rules constructor || \
         type href ownerNode parentStyleSheet title media disabled constructor || true",
    ),
    (
        "ruleProto",
        "[object CSSRule] cssText constructor || [object CSSRuleList] item length constructor",
    ),
];

#[test]
fn every_constructable_sheet_answer_is_firefox_153s() {
    let s = Session::new(
        Document::parse(PAGE),
        StyleSource::Authored(Vec::new()),
        super::test_env(),
        &FetchSession::new(Vec::new()),
    );
    let got = s
        .eval(&format!(
            "(function(){{var r;var resolve=function(o){{r=o}};{PROBE}\nreturn JSON.stringify(r)}})()"
        ))
        .unwrap();
    let got: Value = serde_json::from_str(&got).unwrap();
    let want: Value = serde_json::from_str(CAPTURE).unwrap();
    let want = want.as_object().unwrap();
    assert_eq!(want.len(), 17);
    for (key, value) in want {
        let declared = DECLARED.iter().find(|d| d.0 == key).map(|d| d.1);
        assert_eq!(
            got[key],
            declared.map_or(value.clone(), Value::from),
            "{key}"
        );
    }
}
