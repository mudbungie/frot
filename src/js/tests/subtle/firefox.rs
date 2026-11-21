//! `crypto.subtle` against the real binary, call for call (`bl-db8f`):
//! `firefox153.txt` is some 440 expressions with what each settled to on
//! Firefox 153.0esr — results, key shapes, and every error's name, message and
//! precedence. frot must answer each one identically. The capture, not this
//! file, is the oracle; a difference here is a persona defect, never a reason
//! to edit the capture by hand.

use super::super::sess;

const CAPTURE: &str = include_str!("firefox153.txt");

/// The capture's shared helpers: `show` renders a settled value the way the
/// rig did — a buffer as its length and hex, a key as its four attributes.
const HELPERS: &str = "\
function hex(b){return Array.from(new Uint8Array(b),function(x){return (x+256).toString(16)\
.slice(1)}).join('')}\
function U(n,f){var u=new Uint8Array(n);if(f!==undefined)u.fill(f);return u}\
function unhex(s){return Uint8Array.from(s.match(/../g)||[],function(h){return parseInt(h,16)})}\
var S=crypto.subtle;\
function show(v){\
if(v instanceof ArrayBuffer)return 'buf['+v.byteLength+']:'+hex(v);\
if(v instanceof CryptoKey)return 'key:'+v.type+','+v.extractable+','+JSON.stringify(v.algorithm)\
+','+v.usages.join('|');\
if(v&&typeof v==='object')return 'obj:'+JSON.stringify(v);\
return typeof v+':'+String(v)}\
var __out;\
function probe(src){__out='pending';Promise.resolve().then(function(){return (0,eval)(src)})\
.then(function(v){__out=show(v)},function(e){__out=e.name+': '+e.message})}";

#[test]
fn every_captured_call_settles_as_it_does_on_firefox_153() {
    let mut checked = 0;
    for block in CAPTURE.split("\n---\n").skip(1) {
        let (preamble, probes) = block.split_once("\n===\n").unwrap();
        let s = sess("<html><body></body></html>");
        s.eval(HELPERS).unwrap();
        s.eval(preamble).unwrap();
        for line in probes.lines() {
            let (expr, want) = line.split_once(" => ").unwrap();
            let quoted = serde_json::to_string(expr).unwrap();
            s.eval(&format!("probe({quoted});0")).unwrap();
            assert_eq!(s.eval("__out").unwrap(), want, "{expr}");
            checked += 1;
        }
    }
    assert_eq!(checked, CAPTURE.matches(" => ").count());
    assert!(checked > 400, "{checked}");
}
