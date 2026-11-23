//! Iframe windows (js.md §14, bl-54ac): a connected iframe's `contentWindow`
//! is a second realm with its own intrinsics and an `about:blank` document.
//! Every expectation here was read off Firefox 153.0esr (js.md §14.1).

use super::{drive, sess};

/// Evaluate `src` in a fresh session over a page with one appended iframe `f`
/// and its window `w`.
fn with_frame(src: &str) -> String {
    let s = sess("<body><p>host</p></body>");
    s.eval(&format!(
        "var f = document.createElement('iframe'); document.body.appendChild(f); \
         var w = f.contentWindow; {src}"
    ))
    .unwrap()
}

#[test]
fn a_detached_iframe_has_no_window_and_no_document() {
    let s = sess("<body></body>");
    let out = s.eval("var f = document.createElement('iframe'); [f.contentWindow, f.contentDocument].join('|') + (f.contentWindow === null)");
    assert_eq!(out.unwrap(), "|true");
}

#[test]
fn the_window_is_one_object_whose_document_is_the_content_document() {
    let out = with_frame("[w === f.contentWindow, f.contentDocument === w.document, w.document.defaultView === w, w !== window].join()");
    assert_eq!(out, "true,true,true,true");
}

#[test]
fn the_window_is_its_own_realm() {
    let out = with_frame(
        "[w.Array !== Array, w.Object.prototype !== Object.prototype, [] instanceof w.Array, \
          w.Array.isArray([]), w.navigator !== navigator, w.navigator.userAgent === navigator.userAgent, \
          (new w.Function('return Array'))() === w.Array].join()",
    );
    assert_eq!(out, "true,true,false,true,true,true,true");
}

#[test]
fn eval_runs_in_the_frame_global() {
    let out = with_frame(
        "w.eval('var zz = 7'); [w.eval('1+1'), w.eval('this') === w, w.zz, typeof zz].join()",
    );
    assert_eq!(out, "2,true,7,undefined");
}

#[test]
fn the_frame_links_to_its_parent_and_the_parent_counts_it() {
    let out = with_frame(
        "[w.parent === window, w.top === window, w.frameElement === f, window.frameElement, \
          window.parent === window, window.frames === window, window.length].join()",
    );
    assert_eq!(out, "true,true,true,,true,true,1");
}

#[test]
fn the_initial_document_is_a_complete_about_blank() {
    let out = with_frame(
        "[w.location.href, w.document.readyState, w.document.documentElement.outerHTML, \
          document.readyState].join('|')",
    );
    assert_eq!(
        out,
        "about:blank|complete|<html><head></head><body></body></html>|loading"
    );
}

#[test]
fn a_frame_document_is_its_own_tree() {
    let out = with_frame(
        "w.document.body.innerHTML = '<b>x</b>'; \
         [w.document.body.innerHTML, document.querySelectorAll('b').length, \
          w.document.createElement('span') instanceof w.HTMLSpanElement].join()",
    );
    assert_eq!(out, "<b>x</b>,0,true");
}

#[test]
fn frames_nest_and_top_is_the_outermost_window() {
    let out = with_frame(
        "var i = w.document.createElement('iframe'); w.document.body.appendChild(i); \
         var v = i.contentWindow; [v.parent === w, v.top === window, w.length, window.length].join()",
    );
    assert_eq!(out, "true,true,1,1");
}

#[test]
fn a_removed_iframe_has_no_window() {
    let out =
        with_frame("f.remove(); [f.contentWindow === null, f.contentDocument === null].join()");
    assert_eq!(out, "true,true");
}

#[test]
fn length_is_replaceable() {
    let s = sess("<body><iframe></iframe></body>");
    assert_eq!(
        s.eval("var n = length; length = 5; [n, length].join()")
            .unwrap(),
        "1,5"
    );
}

/// One agent, one loop: a frame's timer fires from the top window's queue,
/// with the frame as `this`, and the two windows read one `Date.now`.
#[test]
fn a_frame_timer_runs_on_the_one_event_loop() {
    let (doc, report) = drive(
        "<body><script>\
           var f = document.createElement('iframe'); document.body.appendChild(f);\
           var w = f.contentWindow;\
           w.setTimeout(function () {\
             document.body.setAttribute('data-ft', String(this === w));\
             document.body.setAttribute('data-now', String(w.Date.now() === Date.now()));\
             document.body.setAttribute('data-perf', String(w.performance.now() <= performance.now()));\
           }, 5);\
           w.requestIdleCallback(function () { document.body.setAttribute('data-idle', 'y'); });\
           var c = w.requestIdleCallback(function () { document.body.setAttribute('data-gone', 'y'); });\
           w.cancelIdleCallback(c);\
         </script></body>",
    );
    let body = doc.find_by_tag("body")[0];
    let crate::dom::NodeKind::Element(el) = &doc.node(body).kind else {
        unreachable!("body is an element")
    };
    assert_eq!(el.attr("data-ft"), Some("true"));
    assert_eq!(el.attr("data-now"), Some("true"));
    assert_eq!(el.attr("data-perf"), Some("true"));
    assert_eq!(el.attr("data-idle"), Some("y"));
    assert_eq!(el.attr("data-gone"), None);
    assert_eq!((report.errors, report.settled()), (0, true));
}

/// What the views read is the top document: frame content never enters it.
#[test]
fn frame_content_stays_out_of_the_page_document() {
    let (doc, _) = drive(
        "<body><iframe></iframe><script>\
           document.querySelector('iframe').contentDocument.body.innerHTML = '<p id=inner>secret</p>';\
         </script></body>",
    );
    assert!(doc.find_by_tag("p").is_empty());
}
