//! Event handlers (`bl-93c8`) against the real binary: `handlers/probe.js`
//! reads the `on*` accessors' shapes, sets handlers every way a page can (IDL
//! property, content attribute, parsed attribute, `innerHTML`, a body's
//! forwarded ones, `window.onerror`), and dispatches through them;
//! `handlers/firefox153.json` is what Firefox 153.0esr answered (identity.md
//! §3.19 rig, over `handlers/page.html`, the probe run after `load`). frot must
//! answer every key identically but the declared ones below.

use serde_json::Value;

use super::drive;
use crate::dom::NodeKind;

const PROBE: &str = include_str!("handlers/probe.js");
const PAGE: &str = include_str!("handlers/page.html");
const CAPTURE: &str = include_str!("handlers/firefox153.json");

/// Keys frot answers differently, each a declared residual, with what frot
/// answers: a syntax error's message is the engine's wording (identity.md §11,
/// "Engine error messages"); a reported exception's `lineno`/`colno`/`filename`
/// are 0/0/`''` (js.md §3, "One dispatch path"); quickjs lists a function's own
/// `length` first; `scope` reaches the named access frot does not build (a
/// form's named controls, document and window named properties, `bl-de60` —
/// `bl-09c8` built the rest); and the two images'
/// tasks end in document order, where Firefox's network ended the `data:` one
/// first.
const DECLARED: [(&str, &str); 9] = [
    ("syntaxErr", "null;ErrorEvent:ReferenceError: log is not defined:ReferenceError:0 ErrorEvent:SyntaxError: unexpected token in expression: '}':SyntaxError:0;"),
    ("syntaxErrDispatch", "1;ErrorEvent:SyntaxError: unexpected token in expression: '}':SyntaxError:0;2;"),
    ("throwing", "ErrorEvent:RangeError: rr:RangeError:0"),
    ("throwingNonError", "ErrorEvent:uncaught exception: ss:undefined:0"),
    ("scope", "undefined,object,string,string,q,function,object,undefined,true,undefined"),
    ("winOnerror", "on:string/Error: w1/string/0/0/w1/true/5 L:true on:object/[object Event]/undefined////true/1 L:false plain:true1 L:true true ee:mm5 L:true true"),
    ("winOnerrorAttr", "function;5;\"function onerror(event, source, lineno, colno, error) {\\nwindow.OE = [event, source, lineno, colno, error && error.message, typeof arguments[0]].join(\\\"/\\\"); return true\\n}\";Error: w2//0/0/w2/string;ErrorEvent:Error: w2:Error:0"),
    ("handlerName", "onload,length/name/prototype"),
    ("imgAttr", "err:error:i1:true load:i2"),
];

fn run() -> Value {
    let page = PAGE.replace(
        "</body>",
        &format!(
            "<script>addEventListener('load',function(){{setTimeout(function(){{\
             var resolve=function(o){{document.documentElement.setAttribute('data-out',JSON.stringify(o))}};\
             {PROBE}\n}},100)}})</script></body>"
        ),
    );
    let (doc, _) = drive(&page);
    let html = doc.find_by_tag("html")[0];
    let NodeKind::Element(el) = &doc.node(html).kind else {
        unreachable!("html is an element")
    };
    serde_json::from_str(el.attr("data-out").unwrap()).unwrap()
}

#[test]
fn every_captured_handler_answers_as_firefox_153_does() {
    let got = run();
    let want: Value = serde_json::from_str(CAPTURE).unwrap();
    let want = want.as_object().unwrap();
    let mut wrong = Vec::new();
    for (key, value) in want {
        let declared = DECLARED.iter().find(|d| d.0 == key);
        if let Some((_, frot)) = declared {
            assert_eq!(got[key], *frot, "{key}");
        } else if &got[key] != value {
            wrong.push(format!("{key}: got {} want {value}", got[key]));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The parsed `<body onload>` is the window's `load` handler, `this` the
/// window, and the classic broken-image swap runs: an `<img onerror>` hides
/// itself when its request fails.
#[test]
fn a_parsed_body_onload_and_img_onerror_run() {
    let (doc, report) = drive(
        "<body onload=\"document.body.setAttribute('data-out', [event.type, this === window, \
         document.getElementById('i').style.display].join())\">\
         <img id=i src=/a.png onerror=\"this.style.display='none'\"></body>",
    );
    assert_eq!(report.errors, 0);
    let body = doc.find_by_tag("body")[0];
    let NodeKind::Element(el) = &doc.node(body).kind else {
        unreachable!("body is an element")
    };
    assert_eq!(el.attr("data-out"), Some("load,true,none"));
}
