//! Hidden fallback copy is not content (`bl-eeb4`).
//!
//! Field repro 2026-08-11: a deploy console and a task app both returned
//! `status:"ok"` on a post-JS empty mount because a fallback subtree the
//! browser never paints — one `hidden`, one hidden by a class — carried text
//! the starvation walk counted. The visibility oracle differs by recipe: the
//! cascade under `--css`, the `hidden` attribute alone without it.

use super::*;
use crate::css::compute;

const HIDDEN_SHELL: &str = include_str!("../../tests/fixtures/needs/hidden-fallback-shell.html");

fn styles_of(doc: &Document) -> crate::css::Styles {
    compute(doc)
}

#[test]
fn hidden_fallback_text_does_not_mask_an_empty_mount() {
    let doc = Document::parse(HIDDEN_SHELL);
    for view in [View::Text, View::Ax, View::Links, View::Forms] {
        assert_eq!(detect(view, &doc, None), vec![NeedsKind::Js], "{view:?}");
    }
}

#[test]
fn the_cascade_reaches_the_same_verdict_under_css() {
    // Under `--css` the UA `[hidden] { display: none }` rule is already in the
    // cascade, so the styles path needs no attribute check of its own.
    let doc = Document::parse(HIDDEN_SHELL);
    let s = styles_of(&doc);
    assert_eq!(
        detect(View::Text, &doc, Some(&s)),
        vec![NeedsKind::Js],
        "styles"
    );
}

#[test]
fn author_css_that_unhides_the_fallback_makes_it_content() {
    // `[hidden]` is a UA rule: an author `display` wins, and then the text
    // really is painted — so the page is not starved.
    let doc = Document::parse(
        "<html><style>[hidden]{display:block}</style><body><div id=root></div>\
         <div hidden><span>Opens in a new tab</span></div>\
         <script src=app.js></script></body></html>",
    );
    let s = styles_of(&doc);
    assert!(detect(View::Text, &doc, Some(&s)).is_empty());
    // Without `--css` there is no author sheet to consult: the attribute stands.
    assert_eq!(detect(View::Text, &doc, None), vec![NeedsKind::Js]);
}

#[test]
fn css_hidden_fallback_text_does_not_mask_an_empty_mount() {
    // The second field repro: the fallback is hidden by an author class, not
    // the attribute, so only the cascade can see through it.
    let doc = Document::parse(
        "<html><style>.fallback{display:none}</style><body><div id=root></div>\
         <div class=fallback>could not load the required files</div>\
         <script src=app.js></script></body></html>",
    );
    let s = styles_of(&doc);
    assert_eq!(detect(View::Text, &doc, Some(&s)), vec![NeedsKind::Js]);
    // Without `--css` that text is painted (nothing hides it) and the page
    // honestly carries content.
    assert!(detect(View::Text, &doc, None).is_empty());
}

#[test]
fn a_rendered_app_inside_a_section_is_not_chrome() {
    // A `<header>` scoped by sectioning content is that section's heading, not
    // the page banner: the app rendered, so no view is starved. (Live TodoMVC
    // renders exactly this, with its `<main>`/`<footer>` `hidden` until a todo
    // exists — the shape that pinned the two rules against each other.)
    let doc = Document::parse(
        "<html><body><section id=root><header><h1>todos</h1></header>\
         <main hidden><p>0 items left</p></main></section>\
         <script src=app.js></script></body></html>",
    );
    for view in [View::Text, View::Ax, View::Links, View::Forms] {
        assert!(detect(view, &doc, None).is_empty(), "{view:?}");
    }
}

#[test]
fn page_level_header_and_footer_are_still_chrome() {
    // Unscoped, they are the `banner`/`contentinfo` landmarks: boilerplate
    // around an empty mount, exactly as before.
    let doc = Document::parse(
        "<html><body><header><h1>Sketchpad</h1></header><div id=root></div>\
         <footer><p>Created by the team</p></footer>\
         <script src=app.js></script></body></html>",
    );
    assert_eq!(detect(View::Text, &doc, None), vec![NeedsKind::Js]);
}
