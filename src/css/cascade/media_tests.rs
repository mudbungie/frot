//! `<video>`/`<audio>` fallback content is never rendered (`bl-0f83`).
//!
//! Field repro 2026-08-11, `https://www.w3.org/` under `--css --out text`:
//! frot emitted "Sorry, your browser does not support embedded videos" and the
//! AX tree exposed it too. Chrome at 1280×720 paints the `<video>` as a
//! 500×281 replaced box and exposes neither the text nor its AX node —
//! unchanged with JavaScript disabled, because fallback is for a UA that does
//! not *implement* the element, not for one whose playback is idle.
//!
//! Same seam as the closed `<details>` beside it (`details_tests.rs`): one
//! structural fact in `dom::Document::concealed`, folded into `display` after
//! the cascade, so it is not author-overridable — the box belongs to the
//! `<video>`, not to the fallback.

use crate::css::{compute, rendered_subtree_text, Display, Styles};
use crate::dom::{Document, NodeId};

fn styles(html: &str) -> (Document, Styles) {
    let doc = Document::parse(html);
    let s = compute(&doc);
    (doc, s)
}

fn first_tag(doc: &Document, tag: &str) -> NodeId {
    *doc.find_by_tag(tag).first().expect("tag present")
}

const FALLBACK: &str = "<video controls poster='p.jpg' aria-label='A demo'>\
     <source src='a.mp4' type='video/mp4'><track kind=captions src='c.vtt'>\
     <p>Sorry, your browser does not support embedded videos</p></video>";

#[test]
fn the_media_box_renders_and_nothing_inside_it_does() {
    let (doc, s) = styles(FALLBACK);
    assert_ne!(s.display(first_tag(&doc, "video")), Display::None);
    for tag in ["source", "track", "p"] {
        assert!(s.display_none(first_tag(&doc, tag)), "{tag}");
    }
    // `<audio>` is the same element class, so the same fact.
    let (doc, s) = styles("<audio><p>no audio</p></audio>");
    assert!(s.display_none(first_tag(&doc, "p")));
}

#[test]
fn author_css_cannot_reveal_the_fallback() {
    // Neither a sheet rule, an `!important` one, nor inline `style=` gives the
    // fallback a box: a supporting UA generates none for it at all.
    for html in [
        "<style>video p{display:block}</style><video><p>b</p></video>",
        "<style>p{display:block!important}</style><video><p>b</p></video>",
        "<video><p style='display:block'>b</p></video>",
    ] {
        let (doc, s) = styles(html);
        assert!(s.display_none(first_tag(&doc, "p")), "{html}");
    }
    // The `<video>` itself is an ordinary element: author CSS still reaches it.
    let (doc, s) = styles("<style>video{display:none}</style><video></video>");
    assert!(s.display_none(first_tag(&doc, "video")));
}

#[test]
fn raw_fallback_text_is_concealed_with_the_elements() {
    // The w3.org shape: bare prose, no element of its own to carry the rule.
    let (doc, s) = styles("<video>Sorry, your browser does not support embedded videos</video>");
    let v = first_tag(&doc, "video");
    let text_node = *doc.node(v).children.last().unwrap();
    assert!(s.display_none(text_node));
    // The accessible-name walk reads the same table, so the fallback is not
    // part of any name computed from the media element's contents either.
    assert_eq!(rendered_subtree_text(&doc, v, &s), "");
}

#[test]
fn media_outside_a_media_element_is_untouched() {
    // The rule keys on the parent. A `<source>` in a `<picture>` is not a
    // media-element child, and `<picture>` is not in scope here.
    let (doc, s) = styles("<picture><source srcset=a.webp><img alt=a></picture>");
    assert!(!s.display_none(first_tag(&doc, "source")));
    assert!(!s.display_none(first_tag(&doc, "img")));
}
