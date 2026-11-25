//! Scripts a page inserts (js.md §4, `bl-a86b`), against Firefox 153.0esr:
//! an inline one runs inside the call that inserted it, an external or module
//! one runs as a task — whenever it arrives, timers included — and none runs
//! twice. Every expected order below is the one the rig printed, except where
//! a comment names the network race frot resolves one way.

use super::{drive_bounded, drive_env, test_env};
use crate::dom::NodeKind;
use crate::js::{Env, JsStop, Report};

/// A file base with no siblings: a relative `src` fails offline (the repo rule).
const NOWHERE: &str = "file:///nonexistent/page.html";

/// Run `script` as the page's first script (with `L(...)` logging), `after` as
/// a second parser script, and return the log the page wrote at the end.
fn run_at(url: &str, script: &str, after: &str) -> (String, Report) {
    let env = Env {
        url: url.into(),
        ..test_env()
    };
    let (doc, report) = drive_env(
        &format!(
            "<body><script>var log=[];function L(){{log.push([].slice.call(arguments).join(':'))}}\
             {script};setTimeout(function(){{document.body.setAttribute('data-out',log.join(' '))}},9000)\
             </script><script>{after}</script></body>"
        ),
        env,
    );
    let body = doc.find_by_tag("body")[0];
    let NodeKind::Element(el) = &doc.node(body).kind else {
        unreachable!("body is an element")
    };
    (el.attr("data-out").unwrap_or_default().to_string(), report)
}

fn run(script: &str) -> (String, Report) {
    run_at("https://example.com/", script, "L('p2')")
}

/// `add(attrs, text)`: create a script, set its properties, append it to body.
const ADD: &str = "function add(p,t){var s=document.createElement('script');\
    for(var k in p)s[k]=p[k];if(t)s.text=t;document.body.appendChild(s);return s}";

/// An external script that logs `k`, as a `data:` URL.
fn ext(k: &str) -> String {
    format!("data:text/javascript,L('ext','{k}')")
}

#[test]
fn an_inline_script_runs_inside_the_insertion_as_the_current_script() {
    let (out, report) = run(&format!(
        "{ADD};L('before');add({{id:'ii'}},\"L('ran',document.currentScript.id)\");L('after')"
    ));
    assert_eq!(out, "before ran:ii after p2");
    assert_eq!((report.scripts, report.errors), (3, 0));
}

#[test]
fn an_external_script_waits_for_a_task_then_fires_load_or_error() {
    let (out, report) = run_at(
        NOWHERE,
        &format!(
            "{ADD};[[\"{a}\",'a'],['missing.js','m'],[\"{b}\",'b']].forEach(function(p){{\
             var s=add({{src:p[0]}});['load','error'].forEach(function(t){{\
             s.addEventListener(t,function(){{L(t,p[1])}})}})}});L('end')",
            a = ext("a"),
            b = ext("b")
        ),
        "L('p2')",
    );
    assert_eq!(
        out, "end p2 ext:a load:a error:m ext:b load:b",
        "{report:?}"
    );
    assert_eq!((report.scripts, report.errors), (4, 1));
    assert_eq!(report.messages[0].kind, "subfetch");
}

#[test]
fn a_script_a_timer_inserts_runs_before_the_next_timer() {
    // The Cloudflare shape: the next stage is a `<script src>` a timer appends.
    // Firefox runs a 0 ms timer queued after it first, because the fetch takes
    // time; frot's fetch completes at once, which is a schedule a browser may
    // also take.
    let (out, _) = run(&format!(
        "{ADD};setTimeout(function(){{add({{src:\"{}\",onload:function(){{L('loaded')}}}});\
         setTimeout(function(){{L('t2')}},0)}},10)",
        ext("late")
    ));
    assert_eq!(out, "p2 ext:late loaded t2");
}

#[test]
fn a_script_runs_once_and_never_from_markup() {
    let (out, report) = run(&format!(
        "{ADD};var a=add({{}},\"L('a')\");document.body.appendChild(a);\
         document.body.appendChild(a.cloneNode(true));\
         var d=document.createElement('div');d.innerHTML=\"<script>L('html')<\\/script>\";\
         document.body.appendChild(d);document.body.appendChild(d.cloneNode(true));L('end')"
    ));
    assert_eq!(out, "a end p2");
    assert_eq!(report.scripts, 3);
}

#[test]
fn an_empty_or_foreign_script_runs_once_it_becomes_javascript() {
    let (out, _) = run(&format!(
        "{ADD};var e=add({{}});e.textContent=\"L('text')\";\
         var c=add({{}});c.appendChild(document.createTextNode(\"L('child')\"));\
         var s=add({{}});s.src=\"{}\";\
         var u=add({{type:'text/x-foo'}},\"L('foreign')\");L('skipped');\
         u.type='';document.body.appendChild(u);L('end')",
        ext("src")
    ));
    assert_eq!(out, "text child skipped foreign end p2 ext:src");
}

#[test]
fn a_fragment_runs_its_scripts_in_tree_order() {
    let (out, _) = run(
        "var f=document.createDocumentFragment();['one','two'].forEach(function(k){\
         var s=document.createElement('script');s.text=\"L('\"+k+\"')\";f.appendChild(s)});\
         L('built');document.body.appendChild(f);L('end')",
    );
    assert_eq!(out, "built one two end p2");
}

#[test]
fn a_throw_is_the_scripts_and_the_insertion_carries_on() {
    let (out, report) = run(&format!(
        "{ADD};add({{}},\"throw new Error('boom')\");L('after')"
    ));
    assert_eq!(out, "after p2");
    assert_eq!((report.scripts, report.errors), (3, 1));
    assert_eq!(report.messages[0].kind, "throw");
    assert!(report.messages[0].text.contains("boom"));
}

#[test]
fn a_module_waits_for_a_task_inline_or_external() {
    let (out, report) = run(&format!(
        "{ADD};add({{type:'module'}},\"L('inline')\");\
         add({{type:'module',src:\"{}\",onload:function(){{L('loaded')}}}});L('end')",
        ext("module")
    ));
    assert_eq!(out, "end p2 inline ext:module loaded");
    assert_eq!(report.errors, 0);
}

#[test]
fn a_parser_script_waits_for_its_turn_wherever_it_is_moved() {
    // In a browser the parser has not made the second script yet, so the
    // first cannot run it early by moving it or giving it more text.
    let (out, _) = run("var p=document.getElementsByTagName('script')[1];\
         document.body.appendChild(p);p.textContent+=';L(\\'more\\')';L('moved')");
    assert_eq!(out, "moved p2 more");
}

#[test]
fn a_frame_runs_what_is_inserted_into_its_document_in_its_own_window() {
    let (out, _) = run(
        "var f=document.createElement('iframe');document.body.appendChild(f);\
         var d=f.contentDocument,s=d.createElement('script');s.text='window.zz=1';\
         d.body.appendChild(s);L(f.contentWindow.zz,typeof window.zz)",
    );
    assert_eq!(out, "1:undefined p2");
}

/// `bl-b34e`, measured on Firefox 153.0esr (identity.md §3.19 rig): an
/// external, an adopted external, an external module and an inline module
/// inserted into a same-origin `about:blank` frame each run in the frame's
/// window, the classic ones as its `currentScript`, each external then firing
/// `load` (or `error`) at its element. Firefox ran them after a 0 ms timer the
/// page queued, because they were fetched; frot's fetch is instant, so they
/// take their turn after the parser's scripts, as at the top (js.md §4.7).
#[test]
fn a_frame_runs_an_external_or_module_script_inserted_into_it_in_its_own_window() {
    let ext = "data:text/javascript,parent.L('ext',window!==parent,document.currentScript.id)";
    let (out, report) = run_at(
        NOWHERE,
        &format!(
            "var f=document.createElement('iframe');document.body.appendChild(f);             var d=f.contentDocument;function put(doc,p){{var s=doc.createElement('script');             for(var k in p)s[k]=p[k];s.onload=function(){{L('load',s.id)}};             s.onerror=function(){{L('error',s.id)}};d.head.appendChild(s)}}             put(d,{{id:'own',src:\"{ext}\"}});put(document,{{id:'adopted',src:\"{ext}\"}});             put(d,{{id:'m',type:'module',text:\"parent.L('mod',window!==parent)\"}});             put(d,{{id:'xm',type:'module',src:\"data:text/javascript,parent.L('xmod')\"}});\
             put(d,{{id:'bad',src:'missing.js'}});L('after')"
        ),
        "L('p2')",
    );
    assert_eq!(
        out,
        "after p2 ext:true:own load:own ext:true:adopted load:adopted mod:true xmod load:xm \
         error:bad"
    );
    assert_eq!((report.scripts, report.errors), (6, 1));
}

#[test]
fn the_budget_stops_an_inserted_script_and_everything_queued_after_it() {
    let spin = "data:text/javascript,for(;;){}";
    let (doc, report) = drive_bounded(
        &format!(
            "<body><script>{ADD};add({{src:'{spin}',onload:function(){{document.body.id='load'}}}});\
             add({{src:\"data:text/javascript,document.body.id='second'\"}})</script></body>"
        ),
        50,
    );
    assert_eq!(report.stopped, Some(JsStop::Budget));
    let body = doc.find_by_tag("body")[0];
    let NodeKind::Element(el) = &doc.node(body).kind else {
        unreachable!("body is an element")
    };
    assert_eq!(el.attr("id"), None);
}

#[test]
fn the_budget_stops_a_spinning_load_listener() {
    let (_, report) = drive_bounded(
        &format!(
            "<body><script>{ADD};add({{src:\"data:text/javascript,1\",\
             onload:function(){{for(;;){{}}}}}})</script></body>"
        ),
        50,
    );
    assert_eq!(report.stopped, Some(JsStop::Budget));
}
