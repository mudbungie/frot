//! The EventTarget trio called with no receiver (`bl-a7c5`): in strict code a
//! bare `addEventListener(…)` — or the function called through a variable —
//! has `this === undefined`, and a browser resolves that to the window.
//! Cloudflare's challenge script opens with exactly that call.

use super::sess;

#[test]
fn a_receiverless_call_is_a_call_on_the_window() {
    let s = sess("<html></html>");
    let got = s
        .eval(
            "(function(){'use strict';var o=[];\
             var add=addEventListener,rm=removeEventListener,fire=dispatchEvent;\
             function f(e){o.push(e.type+(e.target===window))}\
             add('ping',f);addEventListener('pong',f);\
             window.dispatchEvent(new Event('ping'));o.push(fire(new Event('pong')));\
             rm('ping',f);window.removeEventListener('pong',f);\
             fire(new Event('ping'));dispatchEvent(new Event('pong'));\
             return o.join()})()",
        )
        .unwrap();
    assert_eq!(got, "pingtrue,pongtrue,true");
}
