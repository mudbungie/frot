//! `PerformanceEntry` and `PerformanceResourceTiming` (`bl-c378`, js.md §5 "Resource timing")
//! against Firefox 153.0esr. `tests/fixtures/js/perf-probe.js` was served to
//! the binary as `/probe.js` (identity.md §3.19 rig) beside a script, a
//! stylesheet fetched by `fetch` and one by `XMLHttpRequest`, and its answers
//! captured verbatim in `perf-firefox153.json`. Here a local server serves the
//! same files to frot, and every answer must be the binary's.

use mockito::Server;
use serde_json::{Map, Value};

use super::drive_at;
use crate::dom::NodeKind;

const PROBE: &str = include_str!("../../../tests/fixtures/js/perf-probe.js");
const FIREFOX: &str = include_str!("../../../tests/fixtures/js/perf-firefox153.json");
const PAGE: &str = "<!doctype html><html><head><meta charset=utf-8><script>\
    window.done=function(v){document.documentElement.setAttribute('data-out',JSON.stringify(v))};\
    var s=document.createElement('script');s.src='/probe.js';document.head.appendChild(s);\
    </script></head><body>x</body></html>";

#[test]
fn resource_entries_answer_as_firefox_does() {
    let mut server = Server::new();
    let _m = [
        ("/probe.js", "text/javascript", PROBE),
        ("/ext.js", "text/javascript", "window.extLoaded=1;\n"),
        ("/s.css", "text/css", "body{}\n"),
        ("/x.css", "text/css", "body{}\n"),
    ]
    .map(|(path, ctype, body)| {
        server
            .mock("GET", path)
            .with_header("content-type", ctype)
            .with_body(body)
            .create()
    });
    let (doc, report) = drive_at(PAGE, &format!("{}/p.html", server.url()));
    assert_eq!(report.errors, 0);
    let html = doc.find_by_tag("html")[0];
    let NodeKind::Element(el) = &doc.node(html).kind else {
        unreachable!("html is an element")
    };
    let got: Map<String, Value> = serde_json::from_str(el.attr("data-out").unwrap()).unwrap();
    let want: Map<String, Value> = serde_json::from_str(FIREFOX).unwrap();
    assert_eq!(got, want);
}
