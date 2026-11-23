//! `TreeWalker` and `NodeFilter` against the real binary (`bl-b8c0`):
//! `treewalker/probe.js` walks one tree under seven filters and seven
//! `whatToShow` masks in every direction, then pokes the argument, filter-call
//! and receiver edges; `treewalker/firefox153.json` is what Firefox 153.0esr
//! answered (identity.md §3.19 rig, over `treewalker/page.html`). frot must
//! answer every key identically but the declared few below. The capture, not
//! this file, is the oracle.

use serde_json::Value;

use super::sess;

const PROBE: &str = include_str!("treewalker/probe.js");
const PAGE: &str = include_str!("treewalker/page.html");
const CAPTURE: &str = include_str!("treewalker/firefox153.json");

/// Keys frot answers differently, each a declared residual: the engine's own
/// wording for a Symbol (identity.md §11, "Engine error messages"); no
/// `NodeIterator` (bl-b8c0 left it out); no `template.content` (bl-aff4).
const DECLARED: [&str; 5] = ["retSym", "newNI", "ctwlen", "ni", "litlike"];

#[test]
fn every_captured_walk_answers_as_firefox_153_does() {
    let s = sess(PAGE);
    let got = s
        .eval(&format!(
            "(function(){{var r;var resolve=function(o){{r=o}};{PROBE}\nreturn JSON.stringify(r)}})()"
        ))
        .unwrap();
    let got: Value = serde_json::from_str(&got).unwrap();
    let want: Value = serde_json::from_str(CAPTURE).unwrap();
    let want = want.as_object().unwrap();
    assert_eq!(want.len(), 283);
    for (key, value) in want {
        if !DECLARED.contains(&key.as_str()) {
            assert_eq!(&got[key], value, "{key}");
        }
    }
    assert_eq!(
        got["retSym"],
        "THROW TypeError: cannot convert symbol to number"
    );
}
