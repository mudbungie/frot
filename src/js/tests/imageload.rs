//! What an `<img>`'s `src` does (`bl-91a4`), against Firefox 153.0esr with
//! images blocked: a `data:` image loads with its real size, everything else
//! fails, on a task, and the document's `load` waits for both.

use super::{drive, drive_bounded, sess};
use crate::dom::NodeKind;
use crate::js::{JsStop, Report};

const GIF: &str = "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";

/// Run a page whose scripts write `log` into `<body data-out>` at the end of
/// the run (a far timer), and return it with the report.
fn run(script: &str) -> (String, Report) {
    let (doc, report) = drive(&format!(
        "<body><script>var log=[];function L(){{log.push([].slice.call(arguments).join(':'))}}\
         {script};setTimeout(function(){{document.body.setAttribute('data-out',log.join(' '))}},9000)\
         </script></body>"
    ));
    let body = doc.find_by_tag("body")[0];
    let NodeKind::Element(el) = &doc.node(body).kind else {
        unreachable!("body is an element")
    };
    (el.attr("data-out").unwrap_or_default().to_string(), report)
}

/// `watch(img, label)`: log each event with what the image reports when it fires.
const WATCH: &str = "function watch(i,k){['load','error'].forEach(function(t){\
    i.addEventListener(t,function(e){L(k,t,e.isTrusted,e.bubbles,i.complete,i.naturalWidth,\
    i.naturalHeight,i.width,i.currentSrc.slice(0,10))})})}";

#[test]
fn a_data_image_loads_and_everything_else_fails() {
    let (out, report) = run(&format!(
        "{WATCH};[['gif','{GIF}'],['http','/a.png'],['text','data:,x'],['empty','']]\
         .forEach(function(p){{var i=new Image();watch(i,p[0]);i.src=p[1];L(p[0],'sync',i.complete)}})"
    ));
    assert_eq!(report.errors, 0);
    assert_eq!(
        out,
        "gif:sync:false http:sync:false text:sync:false empty:sync:true \
         gif:load:true:false:true:1:1:1:data:image \
         http:error:true:false:true:0:0:0:https://ex \
         text:error:true:false:true:0:0:0:data:,x \
         empty:error:true:false:true:0:0:0:"
    );
}

/// The parsed document's images end before its `load` (measured: both before
/// `window` `load`); an `<img>` with no `src` fires nothing and is complete.
#[test]
fn the_document_load_waits_for_its_images() {
    let (doc, report) = drive(&format!(
        "<body><img id=a src=/a.png><img id=b src='{GIF}'><img id=c>\
         <script>var log=[];['a','b','c'].forEach(function(k){{var i=document.getElementById(k);\
         i.onload=i.onerror=function(e){{log.push(k+':'+e.type)}}}});\
         addEventListener('load',function(){{log.push('load',document.getElementById('c').complete);\
         document.body.setAttribute('data-out',log.join())}})</script></body>"
    ));
    assert_eq!(report.errors, 0);
    let body = doc.find_by_tag("body")[0];
    let NodeKind::Element(el) = &doc.node(body).kind else {
        unreachable!("body is an element")
    };
    assert_eq!(el.attr("data-out"), Some("a:error,b:load,load,true"));
}

/// A new `src` abandons the request before it; removing `src` abandons it and
/// fires nothing; setting the same `src` again is a new request.
#[test]
fn only_the_latest_request_ends() {
    let (out, _) = run(&format!(
        "{WATCH};var a=new Image();watch(a,'a');a.src='/x.png';a.src='{GIF}';\
         var b=new Image();watch(b,'b');b.src='/x.png';b.removeAttribute('src');L('b',b.complete);\
         b.removeAttribute('src');var c=new Image();watch(c,'c');c.setAttribute('SRC','/y');\
         var d=document.createElement('div');d.setAttribute('src','/z');watch(d,'d')"
    ));
    assert_eq!(
        out,
        "b:true a:load:true:false:true:1:1:1:data:image c:error:true:false:true:0:0:0:https://ex"
    );
}

/// Markup and clones start requests too: one seam, every API.
#[test]
fn inner_html_and_clones_request_their_images() {
    let (out, _) = run(
        "var d=document.createElement('div');d.innerHTML='<p><img src=/q.png></p>';\
         var i=d.querySelector('img');i.onerror=function(){L('inner')};\
         var c=i.cloneNode();c.addEventListener('error',function(){L('clone')})",
    );
    assert_eq!(out, "inner clone");
}

/// After `load`, an image task runs ahead of any timer — it is due now.
#[test]
fn a_late_image_runs_ahead_of_the_timers() {
    let (out, _) = run(
        "setTimeout(function(){var i=new Image();i.onerror=function(){L('error')};i.src='/a';\
         setTimeout(function(){L('timer')},0)},10)",
    );
    assert_eq!(out, "error timer");
}

/// `decode()`, against Firefox's messages for each outcome.
#[test]
fn decode_settles_with_the_request() {
    let (out, _) = run(&format!(
        "function t(k,i){{i.decode().then(function(v){{L(k,'ok',v)}},function(e){{L(k,e.name,e.message)}})}}\
         t('none',new Image());var a=new Image();a.src='{GIF}';t('good',a);\
         var b=new Image();b.src='/x';t('bad',b);var c=new Image();c.src='';t('empty',c);\
         var d=new Image();d.src='{GIF}';t('changed',d);d.src='/y';\
         var e=new Image();e.src='{GIF}';e.onload=function(){{t('loaded',e)}};\
         var f=new Image();f.src='/x';f.onerror=function(){{t('failed',f)}};L(HTMLImageElement.prototype.decode.length)"
    ));
    assert_eq!(
        out,
        "0 none:EncodingError:Invalid image request. empty:EncodingError:Invalid image request. \
         changed:EncodingError:Invalid image request. good:ok: bad:EncodingError:Invalid encoded image data. \
         loaded:ok: failed:EncodingError:Invalid encoded image data."
    );
}

/// A handler that throws is a page error, counted like any other; the image
/// still ends and the run settles.
#[test]
fn an_image_handler_that_throws_is_counted() {
    let (_, report) = run(
        "var i=new Image();i.onerror=function(){throw 1};i.addEventListener('error',function(){throw 2});i.src='/a';\
         setTimeout(function(){var j=new Image();j.onerror=function(){throw 3};j.src='/b'},5)",
    );
    assert_eq!((report.errors, report.settled()), (3, true));
}

/// The compute budget spans image tasks: a spinning handler stops the run
/// before the document's `load`.
#[test]
fn a_budget_trip_inside_an_image_handler_stops_the_loop() {
    let (_, report) = drive_bounded(
        "<body><img src=/a.png><script>document.querySelector('img').onerror=function(){while(true){}};\
         onload=function(){document.body.appendChild(document.createElement('hr'))}</script></body>",
        20,
    );
    assert_eq!(report.stopped, Some(JsStop::Budget));
}

/// The interface: Firefox's 27 members in its order, and its reflections.
#[test]
fn the_image_interface_is_firefox_shaped() {
    let s = sess("<html><body><img id=p src=/a.png></body></html>");
    for (expr, want) in [
        (
            "Object.getOwnPropertyNames(HTMLImageElement.prototype).join()",
            "decode,alt,src,srcset,crossOrigin,useMap,referrerPolicy,isMap,width,height,decoding,\
             loading,fetchPriority,naturalWidth,naturalHeight,complete,name,align,hspace,vspace,\
             longDesc,border,sizes,currentSrc,lowsrc,x,y,constructor",
        ),
        (
            "(function(){var i=new Image();return [i.crossOrigin,i.decoding,i.loading,i.fetchPriority,\
             i.referrerPolicy,i.alt,i.isMap,i.hspace,i.longDesc,i.x,i.y].map(String)})()",
            "null,auto,eager,auto,,,false,0,,0,0",
        ),
        (
            "(function(){var o=[];[['crossorigin',['','USE-credentials','bogus'],'crossOrigin'],\
             ['decoding',['SYNC','bogus'],'decoding'],['loading',['LAZY','bogus'],'loading'],\
             ['fetchpriority',['HIGH','bogus'],'fetchPriority'],\
             ['referrerpolicy',['no-referrer','bogus'],'referrerPolicy'],['hspace',['-3','7'],'hspace'],\
             ['longdesc',['/x'],'longDesc'],['lowsrc',['y.png'],'lowsrc'],['alt',['A'],'alt']]\
             .forEach(function(r){r[1].forEach(function(v){var i=new Image();i.setAttribute(r[0],v);\
             o.push(i[r[2]])})});return o.join('|')})()",
            "anonymous|use-credentials|anonymous|sync|auto|lazy|eager|high|auto|no-referrer||0|7|\
             https://example.com/x|https://example.com/y.png|A",
        ),
        (
            "(function(){var i=new Image();i.isMap=true;var a=i.outerHTML;i.isMap=false;\
             i.crossOrigin='x';var b=i.outerHTML;i.crossOrigin=null;i.alt='q';\
             return [a,b,i.outerHTML].join('|')})()",
            "<img ismap=\"\">|<img crossorigin=\"x\">|<img alt=\"q\">",
        ),
        ("document.getElementById('p').x", "0"),
        (
            "['decode','alt','complete','x'].map(function(k){var d=Object.getOwnPropertyDescriptor(\
             HTMLImageElement.prototype,k);return String(d.value||d.get).indexOf('native code')>0})",
            "true,true,true,true",
        ),
    ] {
        assert_eq!(s.eval(expr).unwrap(), want, "{expr}");
    }
}
