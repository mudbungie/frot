use super::*;
use crate::{css, layout};

/// Layout with the full author cascade (`<style>` + inline + UA).
fn full(html: &str) -> (Document, layout::Layout, Styles) {
    let doc = Document::parse(html);
    let styles = css::compute_with(&doc, &[]);
    let l = layout::compute(&doc, &styles, layout::VIEWPORT_WIDTH);
    (doc, l, styles)
}

/// Layout with *bare* styles (UA-implicit display + inline `style=` only) — the
/// no-`--css` path (`layout.md` §3).
fn bare(html: &str) -> (Document, layout::Layout, Styles) {
    let doc = Document::parse(html);
    let styles = css::compute_bare(&doc);
    let l = layout::compute(&doc, &styles, layout::VIEWPORT_WIDTH);
    (doc, l, styles)
}

fn entries(v: Value) -> Vec<Value> {
    match v {
        Value::Array(a) => a,
        _ => panic!("bboxes did not return an array"),
    }
}

fn tag<'a>(out: &'a [Value], name: &str) -> &'a Value {
    out.iter().find(|e| e["tag"] == name).unwrap()
}

#[test]
fn stacked_blocks_emit_source_i_rect_text_in_order() {
    let (doc, l, s) = full("<h1>Welcome</h1><p>Body copy</p>");
    let out = entries(bboxes(&doc, &l, &s));
    let tags: Vec<&str> = out.iter().map(|e| e["tag"].as_str().unwrap()).collect();
    // head is non-rendered (no box) and omitted; the rest emit in source order.
    assert_eq!(tags, ["html", "body", "h1", "p"]);
    let h1 = &out[2];
    assert_eq!(h1["text"], "Welcome");
    // `i` is the source ordinal: html=0, head=1 (skipped), body=2, h1=3, p=4.
    assert_eq!(h1["i"], 3);
    assert_eq!(out[3]["i"], 4);
    // Full-width block at the top; `p` stacks directly below `h1`.
    assert_eq!(h1["rect"]["x"], 0);
    assert_eq!(h1["rect"]["y"], 0);
    assert_eq!(h1["rect"]["w"], 1280);
    assert_eq!(out[3]["rect"]["y"], h1["rect"]["h"]);
}

#[test]
fn display_none_element_is_omitted() {
    let (doc, l, s) = bare("<p>one</p><p style='display:none'>two</p><p>three</p>");
    let out = entries(bboxes(&doc, &l, &s));
    let texts: Vec<&Value> = out
        .iter()
        .filter(|e| e["tag"] == "p")
        .map(|e| &e["text"])
        .collect();
    assert_eq!(texts, [&Value::from("one"), &Value::from("three")]);
}

#[test]
fn visibility_hidden_element_is_included() {
    let (doc, l, s) = bare("<p style='visibility:hidden'>ghost</p>");
    let out = entries(bboxes(&doc, &l, &s));
    assert_eq!(tag(&out, "p")["text"], "ghost");
}

#[test]
fn flex_children_emit_in_reading_order_with_non_monotonic_i() {
    // order:2 pushes `a` last among equal-order `b`/`c`; row-reverse then flips
    // the whole line → reading order a, c, b.
    let (doc, l, s) = bare(
        "<div style='display:flex;flex-direction:row-reverse'>\
         <span style='order:2'>a</span><span>b</span><span>c</span></div>",
    );
    let out = entries(bboxes(&doc, &l, &s));
    let spans: Vec<(&str, i64)> = out
        .iter()
        .filter(|e| e["tag"] == "span")
        .map(|e| (e["text"].as_str().unwrap(), e["i"].as_i64().unwrap()))
        .collect();
    let texts: Vec<&str> = spans.iter().map(|(t, _)| *t).collect();
    assert_eq!(texts, ["a", "c", "b"]);
    // `c`'s source ordinal is the largest yet it is emitted in the middle — the
    // array is reading order, so `i` is not monotonic.
    assert!(spans[1].1 > spans[2].1, "expected non-monotonic i: {:?}", spans);
}

#[test]
fn element_without_direct_text_has_null_text() {
    let (doc, l, s) = full("<div><span>hi</span></div>");
    let out = entries(bboxes(&doc, &l, &s));
    // `div`'s only child is an element, so it has no direct text.
    assert_eq!(tag(&out, "div")["text"], Value::Null);
    assert_eq!(tag(&out, "span")["text"], "hi");
}

#[test]
fn direct_text_is_whitespace_normalized() {
    let (doc, l, s) = full("<h1>  Welcome   good\n friend </h1>");
    let out = entries(bboxes(&doc, &l, &s));
    assert_eq!(tag(&out, "h1")["text"], "Welcome good friend");
}

#[test]
fn bare_ignores_style_block_that_css_would_reorder() {
    let html = "<style>div{display:flex;flex-direction:row-reverse}</style>\
        <div><em>a</em><em>b</em></div>";
    // Bare: `<style>` ignored → div is block flow → source order a, b.
    let doc = Document::parse(html);
    let bare_s = css::compute_bare(&doc);
    let bare_l = layout::compute(&doc, &bare_s, layout::VIEWPORT_WIDTH);
    let bare_out = entries(bboxes(&doc, &bare_l, &bare_s));
    let bare_ems: Vec<&str> = bare_out
        .iter()
        .filter(|e| e["tag"] == "em")
        .map(|e| e["text"].as_str().unwrap())
        .collect();
    assert_eq!(bare_ems, ["a", "b"]);
    // Full: `<style>` applied → flex row-reverse → reading order b, a.
    let full_s = css::compute_with(&doc, &[]);
    let full_l = layout::compute(&doc, &full_s, layout::VIEWPORT_WIDTH);
    let full_out = entries(bboxes(&doc, &full_l, &full_s));
    let full_ems: Vec<&str> = full_out
        .iter()
        .filter(|e| e["tag"] == "em")
        .map(|e| e["text"].as_str().unwrap())
        .collect();
    assert_eq!(full_ems, ["b", "a"]);
}
