//! The geometry syscalls driven through raw `__frot_rect` / `__frot_computed_style`
//! calls, so the Rust cache reaches 100% coverage without leaning on the JS
//! prelude (`llvm-cov` cannot see it — that is the golden suite's job, js.md §3).

use crate::dom::Document;
use crate::fetch::FetchSession;
use crate::js::{Env, Session, StyleSource};

fn env() -> Env {
    Env {
        url: "https://example.com/".into(),
        user_agent: "frot-test/1".into(),
    }
}

fn bare(html: &str) -> Session {
    Session::new(Document::parse(html), StyleSource::Bare, env(), &FetchSession::new(Vec::new()))
}

fn authored(html: &str, external: Vec<String>) -> Session {
    Session::new(
        Document::parse(html),
        StyleSource::Authored(external),
        env(),
        &FetchSession::new(Vec::new()),
    )
}

/// A rendered block gets a real box (full 1280 viewport, one line tall); a
/// box-less node reads back the all-zero rect — the total, spec-legal answer.
#[test]
fn rect_is_the_box_or_all_zero_for_box_less_nodes() {
    let s = bare("<html><body><div id='a'>hi</div><div id='b' style='display:none'>x</div></body></html>");
    s.eval("globalThis.a = __frot_query_doc('#a')[0]").unwrap();
    s.eval("globalThis.b = __frot_query_doc('#b')[0]").unwrap();
    assert_eq!(s.eval("JSON.stringify(__frot_rect(a))").unwrap(), "[0,0,1280,20]");
    // display:none → box-less → the Rect::ZERO arm of unwrap_or.
    assert_eq!(s.eval("JSON.stringify(__frot_rect(b))").unwrap(), "[0,0,0,0]");
    // a non-element (a's own text node) is box-less too.
    assert_eq!(s.eval("JSON.stringify(__frot_rect(__frot_children(a)[0]))").unwrap(), "[0,0,0,0]");
}

/// `getComputedStyle`'s `display` covers every one of the seven keywords the
/// cascade can compute; inline `style=` applies under the bare policy.
#[test]
fn computed_display_covers_all_seven_keywords() {
    let s = bare(
        "<html><body>\
         <i id='d0' style='display:none'></i>\
         <i id='d1' style='display:block'></i>\
         <i id='d2' style='display:inline'></i>\
         <i id='d3' style='display:inline-block'></i>\
         <i id='d4' style='display:list-item'></i>\
         <i id='d5' style='display:flex'></i>\
         <i id='d6' style='display:inline-flex'></i>\
         </body></html>",
    );
    let want = ["none", "block", "inline", "inline-block", "list-item", "flex", "inline-flex"];
    for (i, kw) in want.iter().enumerate() {
        let js = format!("__frot_computed_style(__frot_query_doc('#d{i}')[0], 'display')");
        assert_eq!(&s.eval(&js).unwrap(), kw);
    }
}

/// The rest of the §8 subset — `visibility`, `order`, all four `flex-direction`
/// values — plus the `""` fall-through for any other property.
#[test]
fn computed_visibility_order_flex_direction_and_unknown() {
    let s = bare(
        "<html><body>\
         <i id='v' style='visibility:hidden'></i>\
         <i id='o' style='order:5'></i>\
         <i id='r0' style='flex-direction:row'></i>\
         <i id='r1' style='flex-direction:row-reverse'></i>\
         <i id='r2' style='flex-direction:column'></i>\
         <i id='r3' style='flex-direction:column-reverse'></i>\
         <i id='plain'></i>\
         </body></html>",
    );
    let cs = |sel: &str, prop: &str| {
        s.eval(&format!("__frot_computed_style(__frot_query_doc('{sel}')[0], '{prop}')")).unwrap()
    };
    assert_eq!(cs("#v", "visibility"), "hidden");
    assert_eq!(cs("#plain", "visibility"), "visible");
    assert_eq!(cs("#o", "order"), "5");
    assert_eq!(cs("#plain", "order"), "0");
    assert_eq!(cs("#r0", "flex-direction"), "row");
    assert_eq!(cs("#r1", "flex-direction"), "row-reverse");
    assert_eq!(cs("#r2", "flex-direction"), "column");
    assert_eq!(cs("#r3", "flex-direction"), "column-reverse");
    // Anything outside the subset reads back "" — no faked computed set.
    assert_eq!(cs("#plain", "color"), "");
}

/// The cache computes once per generation: reused while unchanged, recomputed
/// after a mutation bumps [`Document::generation`] (js.md §2/§8).
#[test]
fn cache_reuses_within_a_generation_and_invalidates_on_mutation() {
    let s = bare("<html><body><div id='a'>hi</div></body></html>");
    s.eval("globalThis.a = __frot_query_doc('#a')[0]").unwrap();
    // First query computes (cache: None); second reuses (fresh hit).
    assert_eq!(s.eval("JSON.stringify(__frot_rect(a))").unwrap(), "[0,0,1280,20]");
    assert_eq!(s.eval("JSON.stringify(__frot_rect(a))").unwrap(), "[0,0,1280,20]");
    // A mutation bumps the generation; the next query recomputes (stale miss).
    s.eval("__frot_set_attr(a, 'style', 'display:none')").unwrap();
    assert_eq!(s.eval("JSON.stringify(__frot_rect(a))").unwrap(), "[0,0,0,0]");
}

/// The styling policy decides the cache's cascade: bare ignores `<style>`; the
/// authored policy applies both the document's `<style>` and external sheets.
#[test]
fn style_source_selects_bare_or_authored_cascade() {
    let html = "<html><head><style>#a{display:none}</style></head><body><div id='a'>hi</div></body></html>";
    // Bare ignores the <style> block: #a still renders.
    let b = bare(html);
    b.eval("globalThis.a = __frot_query_doc('#a')[0]").unwrap();
    assert_eq!(b.eval("__frot_computed_style(a, 'display')").unwrap(), "block");
    assert_eq!(b.eval("JSON.stringify(__frot_rect(a))").unwrap(), "[0,0,1280,20]");
    // Authored applies the document's own <style>: #a is display:none.
    let a = authored(html, Vec::new());
    a.eval("globalThis.a = __frot_query_doc('#a')[0]").unwrap();
    assert_eq!(a.eval("__frot_computed_style(a, 'display')").unwrap(), "none");
    // Authored also applies external <link> sheet texts handed to the session.
    let e = authored("<html><body><div id='a'>hi</div></body></html>", vec!["#a{display:none}".into()]);
    e.eval("globalThis.a = __frot_query_doc('#a')[0]").unwrap();
    assert_eq!(e.eval("__frot_computed_style(a, 'display')").unwrap(), "none");
}
