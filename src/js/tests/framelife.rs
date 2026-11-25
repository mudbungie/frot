//! A frame's life (`bl-2aaa`) against the real binary: `framelife/probe.js`
//! reads `window[i]` before and after inserting iframes, removes, re-inserts
//! and moves one, and inserts a frame's nodes into the parent;
//! `framelife/firefox153.json` is what Firefox 153.0esr answered (identity.md
//! §3.19 rig, over `framelife/page.html`). frot must answer every key
//! identically but the declared ones below.

use serde_json::Value;

use super::sess;

const PROBE: &str = include_str!("framelife/probe.js");
const PAGE: &str = include_str!("framelife/page.html");
const CAPTURE: &str = include_str!("framelife/firefox153.json");

/// Declared residuals, with what frot answers: `window[i]` is a getter (it
/// opens the window when read), so its descriptor is `gec` where the binary's
/// is a value, and a strict write throws the engine's wording; `defineProperty`
/// on an index succeeds (no WindowProxy to refuse it); a window's own key
/// order past the indices is frot's (`performance` before `close`); and an
/// adopted node takes the adopting window's prototype (js.md §14.4), so a
/// parent's node moved into a frame is no longer a parent `HTMLElement`, and a
/// frame's node moved into the parent answers the parent's interfaces
/// (`adopt`: its prototype and methods are this window's, not the frame's).
const DECLARED: [(&str, &str); 5] = [
    (
        "indexed",
        "2,true,true,undefined,gec,true,false,true,0/1/performance,0,1",
    ),
    ("setIdxStrict", "THROW TypeError: no setter for property"),
    ("defIdx", "false"),
    (
        "adopt",
        "true,true,true,true,true,false,true,true,true,0,xy,true",
    ),
    ("parentAdoptedIntoFrame", "true,false"),
];

#[test]
fn every_captured_frame_answer_matches_firefox_153() {
    let s = sess(PAGE);
    let got = s
        .eval(&format!(
            "(function(){{var r;var resolve=function(o){{r=o}};{PROBE}\nreturn JSON.stringify(r)}})()"
        ))
        .unwrap();
    let got: Value = serde_json::from_str(&got).unwrap();
    let want: Value = serde_json::from_str(CAPTURE).unwrap();
    let mut wrong = Vec::new();
    for (key, value) in want.as_object().unwrap() {
        match DECLARED.iter().find(|d| d.0 == key) {
            Some((_, frot)) => {
                let shown = got[key]
                    .as_str()
                    .map_or(got[key].to_string(), str::to_string);
                assert_eq!(shown, *frot, "{key}");
            }
            None if &got[key] != value => {
                wrong.push(format!("{key}: got {} want {value}", got[key]))
            }
            None => {}
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// A frame's node inserted into the parent is adopted: the same object, its
/// subtree and listeners with it, out of the frame's tree, now this document's
/// — answering through this window's prototype (js.md §14.4).
#[test]
fn a_frames_node_is_adopted_by_the_document_it_is_inserted_into() {
    let s = sess(PAGE);
    let got = s
        .eval(
            "(function(){var f=document.createElement('iframe');document.body.appendChild(f);\
             var FD=f.contentDocument,p=FD.createElement('p');p.id='ad';p.innerHTML='x<b>y</b>';\
             var b=p.lastChild,n=0;b.addEventListener('zz',function(){n++});FD.body.appendChild(p);\
             document.body.appendChild(p);b.dispatchEvent(new Event('zz',{bubbles:true}));\
             var q=document.adoptNode(FD.createElement('q')),i=document.importNode(FD.createElement('i'));\
             return [p.ownerDocument===document,document.getElementById('ad')===p,b.parentNode===p,\
             b.ownerDocument===document,FD.body.childNodes.length,p.textContent,n,\
             p instanceof HTMLParagraphElement,q.ownerDocument===document,q.parentNode,\
             i.ownerDocument===document].join()})()",
        )
        .unwrap();
    assert_eq!(got, "true,true,true,true,0,xy,1,true,true,,true");
}
