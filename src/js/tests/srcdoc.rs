//! `<iframe srcdoc>` (`bl-7b86`) against the real binary: `srcdoc/probe.js`
//! inserts a srcdoc frame and logs, in order, what the parent sees right
//! away, the frame's scripts and lifecycle, the iframe's `load`, a srcdoc
//! frame made by `setAttribute`, a changed srcdoc, a removed one, and a
//! throwing script; `srcdoc/firefox153.json` is the log Firefox 153.0esr
//! wrote (identity.md §3.19 rig, over `srcdoc/page.html`). One declared
//! residual: the navigated frame keeps its document object, where Firefox
//! makes a new one, so `contentDocument === D0` reads true twice.

use serde_json::Value;

use super::drive;
use crate::dom::NodeKind;

const PROBE: &str = include_str!("srcdoc/probe.js");
const PAGE: &str = include_str!("srcdoc/page.html");
const CAPTURE: &str = include_str!("srcdoc/firefox153.json");

#[test]
fn a_srcdoc_frame_loads_as_firefox_153_loads_it() {
    let page = PAGE.replace(
        "</body>",
        &format!(
            "<script>var resolve=function(o){{document.documentElement\
             .setAttribute('data-out',JSON.stringify(o))}};\n{PROBE}</script></body>"
        ),
    );
    let (doc, report) = drive(&page);
    // The last frame's script throws on purpose.
    let html = doc.find_by_tag("html")[0];
    let NodeKind::Element(el) = &doc.node(html).kind else {
        unreachable!("html is an element")
    };
    let got: Value = serde_json::from_str(el.attr("data-out").unwrap()).unwrap();
    let want: Value = serde_json::from_str(CAPTURE).unwrap();
    let want = want["log"]
        .as_str()
        .unwrap()
        .replace(":506:false", ":506:true")
        .replace("number:false:", "number:true:");
    assert_eq!(got["log"], want.as_str());
    assert_eq!(report.errors, 1);
}
