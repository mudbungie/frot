//! `Blob` and blob: URLs (`bl-78c5`), against what Firefox 153.0esr does with
//! the same calls (identity.md §3.20).

use super::{sess, Session};

fn page() -> Session {
    let s = sess("<html><body></body></html>");
    s.eval(
        "function U(a){return new Uint8Array(a)}\
         function hex(b){return Array.from(new Uint8Array(b),function(x){return (x+256)\
         .toString(16).slice(1)}).join('')}",
    )
    .unwrap();
    s
}

fn caught(s: &Session, expr: &str) -> String {
    s.eval(&format!(
        "(function(){{try{{return String({expr})}}catch(e){{return e.name+': '+e.message}}}})()"
    ))
    .unwrap()
}

/// What the promise `expr` settles to: its value, or `Name: message`.
fn settle(s: &Session, expr: &str) -> String {
    s.eval(&format!(
        "var __out='pending';Promise.resolve().then(function(){{return {expr}}}).then(\
         function(v){{__out=String(v)}},function(e){{__out=e.name+': '+e.message}});0"
    ))
    .unwrap();
    s.eval("__out").unwrap()
}

#[test]
fn blob_is_firefox_shaped() {
    let s = page();
    // `stream` is the one member of the binary's prototype frot does not have.
    assert_eq!(
        caught(&s, "Object.getOwnPropertyNames(Blob.prototype)"),
        "slice,text,arrayBuffer,bytes,size,type,constructor"
    );
    assert_eq!(
        caught(
            &s,
            "[Blob.length, Blob.prototype.slice.length, Blob.prototype.text.length, \
             String(new Blob()), Object.getOwnPropertyNames(new Blob()).length, new Blob().size, \
             JSON.stringify(new Blob().type), new Blob(undefined).size, new Blob([], null).size, \
             Object.getOwnPropertyDescriptor(Blob.prototype,'size').get.name, \
             Object.getOwnPropertyDescriptor(Blob.prototype,'size').set]"
        ),
        "0,0,0,[object Blob],0,0,\"\",0,0,get size,"
    );
    for (call, want) in [
        (
            "new Blob('abc')",
            "Blob constructor: Argument 1 can't be converted to a sequence.",
        ),
        (
            "new Blob({})",
            "Blob constructor: Argument 1 can't be converted to a sequence.",
        ),
        (
            "new Blob(null)",
            "Blob constructor: Argument 1 can't be converted to a sequence.",
        ),
        (
            "new Blob([], 5)",
            "Blob constructor: Argument 2 can't be converted to a dictionary.",
        ),
        (
            "new Blob([], {endings:'x'})",
            "Blob constructor: 'x' (value of 'endings' member of BlobPropertyBag) is not a \
             valid value for enumeration EndingType.",
        ),
        (
            "Object.getOwnPropertyDescriptor(Blob.prototype,'size').get.call({})",
            "'get size' called on an object that does not implement interface Blob.",
        ),
        (
            "Object.getOwnPropertyDescriptor(Blob.prototype,'type').get.call({})",
            "'get type' called on an object that does not implement interface Blob.",
        ),
        (
            "Blob.prototype.slice.call({})",
            "'slice' called on an object that does not implement interface Blob.",
        ),
    ] {
        assert_eq!(caught(&s, call), format!("TypeError: {want}"), "{call}");
    }
    // A reader on the wrong receiver rejects; it does not throw.
    assert_eq!(
        settle(&s, "Blob.prototype.text.call({})"),
        "TypeError: 'text' called on an object that does not implement interface Blob."
    );
}

#[test]
fn parts_and_types_convert_as_measured() {
    let s = page();
    s.eval(
        "var b=new Blob(['ab',U([99]),U([100,101]).buffer,new Blob(['f']),5,\
         {toString:function(){return 'T'}}],{type:'Text/Plain;X=1'});",
    )
    .unwrap();
    assert_eq!(caught(&s, "[b.size, b.type]"), "8,text/plain;x=1");
    assert_eq!(settle(&s, "b.text()"), "abcdef5T");
    assert_eq!(
        settle(
            &s,
            "Promise.all([b.arrayBuffer(),b.bytes()]).then(function(r){return [String(r[0]),\
             r[0].byteLength,Object.prototype.toString.call(r[1]),hex(r[1])]})"
        ),
        "[object ArrayBuffer],8,[object Uint8Array],6162636465663554"
    );
    // Any iterable of parts; a wider view contributes its bytes; a lone
    // surrogate is U+FFFD; a view over part of a buffer contributes that part.
    assert_eq!(
        caught(
            &s,
            "[new Blob(new Set(['a','b'])).size, new Blob([new Uint16Array([1,2])]).size, \
             new Blob(['\\ud800']).size, new Blob([U([1,2,3,4]).subarray(1,3)]).size]"
        ),
        "2,4,3,2"
    );
    assert_eq!(
        caught(
            &s,
            "[new Blob([],{type:'A B'}).type, new Blob([],{type:'a\\u007f'}).type, \
             new Blob([],{type:'a\\u001f'}).type, new Blob([],{type:'a\\u00e9'}).type, \
             new Blob([],{type:5}).type, new Blob([],{type:null}).type, \
             new Blob([],{type:undefined}).type].join('|')"
        ),
        "a b||||5|null|"
    );
    // `endings` touches string parts only: native (Linux) makes every break LF.
    assert_eq!(
        settle(
            &s,
            "Promise.all([new Blob(['a\\r\\nb\\rc\\nd'],{endings:'transparent'}).text(),\
             new Blob(['a\\r\\nb\\rc\\nd'],{endings:'native'}).text()]).then(JSON.stringify)"
        ),
        r#"["a\r\nb\rc\nd","a\nb\nc\nd"]"#
    );
    // text() is a UTF-8 decode: malformed bytes replaced, a leading BOM dropped.
    assert_eq!(
        settle(
            &s,
            "Promise.all([new Blob([U([255,97])]).text(),new Blob([U([239,187,191,97])]).text()])\
             .then(function(r){return r[0].charCodeAt(0).toString(16)+r[0][1]+r[1].length})"
        ),
        "fffda1"
    );
    // The bytes are the blob's own: a part mutated afterwards changes nothing.
    assert_eq!(
        settle(
            &s,
            "(function(){var u=U([104,105]),x=new Blob([u]);u[0]=0;return x.text()})()"
        ),
        "hi"
    );
}

#[test]
fn slice_clamps_like_array_indices_and_takes_its_own_type() {
    let s = page();
    s.eval("var b=new Blob(['abcdef'],{type:'t/x'});").unwrap();
    assert_eq!(
        caught(
            &s,
            "[b.slice(2).size, b.slice(-100,100).size, b.slice(4,2).size, b.slice(1.9,3.9).size, \
             b.slice('1','3').size, b.slice(NaN).size, b.slice().size, \
             JSON.stringify(b.slice(0,1).type), b.slice(undefined,undefined,'A/B').type, \
             JSON.stringify(b.slice(0,1,'a\\u00e9').type), String(b.slice()), b.slice()!==b]"
        ),
        "4,6,0,2,2,6,6,\"\",a/b,\"\",[object Blob],true"
    );
    assert_eq!(
        settle(
            &s,
            "Promise.all([b.slice(1,3).text(),b.slice(-2).text()]).then(String)"
        ),
        "bc,ef"
    );
}

#[test]
fn blob_urls_are_minted_and_nothing_can_load_one() {
    let s = page();
    assert_eq!(
        caught(
            &s,
            "[URL.createObjectURL.length, URL.createObjectURL.name, URL.revokeObjectURL.length, \
             /^blob:https:\\/\\/example\\.com\\/[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-\
             [0-9a-f]{12}$/.test(URL.createObjectURL(new Blob(['x']))), \
             URL.createObjectURL(new Blob())!==URL.createObjectURL(new Blob()), \
             String(URL.revokeObjectURL('blob:nope')), String(URL.revokeObjectURL(5))]"
        ),
        "1,createObjectURL,1,true,true,undefined,undefined"
    );
    for (call, want) in [
        (
            "URL.createObjectURL({})",
            "URL.createObjectURL: Argument 1 could not be converted to any of: Blob, MediaSource.",
        ),
        (
            "URL.createObjectURL()",
            "URL.createObjectURL: At least 1 argument required, but only 0 passed",
        ),
        (
            "URL.revokeObjectURL()",
            "URL.revokeObjectURL: At least 1 argument required, but only 0 passed",
        ),
    ] {
        assert_eq!(caught(&s, call), format!("TypeError: {want}"), "{call}");
    }
    // The declared residual: a blob: URL does not load. The request fails; it is
    // never answered with made-up bytes.
    assert!(
        settle(&s, "fetch(URL.createObjectURL(new Blob(['x'])))").starts_with("TypeError: "),
        "a blob: fetch must reject"
    );
    // A file: page's origin is opaque, so its blob URLs say `null` (measured).
    let file = Session::new(
        crate::dom::Document::parse("<html><body></body></html>"),
        super::StyleSource::Bare,
        super::Env {
            url: "file:///tmp/page.html".into(),
            ..super::test_env()
        },
        &crate::fetch::FetchSession::new(Vec::new()),
    );
    assert_eq!(
        file.eval("URL.createObjectURL(new Blob()).slice(0,10)")
            .unwrap(),
        "blob:null/"
    );
}
