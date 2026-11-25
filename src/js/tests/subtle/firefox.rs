//! `crypto.subtle` against the real binary, call for call (`bl-db8f`):
//! `firefox153.txt` is some 440 expressions with what each settled to on
//! Firefox 153.0esr — results, key shapes, and every error's name, message and
//! precedence. frot must answer each one identically. The capture, not this
//! file, is the oracle; a difference here is a persona defect, never a reason
//! to edit the capture by hand.

use super::super::sess;

const CAPTURE: &str = include_str!("firefox153.txt");

/// What frot answers on a line the capture marks as a residual.
const NOT_SUPPORTED: &str = "NotSupportedError: Operation is not supported";

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

/// Every line of one capture, run in frot: the lines frot answers otherwise.
fn mismatches(capture: &str) -> (usize, Vec<String>) {
    let (mut checked, mut wrong) = (0, Vec::new());
    for block in capture.split("\n---\n").skip(1) {
        let (preamble, probes) = block.split_once("\n===\n").unwrap();
        let s = sess("<html><body></body></html>");
        s.eval(HELPERS).unwrap();
        s.eval(preamble).unwrap();
        for line in probes.lines() {
            // `!`: a declared residual — frot rejects what Firefox answers.
            let (residual, line) = line.strip_prefix('!').map_or((false, line), |l| (true, l));
            let (expr, firefox) = line.split_once(" => ").unwrap();
            let want = if residual { NOT_SUPPORTED } else { firefox };
            let quoted = serde_json::to_string(expr).unwrap();
            s.eval(&format!("probe({quoted});0")).unwrap();
            let got = s.eval("__out").unwrap();
            if got != want {
                wrong.push(format!("{expr}\n  frot:    {got}\n  firefox: {want}"));
            }
            checked += 1;
        }
    }
    assert_eq!(checked, capture.matches(" => ").count());
    (checked, wrong)
}

#[test]
fn every_captured_call_settles_as_it_does_on_firefox_153() {
    let (checked, wrong) = mismatches(CAPTURE);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert!(checked > 400, "{checked}");
}

/// The public-key half (`bl-0cae`): ECDSA, ECDH, Ed25519 and X25519.
#[test]
fn every_captured_public_key_call_settles_as_it_does_on_firefox_153() {
    let (checked, wrong) = mismatches(include_str!("firefox153pk.txt"));
    assert!(
        wrong.is_empty(),
        "{} of {checked}:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

/// The RSA half (`bl-2763`): RSASSA-PKCS1-v1_5, RSA-PSS and RSA-OAEP, residuals
/// marked.
#[test]
fn every_captured_rsa_call_settles_as_it_does_on_firefox_153() {
    let capture = include_str!("firefox153rsa.txt");
    let (checked, wrong) = mismatches(capture);
    assert!(
        wrong.is_empty(),
        "{} of {checked}:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
