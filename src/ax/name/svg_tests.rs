//! An inline `<svg>` and its `<title>` (`bl-0a0a`).
//!
//! Every case is a Chrome 156 measurement (`Accessibility.getFullAXTree`,
//! `--headless=new`, 2026-11-18); the first is the fidelity oracle's corpus
//! case (`svg.html`), the rest were measured for the fix.

use super::*;

fn name(html: &str, tag: &str) -> Option<String> {
    let d = Document::parse(html);
    accessible_name(&d, *d.find_by_tag(tag).first().unwrap(), None)
}

#[test]
fn an_svg_is_named_from_its_title_child() {
    for (html, want) in [
        (
            "<svg><title>Chart title</title><text x=10 y=30>svg label words</text></svg>",
            Some("Chart title"),
        ),
        // Wherever among the children it sits, and the first of two.
        (
            "<svg><rect/><title>notfirst</title></svg>",
            Some("notfirst"),
        ),
        (
            "<svg><title>one</title><title>two</title></svg>",
            Some("one"),
        ),
        // Its text content, whitespace collapsed.
        ("<svg><title>t<tspan>in</tspan></title></svg>", Some("tin")),
        (
            "<svg><title>  spaced   out </title></svg>",
            Some("spaced out"),
        ),
        // Only a child: a `<title>` under a `<g>` names the `<g>`.
        ("<svg><g><title>nested</title><rect/></g></svg>", None),
        ("<svg><svg><title>inner</title></svg></svg>", None),
        // No title, or an empty one: no name — the text is not the name.
        ("<svg><text>only text</text></svg>", None),
        ("<svg><title></title><rect/></svg>", None),
        // The author's name outranks it; it outranks the `title` attribute.
        ("<svg aria-label=AL><title>losing</title></svg>", Some("AL")),
        ("<svg title=attr><title>both</title></svg>", Some("both")),
        ("<svg title=attr><rect/></svg>", Some("attr")),
    ] {
        assert_eq!(name(html, "svg").as_deref(), want, "{html}");
    }
}

/// What the graphic contributes to an ancestor's name is the same fact.
#[test]
fn an_svg_contributes_its_title_child_or_else_its_contents() {
    for (html, want) in [
        (
            "<a href=2><svg><title>T</title><text>words</text></svg></a>",
            Some("T"),
        ),
        // Chrome: "emptytitle" — an empty title is not the empty alternative.
        (
            "<a href=4><svg><title></title><text>emptytitle</text></svg></a>",
            Some("emptytitle"),
        ),
        (
            "<a href=3><svg><text>justtext</text></svg></a>",
            Some("justtext"),
        ),
        // Chrome: "deep" — reached here by descending to the title's text.
        (
            "<a href=1><svg><g><title>deep</title></g></svg></a>",
            Some("deep"),
        ),
    ] {
        assert_eq!(name(html, "a").as_deref(), want, "{html}");
    }
}
