//! What a `FileReader` read does over time (`bl-8c46`): the event sequence, the
//! four result forms, handler order and `abort()` — each as measured on
//! Firefox 153.0esr (identity.md §3.20).

use super::out;

/// The measured event sequence: nothing synchronously, then `loadstart`,
/// `progress`, `load`, `loadend` in one task — after the caller's microtasks and
/// before a zero timeout it queued afterwards. An empty blob has no `progress`.
#[test]
fn a_read_fires_its_four_events_in_one_later_task() {
    assert_eq!(
        out(
            "var r=new FileReader();watch(r);r.readAsDataURL(new Blob(['hi']));\
             o.push('sync('+r.readyState+','+r.result+')');\
             Promise.resolve().then(function(){o.push('micro('+r.readyState+')')});\
             setTimeout(function(){o.push('timeout('+r.readyState+')',r.result,r.error);done()},0)"
        ),
        "sync(1,null) micro(1) loadstart(1,0/2) progress(1,2/2) load(2,2/2) loadend(2,2/2) \
         timeout(2) data:application/octet-stream;base64,aGk= "
    );
    assert_eq!(
        out(
            "var r=new FileReader();watch(r);r.readAsArrayBuffer(new Blob([]));\
             r.onloadend=function(){o.push(r.result.byteLength);done()}"
        ),
        "loadstart(1,0/0) load(2,0/0) loadend(2,0/0) 0"
    );
}

#[test]
fn each_reader_gives_the_blob_in_its_own_form() {
    // "hi é" in UTF-8, typed text/plain.
    let blob = "new Blob([new Uint8Array([0x68,0x69,0x20,0xc3,0xa9])],{type:'Text/Plain'})";
    let read = |method: &str, render: &str| {
        out(&format!(
            "var r=new FileReader();r.onload=function(e){{o.push({render});done()}};\
             r.{method}({blob})"
        ))
    };
    assert_eq!(
        read("readAsDataURL", "r.result"),
        "data:text/plain;base64,aGkgw6k="
    );
    assert_eq!(read("readAsText", "r.result"), "hi é");
    assert_eq!(
        read("readAsBinaryString", "escape(r.result)"),
        "hi%20%C3%A9"
    );
    assert_eq!(
        read(
            "readAsArrayBuffer",
            "hex(r.result),r.result===r.result,r.result instanceof ArrayBuffer"
        ),
        "686920c3a9 true true"
    );
    // The event: a trusted ProgressEvent aimed at the reader, handler `this` too.
    assert_eq!(
        read(
            "readAsText",
            "String(e),e.type,e.lengthComputable,e.isTrusted,e.bubbles,e.target===r,this===r"
        ),
        "[object ProgressEvent] load true true false true true"
    );
    // A typeless blob is application/octet-stream; text drops a UTF-8 BOM and
    // replaces what is malformed; bytes past one chunk survive the base64.
    assert_eq!(
        out(
            "var r=new FileReader();r.onload=function(){o.push(r.result);done()};\
             r.readAsDataURL(new Blob([new Uint8Array([0,255,128,65,1])],{type:'a/b'}))"
        ),
        "data:a/b;base64,AP+AQQE="
    );
    assert_eq!(
        out(
            "var r=new FileReader();r.onload=function(){o.push(escape(r.result));done()};\
             r.readAsText(new Blob([new Uint8Array([0xef,0xbb,0xbf,0x68,0xff])]))"
        ),
        "h%uFFFD"
    );
    assert_eq!(
        out(
            "var r=new FileReader();r.onload=function(){var s=atob(r.result.split(',')[1]);\
             o.push(s.length,s.charCodeAt(8191),s.charCodeAt(8192),s.charCodeAt(19999));done()};\
             r.readAsDataURL(new Blob([new Uint8Array(20000).map(function(_,i){return i%251})]))"
        ),
        "20000 159 160 170"
    );
}

/// A handler attribute is a listener in the place it was first set; setting it
/// to null gives the place up. A throwing handler does not stop `loadend`.
#[test]
fn handlers_fire_in_registration_order() {
    let order = |setup: &str| {
        out(&format!(
            "var r=new FileReader();function L(n){{return function(){{o.push(n)}}}}{setup}\
             r.onloadend=done;r.readAsText(new Blob(['a']))"
        ))
    };
    assert_eq!(
        order("r.addEventListener('load',L('l1'));r.onload=L('h');r.addEventListener('load',L('l2'));"),
        "l1 h l2"
    );
    assert_eq!(
        order("r.onload=L('h1');r.addEventListener('load',L('l'));r.onload=L('h2');"),
        "h2 l"
    );
    assert_eq!(
        order("r.onload=L('h1');r.onload=null;r.addEventListener('load',L('l'));r.onload=L('h2');"),
        "l h2"
    );
    assert_eq!(
        order("r.onload=function(){o.push('threw');throw new Error('boom')};"),
        "threw"
    );
}

/// `abort()` on a read in flight: `abort` then `loadend`, synchronously, with
/// the blob's size as `total`; the read's own events never come. The reader can
/// be used again, and a new read clears the error.
#[test]
fn abort_ends_a_read_in_flight() {
    assert_eq!(
        out(
            "var r=new FileReader();watch(r);r.readAsText(new Blob(['abc']));r.abort();\
             o.push('after('+r.readyState+','+r.result+','+r.error.name+':'+r.error.message+')');\
             r.abort();setTimeout(function(){r.onload=function(){o.push(r.result,r.error)};r.onloadend=done;\
             r.readAsText(new Blob(['b']))},10)"
        ),
        "abort(2,0/3) loadend(2,0/3) after(2,null,AbortError:The operation was aborted. ) \
         loadstart(1,0/1) progress(1,1/1) load(2,1/1) b  loadend(2,1/1)"
    );
    // Aborting from inside the read's own events stops it there.
    assert_eq!(
        out(
            "var r=new FileReader();r.onloadstart=function(){r.abort()};watch(r);\
             r.readAsText(new Blob(['abc']));setTimeout(function(){o.push(r.result);done()},10)"
        ),
        "abort(2,0/3) loadend(2,0/3) loadstart(2,0/3) "
    );
    assert_eq!(
        out(
            "var r=new FileReader();watch(r);r.onprogress=function(){r.abort()};\
             r.readAsText(new Blob(['abc']));setTimeout(function(){o.push(r.result);done()},10)"
        ),
        "loadstart(1,0/3) progress(1,3/3) abort(2,0/3) loadend(2,0/3) "
    );
    // A read started from `onload` is allowed; the first read's `loadend` still
    // follows (measured).
    assert_eq!(
        out(
            "var r=new FileReader(),n=0;r.onload=function(){o.push('load'+r.readyState);\
             if(!n++)r.readAsText(new Blob(['b']))};\
             r.onloadend=function(){o.push('loadend'+r.readyState);if(n>1)done()};\
             r.readAsText(new Blob(['a']))"
        ),
        "load2 loadend1 load2 loadend2"
    );
}
