//! Focus and the top layer through the page's API (`bl-bc6d`): the state
//! lives in the arena, so `activeElement`, `:focus`, `:modal` and
//! `:popover-open` read what `focus()`, `showModal()` and `showPopover()` wrote.

use super::sess;

const PAGE: &str = "<!DOCTYPE html><html><body><input id=a><input id=b autofocus>\
    <button id=x disabled>x</button><p id=p>p</p>\
    <dialog id=d><span>t</span><button id=db>in</button><input id=da autofocus></dialog>\
    <dialog id=e>plain</dialog><div popover id=pop><input id=pi autofocus></div>\
    <div popover id=bare>b</div></body></html>";

fn eval(script: &str) -> String {
    sess(PAGE)
        .eval(&format!(
            "(function(){{var D=document,$=function(i){{return D.getElementById(i)}};\
             function act(){{return D.activeElement.id||D.activeElement.tagName}}\
             function t(f){{try{{return String(f())}}catch(e){{return e.name}}}}\
             {script}}})()"
        ))
        .unwrap()
}

#[test]
fn an_untouched_page_is_focused_on_its_autofocus_element() {
    assert_eq!(eval("return act()+' '+$('b').matches(':focus')"), "b true");
}

#[test]
fn focus_moves_with_blur_and_focus_events_and_blur_clears_it() {
    let got = eval(
        "var L=[];['a','b'].forEach(function(i){$(i).addEventListener('focus',function(){L.push('f'+i)});\
         $(i).addEventListener('blur',function(){L.push('b'+i)})});\
         $('a').focus();$('a').focus();$('x').focus();$('p').focus();L.push(act());\
         $('b').blur();L.push(act());$('a').blur();L.push(act(),D.querySelectorAll(':focus').length);\
         return L.join(' ')",
    );
    assert_eq!(got, "bb fa a a ba BODY 0");
}

#[test]
fn opening_a_dialog_runs_the_focusing_steps() {
    assert_eq!(
        eval("$('d').show();return act()+' '+$('d').open"),
        "da true"
    );
    assert_eq!(eval("$('e').show();return act()"), "e");
    let modal = eval(
        "$('e').showModal();$('e').showModal();var L=[act(),D.querySelectorAll(':modal').length];\
         $('e').close('v');$('e').close();L.push(D.querySelectorAll(':modal').length,$('e').open,\
         $('e').returnValue);$('e').open=true;L.push($('e').hasAttribute('open'));\
         $('e').open=false;L.push($('e').hasAttribute('open'));return L.join(' ')",
    );
    assert_eq!(modal, "e 1 0 false v true false");
    assert_eq!(eval("$('d').show();$('d').show();return act()"), "da");
}

#[test]
fn a_popover_is_shown_hidden_and_toggled_by_script() {
    let got = eval(
        "var L=[];$('pop').showPopover();$('pop').showPopover();L.push(act(),\
         D.querySelectorAll(':popover-open').length);$('pop').hidePopover();\
         L.push(D.querySelectorAll(':popover-open').length,$('bare').togglePopover(),\
         $('bare').togglePopover(),$('bare').togglePopover(true),$('bare').togglePopover(false),\
         t(function(){return $('p').showPopover()}));return L.join(' ')",
    );
    assert_eq!(got, "pi 1 0 true false true false NotSupportedError");
}
