//! `FileReader` and `ProgressEvent` (`bl-8c46`), against what Firefox 153.0esr
//! does with the same calls (identity.md §3.20).

use super::{drive, sess, Session};
use crate::dom::NodeKind;

fn caught(s: &Session, expr: &str) -> String {
    s.eval(&format!(
        "(function(){{try{{return String({expr})}}catch(e){{return e.name+': '+e.message}}}})()"
    ))
    .unwrap()
}

/// Run `script` as the page's one script and return what it left in
/// `document.body`'s `data-out` once the loop settled.
fn out(script: &str) -> String {
    let (doc, report) = drive(&format!("<body><script>{PRELUDE}{script}</script></body>"));
    assert_eq!((report.errors, report.settled()), (0, true), "{script}");
    let body = doc.find_by_tag("body")[0];
    let NodeKind::Element(el) = &doc.node(body).kind else {
        unreachable!("body is an element")
    };
    el.attr("data-out").unwrap_or_default().to_string()
}

/// `watch(r)` logs every event as `type(readyState,loaded/total)`; `done()`
/// publishes the log.
const PRELUDE: &str =
    "var o=[];function done(){document.body.setAttribute('data-out',o.join(' '))}\
function watch(r){['loadstart','progress','load','abort','error','loadend'].forEach(function(t){\
r.addEventListener(t,function(e){o.push(t+'('+r.readyState+','+e.loaded+'/'+e.total+')')})})}\
function hex(b){return Array.from(new Uint8Array(b),function(x){return (x+256).toString(16)\
.slice(1)}).join('')}";

#[test]
fn file_reader_is_firefox_shaped() {
    let s = sess("<html><body></body></html>");
    let desc = "function desc(o,k){var d=Object.getOwnPropertyDescriptor(o,k);\
                return Object.keys(d).map(function(x){return x+':'+(typeof d[x]==='function'?\
                'fn '+d[x].name+'/'+d[x].length:String(d[x]))}).join(' ')}";
    s.eval(desc).unwrap();
    for (expr, want) in [
        ("typeof FileReader", "function"),
        ("FileReader.length", "0"),
        (
            "Object.getOwnPropertyNames(FileReader.prototype)",
            "readAsArrayBuffer,readAsBinaryString,readAsText,readAsDataURL,abort,readyState,\
             result,error,onloadstart,onprogress,onload,onabort,onerror,onloadend,EMPTY,LOADING,\
             DONE,constructor",
        ),
        (
            "Object.getPrototypeOf(FileReader.prototype)===EventTarget.prototype",
            "true",
        ),
        (
            "[FileReader.EMPTY,FileReader.LOADING,FileReader.DONE,new FileReader().DONE]",
            "0,1,2,2",
        ),
        (
            "desc(FileReader,'DONE')",
            "value:2 writable:false enumerable:true configurable:false",
        ),
        (
            "desc(FileReader.prototype,'LOADING')",
            "value:1 writable:false enumerable:true configurable:false",
        ),
        (
            "desc(FileReader.prototype,'readyState')",
            "get:fn get readyState/0 set:undefined enumerable:true configurable:true",
        ),
        (
            "desc(FileReader.prototype,'onload')",
            "get:fn get onload/0 set:fn set onload/1 enumerable:true configurable:true",
        ),
        (
            "desc(FileReader.prototype,'readAsDataURL')",
            "value:fn readAsDataURL/1 writable:true enumerable:true configurable:true",
        ),
        (
            "desc(FileReader.prototype,'abort')",
            "value:fn abort/0 writable:true enumerable:true configurable:true",
        ),
        (
            "desc(globalThis,'FileReader')",
            "value:fn FileReader/0 writable:true enumerable:false configurable:true",
        ),
        (
            "(function(){var r=new FileReader();return [String(r),r.readyState,r.result,r.error,\
             r.onload,Object.getOwnPropertyNames(r).length,r instanceof EventTarget].map(String)})()",
            "[object FileReader],0,null,null,null,0,true",
        ),
        // An event handler holds a function or nothing.
        (
            "(function(){var r=new FileReader(),f=function(){};r.onload=5;var a=r.onload;\
             r.onload=f;var b=r.onload===f;r.onload=null;return [a,b,r.onload]})()",
            ",true,",
        ),
        // Aborting a reader that is not reading does nothing at all.
        (
            "(function(){var r=new FileReader();r.abort();return [r.readyState,r.result,r.error]\
             .map(String)})()",
            "0,null,null",
        ),
    ] {
        assert_eq!(caught(&s, expr), want, "{expr}");
    }
}

#[test]
fn file_reader_errors_are_firefoxs() {
    let s = sess("<html><body></body></html>");
    s.eval("var B=new Blob(['hi'])").unwrap();
    for (expr, want) in [
        (
            "new FileReader().readAsText()",
            "TypeError: FileReader.readAsText: At least 1 argument required, but only 0 passed",
        ),
        (
            "new FileReader().readAsText({})",
            "TypeError: FileReader.readAsText: Argument 1 does not implement interface Blob.",
        ),
        (
            "new FileReader().readAsDataURL('x')",
            "TypeError: FileReader.readAsDataURL: Argument 1 is not an object.",
        ),
        (
            "new FileReader().readAsArrayBuffer(null)",
            "TypeError: FileReader.readAsArrayBuffer: Argument 1 is not an object.",
        ),
        (
            "FileReader.prototype.readAsBinaryString.call({},B)",
            "TypeError: 'readAsBinaryString' called on an object that does not implement \
             interface FileReader.",
        ),
        (
            "FileReader.prototype.abort.call({})",
            "TypeError: 'abort' called on an object that does not implement interface FileReader.",
        ),
        (
            "Object.getOwnPropertyDescriptor(FileReader.prototype,'readyState').get.call({})",
            "TypeError: 'get readyState' called on an object that does not implement interface \
             FileReader.",
        ),
        (
            "Object.getOwnPropertyDescriptor(FileReader.prototype,'onload').set.call({},null)",
            "TypeError: 'set onload' called on an object that does not implement interface \
             FileReader.",
        ),
        (
            "Object.getOwnPropertyDescriptor(FileReader.prototype,'onload').get.call({})",
            "TypeError: 'get onload' called on an object that does not implement interface \
             FileReader.",
        ),
        // A second read while one is in flight.
        (
            "(function(){var r=new FileReader();r.readAsText(B);r.readAsText(B)})()",
            "InvalidStateError: An attempt was made to use an object that is not, or is no \
             longer, usable",
        ),
    ] {
        assert_eq!(caught(&s, expr), want, "{expr}");
    }
}

#[test]
fn progress_event_is_firefox_shaped() {
    let s = sess("<html><body></body></html>");
    for (expr, want) in [
        ("typeof ProgressEvent", "function"),
        ("ProgressEvent.length", "1"),
        (
            "Object.getOwnPropertyNames(ProgressEvent.prototype)",
            "lengthComputable,loaded,total,constructor",
        ),
        (
            "Object.getPrototypeOf(ProgressEvent.prototype)===Event.prototype",
            "true",
        ),
        (
            "(function(){var e=new ProgressEvent('x');return [e.type,e.lengthComputable,e.loaded,\
             e.total,e.bubbles,e.isTrusted,Object.getOwnPropertyNames(e),String(e),\
             e instanceof Event]})()",
            "x,false,0,0,false,false,isTrusted,[object ProgressEvent],true",
        ),
        (
            "(function(){var e=new ProgressEvent('x',{lengthComputable:1,loaded:5.7,total:'9',\
             bubbles:true});return [e.lengthComputable,e.loaded,e.total,e.bubbles]})()",
            "true,5.7,9,true",
        ),
        ("new ProgressEvent('x',null).loaded", "0"),
        ("new ProgressEvent('x',{loaded:-1}).loaded", "-1"),
        (
            "new ProgressEvent()",
            "TypeError: ProgressEvent constructor: At least 1 argument required, but only 0 passed",
        ),
        (
            "new ProgressEvent('x',5)",
            "TypeError: ProgressEvent constructor: Value can't be converted to a dictionary.",
        ),
        (
            "new ProgressEvent('x',{loaded:NaN})",
            "TypeError: ProgressEvent constructor: 'loaded' member of ProgressEventInit is not a \
             finite floating-point value.",
        ),
        (
            "new ProgressEvent('x',{total:Infinity})",
            "TypeError: ProgressEvent constructor: 'total' member of ProgressEventInit is not a \
             finite floating-point value.",
        ),
    ] {
        assert_eq!(caught(&s, expr), want, "{expr}");
    }
}

mod reads;
