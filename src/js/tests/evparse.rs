//! The end of parsing as a turn of the loop (`bl-9f4c`, js.md §5): timers due
//! when the parser's scripts are done run before `DOMContentLoaded`, as in
//! Firefox 153.0esr. Measured on the identity.md §3.19 rig, three runs each,
//! identical: a page whose head script sets `setTimeout(f, 0)` and
//! `setTimeout(g, 5)` logs `s1 micro s2 t0:interactive DCL load t5:complete`;
//! with a 0 ms interval and a timer set by the 0 ms one, `… rs:interactive t0
//! t0-micro iv0 … DCL rs:complete load t0-nested iv1 tDCL iv2` — the timers a
//! due timer sets, and the interval's next turns, wait for `load`. (Firefox's
//! 1 and 2 ms timers also ran first there: the page took that long to parse, a
//! wall-clock fact; on the frozen test clock they are not yet due.)

use super::drive;
use crate::dom::NodeKind;

fn log(html: &str) -> String {
    let (doc, report) = drive(html);
    assert_eq!(report.errors, 0);
    let body = doc.find_by_tag("body")[0];
    let NodeKind::Element(el) = &doc.node(body).kind else {
        unreachable!("body is an element")
    };
    el.attr("data-log").unwrap_or_default().to_string()
}

const HEAD: &str = "<body><script>var L=[];function w(){document.body.setAttribute('data-log',\
    L.join(' '))}\
    document.addEventListener('DOMContentLoaded',function(){L.push('DCL');w();\
      setTimeout(function(){L.push('tDCL');w()},0)});\
    document.addEventListener('readystatechange',function(){L.push('rs:'+document.readyState)});\
    addEventListener('load',function(){L.push('load');w()});";

#[test]
fn a_timer_due_at_the_end_of_parsing_runs_before_domcontentloaded() {
    let got = log(&format!(
        "{HEAD}setTimeout(function(){{L.push('t0:'+document.readyState);w()}},0);\
         setTimeout(function(){{L.push('t5:'+document.readyState);w()}},5);\
         Promise.resolve().then(function(){{L.push('micro')}});L.push('s1');</script>\
         <script>L.push('s2')</script></body>"
    ));
    assert_eq!(
        got,
        "s1 micro s2 rs:interactive t0:interactive DCL rs:complete load tDCL t5:complete"
    );
}

#[test]
fn what_an_early_timer_sets_waits_for_load() {
    let got = log(&format!(
        "{HEAD}setTimeout(function(){{L.push('t0');setTimeout(function(){{L.push('t0-nested');\
         w()}},0);Promise.resolve().then(function(){{L.push('t0-micro')}})}},0);\
         var n=0,iv=setInterval(function(){{L.push('iv'+n);if(++n==3)clearInterval(iv);w()}},0);\
         L.push('s1');</script></body>"
    ));
    // Up to `load` as Firefox; after it, a tie between the interval's next turn
    // and the nested timer is broken by frot's queue order (Firefox runs
    // `t0-nested` first), a §5 ordering outside this rule.
    let (early, late) = got.split_once(" load ").unwrap();
    assert_eq!(early, "s1 rs:interactive t0 t0-micro iv0 DCL rs:complete");
    let mut late: Vec<_> = late.split(' ').collect();
    late.sort_unstable();
    assert_eq!(late, ["iv1", "iv2", "t0-nested", "tDCL"]);
}
