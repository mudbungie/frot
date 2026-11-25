//! Messaging (`bl-0af7`) against the real binary: `message/probe.js` posts to
//! its own window (every targetOrigin form), through MessageChannels (queued
//! before `start`, closed, transferred), across BroadcastChannels, and between
//! a window and its iframe, logging each delivery in order;
//! `message/firefox153.json` is what Firefox 153.0esr answered (identity.md
//! §3.19 rig, over `message/page.html`). frot must answer every key
//! identically, with the page's origin read as the test's, but two declared
//! residuals in the log: a frame window's constructor is `Object` (frot builds
//! no `Window` interface), and the `source` of a post made by frame code its
//! parent ran synchronously (`FW.eval`) is the parent — the entry window, not
//! the caller's realm (js.md §3, "Messaging").

use serde_json::Value;

use super::drive;
use crate::dom::NodeKind;

const PROBE: &str = include_str!("message/probe.js");
const PAGE: &str = include_str!("message/page.html");
const CAPTURE: &str = include_str!("message/firefox153.json");

#[test]
fn every_captured_message_answers_as_firefox_153_does() {
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
    let NodeKind::Element(el) = &doc.node(html).kind else {
        unreachable!("html is an element")
    };
    let got: Value = serde_json::from_str(el.attr("data-out").unwrap()).unwrap();
    let capture = CAPTURE
        .replace("http://127.0.0.1:8947", "https://example.com")
        .replace("|Window|MessageEvent", "|Object|MessageEvent")
        .replace(".com|frame|", ".com|win|");
    let want: Value = serde_json::from_str(&capture).unwrap();
    let want = want.as_object().unwrap();
    assert_eq!(want.len(), 15);
    let wrong: Vec<String> = want
        .iter()
        .filter(|(k, v)| &got[k.as_str()] != *v)
        .map(|(k, v)| format!("{k}: got {} want {v}", got[k.as_str()]))
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
