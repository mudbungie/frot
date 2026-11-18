//! `requestIdleCallback` (`bl-b584`) on the virtual-clock loop: the ordering,
//! shape and errors read off Firefox 153.0esr (identity.md §3.20).

use super::{drive, sess};
use crate::dom::NodeKind;

/// Run `script` as the page's one script and return what it left in
/// `document.body`'s `data-out` once the loop settled.
fn out(script: &str) -> String {
    let (doc, report) = drive(&format!("<body><script>{script}</script></body>"));
    assert_eq!((report.errors, report.settled()), (0, true));
    let body = doc.find_by_tag("body")[0];
    let NodeKind::Element(el) = &doc.node(body).kind else {
        unreachable!("body is an element")
    };
    el.attr("data-out").unwrap_or_default().to_string()
}

const LOG: &str = "var o=[];function done(){document.body.setAttribute('data-out',o.join())}";

#[test]
fn an_idle_callback_runs_after_everything_else_due_at_that_instant() {
    // The measured order: microtask, the timeout queued AFTER the idle request,
    // then the idle callback. A cancelled request never runs; handles count
    // from 1 in their own sequence.
    assert_eq!(
        out(&format!(
            "{LOG} var h1=requestIdleCallback(function(d){{o.push('idle1');done()}});\
             var h2=requestIdleCallback(function(){{o.push('idle2')}});cancelIdleCallback(h2);\
             setTimeout(function(){{o.push('timeout0')}},0);\
             Promise.resolve().then(function(){{o.push('micro')}});o.push(h1,h2);"
        )),
        "1,2,micro,timeout0,idle1"
    );
    // Request order among idle callbacks; one requested from inside an idle
    // callback waits for a later period, after a timeout queued beside it.
    assert_eq!(
        out(&format!(
            "{LOG} requestIdleCallback(function(){{o.push('a');requestIdleCallback(function(){{\
             o.push('inner');done()}});setTimeout(function(){{o.push('t0')}},0)}});\
             requestIdleCallback(function(){{o.push('b')}});"
        )),
        "a,b,t0,inner"
    );
}

#[test]
fn the_deadline_is_firefox_shaped_and_counts_down() {
    assert_eq!(
        out(&format!(
            "{LOG} var kept;requestIdleCallback(function(d){{'use strict';kept=d;\
             o.push(String(d),Object.getOwnPropertyNames(d).length,d.didTimeout,arguments.length,\
             this===undefined,d.timeRemaining()>0,d.timeRemaining()<=50)}},{{timeout:1}});\
             setTimeout(function(){{o.push(kept.timeRemaining(),kept.didTimeout);done()}},200);"
        )),
        "[object IdleDeadline],0,false,1,true,true,true,0,false"
    );
    // A timer due inside the 50 ms caps the period at its due time.
    assert_eq!(
        out(&format!(
            "{LOG} setTimeout(function(){{}},11);requestIdleCallback(function(d){{\
             o.push(d.timeRemaining()<=10);done()}});"
        )),
        "true"
    );
}

#[test]
fn the_surface_and_its_errors_are_as_measured() {
    let s = sess("<html><body></body></html>");
    let caught = |expr: &str| {
        s.eval(&format!(
            "(function(){{try{{return String({expr})}}catch(e){{return e.name+': '+e.message}}}})()"
        ))
        .unwrap()
    };
    assert_eq!(
        caught(
            "[requestIdleCallback.length,requestIdleCallback.name,cancelIdleCallback.length,\
             cancelIdleCallback.name,Object.getOwnPropertyNames(IdleDeadline.prototype),\
             String(cancelIdleCallback(999)),String(cancelIdleCallback('x')),\
             typeof requestIdleCallback(function(){},null),typeof __frot_idle_hook,\
             requestIdleCallback.toString().indexOf('native code')>0]"
        ),
        "1,requestIdleCallback,1,cancelIdleCallback,timeRemaining,didTimeout,constructor,\
         undefined,undefined,number,undefined,true"
    );
    for (call, want) in [
        (
            "requestIdleCallback()",
            "Window.requestIdleCallback: At least 1 argument required, but only 0 passed",
        ),
        (
            "requestIdleCallback({})",
            "Window.requestIdleCallback: Argument 1 is not callable.",
        ),
        (
            "requestIdleCallback(null)",
            "Window.requestIdleCallback: Argument 1 is not an object.",
        ),
        (
            "requestIdleCallback('x')",
            "Window.requestIdleCallback: Argument 1 is not an object.",
        ),
        (
            "requestIdleCallback(function(){},5)",
            "Window.requestIdleCallback: Argument 2 can't be converted to a dictionary.",
        ),
        (
            "cancelIdleCallback()",
            "Window.cancelIdleCallback: At least 1 argument required, but only 0 passed",
        ),
        ("new IdleDeadline()", "Illegal constructor."),
        (
            "IdleDeadline.prototype.timeRemaining.call({})",
            "'timeRemaining' called on an object that does not implement interface IdleDeadline.",
        ),
    ] {
        assert_eq!(caught(call), format!("TypeError: {want}"), "{call}");
    }
}

/// A callback that throws is one counted error, and the loop goes on.
#[test]
fn a_throwing_idle_callback_is_counted_and_the_loop_continues() {
    let (doc, report) = drive(
        "<body><script>requestIdleCallback(function(){throw new Error('x')});\
         requestIdleCallback(function(){document.body.appendChild(document.createElement('hr'))});\
         </script></body>",
    );
    assert_eq!((report.errors, report.settled()), (1, true));
    assert_eq!(doc.find_by_tag("hr").len(), 1);
}
