//! Streams (`bl-96f7`, js.md §16) against Firefox 153.0esr. Every call in
//! `tests/fixtures/js/streams-probe.js` was run on the binary (identity.md
//! §3.19 rig) and its answer captured verbatim in `streams-firefox153.json`;
//! this runs the same file in frot and requires the same answer, key by key.
//! The capture is the oracle — nothing in it was produced by frot.

use serde_json::{Map, Value};

use super::drive;
use crate::dom::NodeKind;

const PROBE: &str = include_str!("../../../tests/fixtures/js/streams-probe.js");
const FIREFOX: &str = include_str!("../../../tests/fixtures/js/streams-firefox153.json");

/// Where frot answers otherwise, each a declared residual (js.md §16), pinned
/// to frot's answer so a change in either direction is seen.
const RESIDUAL: [(&str, &str); 3] = [
    // encoding.js decodes UTF-8 only (identity.md §11).
    (
        "tdsEnc",
        "THROW RangeError: TextDecoderStream constructor: The given encoding 'latin1' is not \
         supported.",
    ),
    // Response has no formData, type or redirected.
    (
        "respProto",
        "clone,arrayBuffer,blob,bytes,json,text,url,status,ok,statusText,headers,body,bodyUsed,\
         constructor",
    ),
    // AbortSignal has no toStringTag of its own (abort.js).
    ("signal", "false,true,rr,[object EventTarget]"),
];

fn answers() -> Map<String, Value> {
    let html = format!(
        "<html><body><script>function done(v){{document.documentElement.setAttribute(\
         'data-out',JSON.stringify(v))}}\n{PROBE}</script></body></html>"
    );
    let (doc, report) = drive(&html);
    // One error, and it is the probe's: `sizeThrows` leaves a read of the
    // errored stream rejected and unawaited — an unhandled rejection, which
    // frot counts (js.md §10) as Firefox reports it to its console.
    assert_eq!(report.errors, 1);
    let html = doc.find_by_tag("html")[0];
    let NodeKind::Element(el) = &doc.node(html).kind else {
        unreachable!("html is an element")
    };
    let out = el.attr("data-out").expect("the probe finished");
    serde_json::from_str(out).unwrap()
}

#[test]
fn every_streams_call_answers_as_firefox_does() {
    let got = answers();
    let want: Map<String, Value> = serde_json::from_str(FIREFOX).unwrap();
    assert_eq!(got.len(), want.len(), "the probe ran every call");
    let mut wrong = Vec::new();
    for (key, ff) in &want {
        let frot = &got[key];
        let expect = RESIDUAL
            .iter()
            .find(|(k, _)| k == key)
            .map_or(ff.clone(), |(_, v)| Value::from(*v));
        if *frot != expect {
            wrong.push(format!("{key}: frot {frot} / want {expect}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
