//! A closed `<details>` conceals its disclosure content (`bl-74a6`).
//!
//! Field repro 2026-08-11 (GOV.UK Design System `/components/details/`): both
//! closed disclosures emitted their whole body under `--css`, and `bboxes`
//! emitted the concealed paragraphs as zero-sized entries. Chrome reports
//! `details.open === false` and gives every child after the summary no client
//! rect at all — no interaction, no synthetic click, just the initial state.
//!
//! Unlike the UA attribute rules in `computed.rs`, this one is **not**
//! author-overridable: the box the UA skips is the `::details-content` box,
//! which belongs to the `<details>`, not to the child.

use crate::ax::accessible_name;
use crate::css::{compute, Display, Styles, Visibility};
use crate::dom::{Document, NodeId};

fn styles(html: &str) -> (Document, Styles) {
    let doc = Document::parse(html);
    let s = compute(&doc);
    (doc, s)
}

fn first_tag(doc: &Document, tag: &str) -> NodeId {
    *doc.find_by_tag(tag).first().expect("tag present")
}

/// The `<p>` a `<details>` wraps, and whether the cascade renders it.
fn body_rendered(html: &str) -> bool {
    let (doc, s) = styles(html);
    !s.display_none(first_tag(&doc, "p"))
}

#[test]
fn a_closed_details_renders_only_its_summary() {
    let (doc, s) = styles("<details><summary>Options</summary><p>Use options…</p></details>");
    // The disclosure itself and its control render; the body does not.
    assert_ne!(s.display(first_tag(&doc, "details")), Display::None);
    assert!(!s.display_none(first_tag(&doc, "summary")));
    assert!(s.display_none(first_tag(&doc, "p")));
}

#[test]
fn open_toggles_the_disclosure_content_back_on() {
    // The one fact the whole rule turns on. `--js` mutates the live arena and
    // the cascade runs after it (`run.rs`), so a script that sets `open` is
    // read here exactly as an author-written attribute is.
    let closed = "<details><summary>S</summary><p>body</p></details>";
    assert!(!body_rendered(closed));
    let mut doc = Document::parse(closed);
    let d = first_tag(&doc, "details");
    doc.set_attr(d, "open", "");
    assert!(!compute(&doc).display_none(first_tag(&doc, "p")));
    // …and back off again.
    doc.remove_attr(d, "open");
    assert!(compute(&doc).display_none(first_tag(&doc, "p")));
}

#[test]
fn author_css_cannot_reveal_concealed_content() {
    // `[hidden]` is a UA *declaration* an author outranks; this is a skipped
    // box the author has no handle on. Neither a sheet rule, an `!important`
    // one, nor inline `style=` puts the content back.
    for html in [
        "<style>details p{display:block}</style><details><summary>S</summary><p>b</p></details>",
        "<style>p{display:block!important}</style><details><summary>S</summary><p>b</p></details>",
        "<details><summary>S</summary><p style='display:block'>b</p></details>",
    ] {
        assert!(!body_rendered(html), "{html}");
    }
    // The `<details>` itself is an ordinary element: author CSS still reaches it.
    let (doc, s) =
        styles("<style>details{visibility:hidden}</style><details><summary>S</summary></details>");
    assert_eq!(s.visibility(first_tag(&doc, "summary")), Visibility::Hidden);
}

#[test]
fn nested_details_conceal_independently() {
    // The outer is open, so its content cascades normally — and the inner,
    // closed, still conceals its own.
    let (doc, s) = styles(
        "<details open><summary>outer</summary>\
         <details><summary>inner</summary><p>deep</p></details></details>",
    );
    assert!(!s.display_none(doc.find_by_tag("summary")[1]));
    assert!(s.display_none(first_tag(&doc, "p")));
    // Closing the outer takes the whole inner disclosure with it: consumers
    // prune at the concealed node, so the inner `<details>` is where they stop.
    let (doc, s) = styles(
        "<details><summary>outer</summary>\
         <details open><summary>inner</summary><p>deep</p></details></details>",
    );
    assert!(s.display_none(doc.find_by_tag("details")[1]));
}

#[test]
fn concealed_text_nodes_are_display_none_too() {
    // Raw text is the common shape (`<details><summary>Q</summary>A</details>`)
    // and has no element of its own to carry the rule, so the cascade marks
    // the text node itself — `Styles` is parallel to the whole arena.
    let (doc, s) = styles("<details><summary>Q</summary>An answer.</details>");
    let d = first_tag(&doc, "details");
    let text_node = *doc.node(d).children.last().unwrap();
    assert!(s.display_none(text_node));
    // The accessible-name walk reads the same table, so the concealed text is
    // not part of a name computed over the disclosure either: the link is named
    // by the summary alone.
    let (doc, s) = styles("<a href=x><details><summary>Q</summary>An answer.</details></a>");
    let a = first_tag(&doc, "a");
    assert_eq!(accessible_name(&doc, a, Some(&s)).as_deref(), Some("Q"));
    // And the cascade is not what holds the answer back: the name walk descends
    // through `Document::ax_children`, which applies the same concealment with
    // no `Styles` table at all, so the answer is the same in either recipe
    // (`bl-0aaf`). Chrome 139 agrees — a link wrapping a closed `<details>` is
    // named from its summary, and the disclosure body has no AX node.
    assert_eq!(accessible_name(&doc, a, None).as_deref(), Some("Q"));
}

#[test]
fn a_summary_outside_details_is_untouched() {
    // The rule keys on the parent, not on the tag.
    let (doc, s) = styles("<div><summary>S</summary><p>b</p></div>");
    assert!(!s.display_none(first_tag(&doc, "summary")));
    assert!(!s.display_none(first_tag(&doc, "p")));
}
