//! `TextEncoder` / `TextDecoder` (`bl-53b7`): UTF-8 both ways, against the
//! values Firefox 153.0esr produced for the same inputs (identity.md §3.20).

use super::sess;

fn caught(s: &super::Session, expr: &str) -> String {
    s.eval(&format!(
        "(function(){{try{{return String({expr})}}catch(e){{return e.name+': '+e.message}}}})()"
    ))
    .unwrap()
}

fn page() -> super::Session {
    let s = sess("<html><body></body></html>");
    s.eval(
        "function U(a){return new Uint8Array(a)}\
         function cps(t){return Array.from(t,function(c){return c.codePointAt(0).toString(16)})\
         .join(' ')}",
    )
    .unwrap();
    s
}

#[test]
fn both_interfaces_are_firefox_shaped() {
    let s = page();
    assert_eq!(
        caught(&s, "Object.getOwnPropertyNames(TextEncoder.prototype)"),
        "encode,encodeInto,encoding,constructor"
    );
    assert_eq!(
        caught(&s, "Object.getOwnPropertyNames(TextDecoder.prototype)"),
        "decode,encoding,fatal,ignoreBOM,constructor"
    );
    assert_eq!(
        caught(
            &s,
            "[TextEncoder.length, TextDecoder.length, TextEncoder.prototype.encode.length, \
             TextEncoder.prototype.encodeInto.length, TextDecoder.prototype.decode.length, \
             String(new TextEncoder()), String(new TextDecoder()), \
             Object.getOwnPropertyNames(new TextEncoder()).length, \
             Object.getOwnPropertyNames(new TextDecoder()).length, \
             Object.getOwnPropertyDescriptor(TextDecoder.prototype,'fatal').get.name, \
             Object.getOwnPropertyDescriptor(TextDecoder.prototype,'fatal').set]"
        ),
        "0,0,0,2,0,[object TextEncoder],[object TextDecoder],0,0,get fatal,"
    );
}

#[test]
fn encode_is_utf8_with_lone_surrogates_replaced() {
    let s = page();
    let enc = |arg: &str| caught(&s, &format!("Array.from(new TextEncoder().encode({arg}))"));
    assert_eq!(
        enc("'a\\u00e9\\u20ac\\ud83d\\ude00'"),
        "97,195,169,226,130,172,240,159,152,128"
    );
    assert_eq!(enc(""), "");
    assert_eq!(enc("undefined"), "");
    assert_eq!(enc("12"), "49,50");
    assert_eq!(enc("null"), "110,117,108,108");
    // A lone high surrogate (before a letter, and last), and a lone low one.
    assert_eq!(enc("'\\ud800x'"), "239,191,189,120");
    assert_eq!(enc("'x\\ud800'"), "120,239,191,189");
    assert_eq!(enc("'\\udc00'"), "239,191,189");
    assert_eq!(
        caught(
            &s,
            "[new TextEncoder().encoding, new TextEncoder('utf-16').encoding, \
             Object.prototype.toString.call(new TextEncoder().encode('a'))]"
        ),
        "utf-8,utf-8,[object Uint8Array]"
    );
    // encodeInto stops before a scalar that does not fit, and reports UTF-16
    // units read and bytes written.
    assert_eq!(
        caught(
            &s,
            "(function(){var d=U(5),r=new TextEncoder().encodeInto('a\\u20ac\\ud83d\\ude00',d);\
             return JSON.stringify(r)+Object.getOwnPropertyNames(r)+' '+Array.from(d)})()"
        ),
        "{\"read\":2,\"written\":4}read,written 97,226,130,172,0"
    );
    assert_eq!(
        caught(
            &s,
            "JSON.stringify(new TextEncoder().encodeInto('\\ud83d\\ude00!',U(8)))"
        ),
        "{\"read\":3,\"written\":5}"
    );
    assert_eq!(
        caught(&s, "new TextEncoder().encodeInto('a', [])"),
        "TypeError: TextEncoder.encodeInto: Argument 2 does not implement interface Uint8Array."
    );
}

#[test]
fn decode_is_the_whatwg_utf8_decoder() {
    let s = page();
    let dec = |bytes: &str| caught(&s, &format!("cps(new TextDecoder().decode(U([{bytes}])))"));
    assert_eq!(
        dec("97,195,169,226,130,172,240,159,152,128"),
        "61 e9 20ac 1f600"
    );
    // Malformed input: one U+FFFD per maximal invalid prefix — a stray 0xFF, an
    // overlong lead, a lone continuation, an encoded surrogate, a truncated tail.
    assert_eq!(
        dec("97,255,98,192,128,237,160,128,240,159,152"),
        "61 fffd 62 fffd fffd fffd fffd fffd fffd"
    );
    assert_eq!(dec("226,130"), "fffd");
    // The range rules for the byte after E0, F0 and F4.
    assert_eq!(dec("224,128,128"), "fffd fffd fffd");
    assert_eq!(dec("240,128,128,128"), "fffd fffd fffd fffd");
    assert_eq!(dec("244,144,128,128"), "fffd fffd fffd fffd");
    assert_eq!(
        dec("224,160,128,240,144,128,128,244,143,191,191"),
        "800 10000 10ffff"
    );
    // A leading BOM is dropped unless ignoreBOM; only a leading one.
    assert_eq!(dec("239,187,191,97"), "61");
    assert_eq!(dec("97,239,187,191"), "61 feff");
    assert_eq!(
        caught(
            &s,
            "cps(new TextDecoder('utf-8',{ignoreBOM:true}).decode(U([239,187,191,97])))"
        ),
        "feff 61"
    );
    // No argument, an ArrayBuffer, a DataView, a view over part of a buffer.
    assert_eq!(
        caught(
            &s,
            "[new TextDecoder().decode(), new TextDecoder().decode(U([104,105]).buffer), \
             new TextDecoder().decode(new DataView(U([104,105]).buffer)), \
             new TextDecoder().decode(U([0,104,105,0]).subarray(1,3))]"
        ),
        ",hi,hi,hi"
    );
    // A text longer than one flush of the output buffer survives intact.
    assert_eq!(
        caught(&s, "new TextDecoder().decode(U(20000).fill(97)).length"),
        "20000"
    );
    assert_eq!(
        caught(&s, "new TextDecoder().decode('abc')"),
        "TypeError: TextDecoder.decode: Argument 1 could not be converted to any of: \
         ArrayBufferView, ArrayBuffer."
    );
}

#[test]
fn streaming_fatal_and_labels_behave_as_measured() {
    let s = page();
    // A sequence split across chunks; the BOM split across chunks, and dropped
    // again on the next, fresh, decode.
    assert_eq!(
        caught(
            &s,
            "(function(){var t=new TextDecoder();return JSON.stringify([\
             t.decode(U([226,130]),{stream:true}),cps(t.decode(U([172,240]),{stream:true})),\
             cps(t.decode(U([159,152,128])))])})()"
        ),
        r#"["","20ac","1f600"]"#
    );
    assert_eq!(
        caught(
            &s,
            "(function(){var t=new TextDecoder();return JSON.stringify([\
             t.decode(U([239,187]),{stream:true}),t.decode(U([191,97])),\
             t.decode(U([239,187,191,98]))])})()"
        ),
        r#"["","a","b"]"#
    );
    // Ending the stream with a sequence open is one replacement character…
    assert_eq!(
        caught(
            &s,
            "(function(){var t=new TextDecoder();return JSON.stringify([\
             t.decode(U([226,130]),{stream:true}),cps(t.decode())])})()"
        ),
        r#"["","fffd"]"#
    );
    // …or, when fatal, a TypeError — after which the decoder starts clean.
    let fatal = "TypeError: TextDecoder.decode: Decoding failed.";
    assert_eq!(
        caught(&s, "new TextDecoder('utf-8',{fatal:true}).decode(U([255]))"),
        fatal
    );
    assert_eq!(
        caught(
            &s,
            "(function(){var t=new TextDecoder('utf8',{fatal:1});t.decode(U([226,130]),\
             {stream:true});try{t.decode()}catch(e){return e.name+': '+e.message+'|'+\
             t.decode(U([104,105]))}})()"
        ),
        format!("{fatal}|hi")
    );
    assert_eq!(
        caught(
            &s,
            "(function(){var t=new TextDecoder('utf-8',{fatal:1,ignoreBOM:'x'});\
             return [t.encoding,t.fatal,t.ignoreBOM,new TextDecoder().fatal,\
             new TextDecoder().ignoreBOM]})()"
        ),
        "utf-8,true,true,false,false"
    );
    for label in [
        "utf8",
        "UTF-8",
        " utf-8 ",
        "unicode-1-1-utf-8",
        "unicode11utf8",
        "unicode20utf8",
        "x-unicode20utf8",
    ] {
        assert_eq!(
            caught(&s, &format!("new TextDecoder('{label}').encoding")),
            "utf-8",
            "{label}"
        );
    }
    // Unknown to Firefox too…
    assert_eq!(
        caught(&s, "new TextDecoder('nope')"),
        "RangeError: TextDecoder constructor: The given encoding 'nope' is not supported."
    );
    // …and the declared residual: encodings Firefox decodes and frot refuses.
    for label in ["latin1", "utf-16le", "windows-1252", "shift_jis"] {
        assert_eq!(
            caught(&s, &format!("new TextDecoder('{label}')")),
            format!(
                "RangeError: TextDecoder constructor: The given encoding '{label}' is not \
                 supported."
            ),
        );
    }
}
