//! Audio contexts and nodes as EventTargets (`bl-bd3d`): `audioctx/probe.js`
//! run over `audioctx/page.html` in frot, against Firefox 153.0esr's settled
//! answers to the same probe (`audioctx/firefox153.json`, the identity.md
//! §3.19 rig, 2026-11-24). The `*Own` keys are prototype member lists, of
//! which frot builds a subset: each of frot's members must appear in
//! Firefox's list, in its order. Every other key is answered identically.

use serde_json::Value;

use super::drive;
use crate::dom::NodeKind;

const PAGE: &str = include_str!("audioctx/page.html");
const PROBE: &str = include_str!("audioctx/probe.js");
const CAPTURE: &str = include_str!("audioctx/firefox153.json");

/// `resolve` settles every value (some are promises) before writing them.
const RESOLVE: &str = "var resolve=function(o){var ks=Object.keys(o),n=ks.length,out={};\
    ks.forEach(function(k){Promise.resolve(o[k]).then(function(v){out[k]=String(v);\
    if(--n===0)document.documentElement.setAttribute('data-out',JSON.stringify(out))})})};";

fn subsequence(got: &str, want: &str) -> bool {
    let mut rest = want.split(' ');
    got.split(' ').all(|m| rest.any(|w| w == m))
}

#[test]
fn audio_contexts_answer_as_firefox_153_does() {
    let script = format!("<script>(function(){{{RESOLVE}\n{PROBE}\n}})()</script></body>");
    let (doc, _) = drive(&PAGE.replace("</body>", &script));
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
            let (g, w) = (got[k.as_str()].as_str().unwrap_or(""), v.as_str().unwrap());
            if k.ends_with("Own") {
                !subsequence(g, w)
            } else {
                g != w
            }
        })
        .map(|(k, v)| format!("{k}: got {} want {v}", got[k.as_str()]))
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
