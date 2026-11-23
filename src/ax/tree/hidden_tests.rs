//! `aria-hidden` / `inert` subtree exclusion ([`crate::ax::hidden`]).
//!
//! Every case is paired with its inverse, and the browser-observed Angular
//! shape (aria-hidden SVGs + inert tabpanels) is pinned at the bottom.

use super::*;
use crate::views::{dom::dom_json, text::text};

fn tree(html: &str) -> Value {
    ax_tree(&Document::parse(html), None)
}

fn count_role(v: &Value, role: &str) -> usize {
    let mut n = 0;
    if let Value::Array(arr) = v {
        for item in arr {
            if item["role"] == role {
                n += 1;
            }
            n += count_role(&item["children"], role);
        }
    }
    n
}

fn names(v: &Value, role: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Value::Array(arr) = v {
        for item in arr {
            if item["role"] == role {
                out.push(item["name"].as_str().unwrap_or("<null>").to_string());
            }
            out.extend(names(&item["children"], role));
        }
    }
    out
}

#[test]
fn aria_hidden_true_removes_element_and_subtree() {
    let v = tree("<div aria-hidden='true'><h1>Gone</h1></div><h2>Kept</h2>");
    assert_eq!(count_role(&v, "heading"), 1);
    assert_eq!(names(&v, "heading"), vec!["Kept"]);
}

#[test]
fn aria_hidden_false_keeps_the_subtree() {
    let v = tree("<div aria-hidden='false'><h1>Kept</h1></div>");
    assert_eq!(names(&v, "heading"), vec!["Kept"]);
}

#[test]
fn aria_hidden_true_is_ascii_case_insensitive() {
    let v = tree("<div aria-hidden='TRUE'><h1>Gone</h1></div>");
    assert_eq!(count_role(&v, "heading"), 0);
}

#[test]
fn invalid_aria_hidden_value_is_not_hiding() {
    // The attribute is an enumerated true/false defaulting to false, so an
    // unparseable token leaves the node exposed.
    let v = tree("<div aria-hidden='yes'><h1>Kept</h1></div>");
    assert_eq!(names(&v, "heading"), vec!["Kept"]);
}

#[test]
fn empty_aria_hidden_value_is_not_hiding() {
    let v = tree("<div aria-hidden=''><h1>Kept</h1></div>");
    assert_eq!(names(&v, "heading"), vec!["Kept"]);
}

#[test]
fn aria_hidden_false_cannot_rescue_a_hidden_ancestor() {
    let v = tree("<div aria-hidden='true'><span aria-hidden='false'><h1>Gone</h1></span></div>");
    assert_eq!(count_role(&v, "heading"), 0);
}

#[test]
fn explicit_role_inside_a_hidden_ancestor_is_still_excluded() {
    let v = tree("<div aria-hidden='true'><div role='button'>Press</div></div>");
    assert_eq!(count_role(&v, "button"), 0);
}

#[test]
fn focusable_content_under_aria_hidden_is_still_excluded() {
    // Authoring error; Chromium's only carve-out is the *focused* element and
    // frot never drives a page, so the subtree is cut unconditionally.
    let v = tree("<div aria-hidden='true'><button>Press</button><a href='/x'>L</a></div>");
    assert_eq!(count_role(&v, "button"), 0);
    assert_eq!(count_role(&v, "link"), 0);
}

#[test]
fn inert_removes_element_and_subtree() {
    let v = tree("<div inert><h1>Gone</h1></div><h2>Kept</h2>");
    assert_eq!(names(&v, "heading"), vec!["Kept"]);
}

#[test]
fn inert_false_is_still_inert() {
    // Boolean attribute: presence is the whole signal.
    let v = tree("<div inert='false'><h1>Gone</h1></div>");
    assert_eq!(count_role(&v, "heading"), 0);
}

#[test]
fn nested_inert_descendant_is_excluded_once() {
    let v = tree("<div inert><section inert><h1>Gone</h1></section></div><h2>Kept</h2>");
    assert_eq!(names(&v, "heading"), vec!["Kept"]);
}

#[test]
fn a_non_inert_sibling_of_an_inert_subtree_survives() {
    let v = tree("<div inert><h1>Gone</h1></div><div><h2>Kept</h2></div>");
    assert_eq!(names(&v, "heading"), vec!["Kept"]);
}

/// The shape observed on angular.dev: decorative SVGs marked `aria-hidden` and
/// a tab panel set where only the selected panel is non-`inert`. Chrome exposes
/// the one live panel and none of the hidden graphics.
#[test]
fn angular_tabpanel_and_svg_shape() {
    let v = tree(
        "<svg aria-hidden='true'><title>Deco</title></svg>\
         <svg aria-hidden='true'></svg>\
         <svg><title>Real</title></svg>\
         <div role='tabpanel' aria-label='Signals'>A</div>\
         <div role='tabpanel' aria-label='Control Flow' inert>B</div>\
         <div role='tabpanel' aria-label='Deferrable Views' inert>C</div>\
         <div role='tabpanel' aria-label='Hydration' inert>D</div>",
    );
    // A graphic with nothing exposed inside it is one picture (`bl-07bd`).
    assert_eq!(names(&v, "img"), vec!["Real"]);
    assert_eq!(names(&v, "tabpanel"), vec!["Signals"]);
}

#[test]
fn text_and_dom_views_are_unaffected_by_ax_exclusion() {
    // AX inclusion is not visibility: an excluded subtree still renders.
    let html = "<div aria-hidden='true'><p>Alpha</p></div><div inert><p>Beta</p></div>";
    let doc = Document::parse(html);
    let out = text(&doc, None);
    assert!(
        out.contains("Alpha"),
        "text lost aria-hidden content: {out}"
    );
    assert!(out.contains("Beta"), "text lost inert content: {out}");
    assert!(dom_json(&doc).to_string().contains("aria-hidden"));
}

/// `bl-240d`, Chrome 158: a menu list's popup keeps the options the cascade
/// hides; a list box drops them, and so does a hidden select.
#[test]
fn a_menu_list_keeps_its_css_hidden_options() {
    let options = "<option hidden>a</option><option style='display:none'>b</option>\
        <option style='visibility:hidden'>c</option><option>d</option>";
    let count = |attrs: &str| {
        let doc = Document::parse(&format!("<select {attrs}>{options}</select>"));
        let styles = crate::css::compute(&doc);
        count_role(&ax_tree(&doc, Some(&styles)), "option")
    };
    assert_eq!(count(""), 4);
    assert_eq!(count("size=3"), 1);
    assert_eq!(count("multiple"), 1);
    assert_eq!(count("style='visibility:hidden'"), 0);
    assert_eq!(count("hidden"), 0);
}
