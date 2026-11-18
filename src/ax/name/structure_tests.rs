//! Names of structural roles — tables and their parts, description lists —
//! as the fidelity oracle (`docs/design/fidelity.md`) recorded them.
//!
//! Every case is a Chrome 156 measurement (`Accessibility.getFullAXTree`,
//! `--headless=new`, 2026-11-18): the first of each group is in the oracle's
//! corpus, the rest were measured for the ball that fixed the group.

use super::*;

fn name(html: &str, tag: &str) -> Option<String> {
    let d = Document::parse(html);
    accessible_name(&d, *d.find_by_tag(tag).first().unwrap(), None)
}

fn name_css(html: &str, tag: &str) -> Option<String> {
    let d = Document::parse(html);
    let s = crate::css::compute(&d);
    accessible_name(&d, *d.find_by_tag(tag).first().unwrap(), Some(&s))
}

/// `bl-fd85`: a `<table>` is named by its first `<caption>` child.
#[test]
fn a_table_is_named_from_its_caption() {
    for (html, want) in [
        // The corpus case (`structure.html`).
        (
            "<table><caption>Quarterly totals</caption><tr><th>Q</th></tr></table>",
            Some("Quarterly totals"),
        ),
        // Wherever among the children the caption sits.
        (
            "<table><tr><th>x</th></tr><caption>late</caption></table>",
            Some("late"),
        ),
        // The first of two.
        (
            "<table><caption>first</caption><caption>second</caption><tr><th>x</th></tr></table>",
            Some("first"),
        ),
        // The caption's text alternatives, not its raw text.
        (
            "<table><caption><img alt=IMG> tail</caption><tr><th>x</th></tr></table>",
            Some("IMG tail"),
        ),
        // The caption's own `aria-label` speaks for it …
        (
            "<table><caption aria-label=CAL>text</caption><tr><th>x</th></tr></table>",
            Some("CAL"),
        ),
        // … but its `title` does not: a caption is read as a descendant.
        (
            "<table><caption title=CT></caption><tr><th>x</th></tr></table>",
            None,
        ),
        // An excluded caption names nothing.
        (
            "<table><caption aria-hidden=true>ah</caption><tr><th>x</th></tr></table>",
            None,
        ),
        // No caption, no name; an empty one likewise.
        ("<table><tr><th>x</th></tr></table>", None),
        (
            "<table><caption></caption><tr><th>x</th></tr></table>",
            None,
        ),
        // The author's name outranks the caption; the caption outranks `title`;
        // a reference that resolves to nothing falls through to it.
        (
            "<table aria-label=AL><caption>cap</caption><tr><th>x</th></tr></table>",
            Some("AL"),
        ),
        (
            "<table title=TI><caption>cap</caption><tr><th>x</th></tr></table>",
            Some("cap"),
        ),
        (
            "<table aria-labelledby=nope><caption>cap</caption><tr><th>x</th></tr></table>",
            Some("cap"),
        ),
        // An author role on the table does not change where its name is.
        (
            "<table role=grid><caption>gridcap</caption><tr><th>x</th></tr></table>",
            Some("gridcap"),
        ),
    ] {
        assert_eq!(name(html, "table").as_deref(), want, "{html}");
    }
}

/// A caption CSS hides names nothing — `display:none` and `visibility:hidden`
/// alike (Chrome 156: `table ""` for both).
#[test]
fn a_hidden_caption_names_nothing() {
    for style in ["display:none", "visibility:hidden"] {
        let html =
            format!("<table><caption style='{style}'>hid</caption><tr><th>x</th></tr></table>");
        assert_eq!(name_css(&html, "table"), None, "{style}");
    }
}
