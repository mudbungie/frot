//! ARIA role resolution.
//!
//! [`role`] returns the effective role of a node: an explicit `role=`
//! attribute if it names a known role, otherwise the element's implicit role
//! per the HTML-AAM. `None` means "no semantic role at all" — the node
//! contributes nothing to the AX tree (`<br>`, `<link>`, `<meta>`,
//! `<input type=hidden>`).
//!
//! ## Why the whole document, not just the element
//!
//! Most HTML-AAM mappings are *conditional*: `<th>` is a `rowheader` or a
//! `columnheader` depending on `scope` and its position in the row, `<footer>`
//! is `contentinfo` only outside a sectioning ancestor, `<section>` is a
//! `region` only when it is named, `<li>` is a `listitem` only inside a list.
//! A flat tag→role table cannot express any of that, so resolution takes
//! `(doc, id)` and reads the context it needs through [`context`]. There is one
//! resolver and it is the authority; nothing downstream re-derives a role.
//!
//! [`level`] extracts the integer level for heading roles (h1–h6), honoring
//! `aria-level` if present.

use crate::css::Styles;
use crate::dom::{Document, Element, NodeId, NodeKind};

mod conflict;
mod context;
mod implicit;
mod marked;
mod svg;

pub use svg::titled as svg_titled;

/// Effective ARIA role for the node `id`, or `None` when it carries no
/// semantics (a non-element node, or an element the HTML-AAM maps to nothing).
pub fn role(doc: &Document, id: NodeId) -> Option<&'static str> {
    role_under(doc, id, None)
}

/// [`role`] as the accessibility tree asks it: with the cascade, where there
/// is one. Roles are CSS-independent with one measured exception — whether an
/// `<svg>` is a picture or a document turns on what it *shows* ([`svg`]) —
/// so every other caller asks [`role`] and gets the markup's answer.
pub fn role_under(doc: &Document, id: NodeId, styles: Option<&Styles>) -> Option<&'static str> {
    let NodeKind::Element(el) = &doc.node(id).kind else {
        return None;
    };
    explicit(doc, id, el).or_else(|| implicit::role(doc, id, el, styles))
}

/// An author `role=` attribute — else the role an element's internals set
/// (`Document::aria`, js.md §15.6) — resolved to its first **usable** token; with
/// none, the implicit role stands. Two things make a token unusable, both
/// measured in Chrome 158 (`bl-c5a6`):
///
/// - it is not a role (`role="foo button"` is a `button`);
/// - it is a [`NAMED_ONLY`] landmark on an element the author did not name.
///   `<div role=region>` is generic, `<div role="form button">` is a `button`,
///   and `<nav role=region>` is still a `navigation`. With `aria-label`,
///   `aria-labelledby` or a `title` attribute — even an empty one
///   ([`context::claims_landmark`]) — the landmark stands. The element `<form>`
///   needs no name — that is its implicit role, not this rule.
///
/// A usable `none`/`presentation` can still be overruled ([`conflict`]), and
/// the deprecated `directory` is a `list` (Chrome 158, `bl-d61c`).
pub fn explicit(doc: &Document, id: NodeId, el: &Element) -> Option<&'static str> {
    let role = doc
        .aria(id, el, "role")?
        .split_whitespace()
        .filter_map(|token| KNOWN_ROLES.iter().find(|known| **known == token).copied())
        .find(|role| !NAMED_ONLY.contains(role) || context::claims_landmark(doc, el))?;
    match role {
        "none" | "presentation" if conflict::overruled(doc, id, el) => None,
        "directory" => Some("list"),
        _ => Some(role),
    }
}

/// Landmark roles an author may claim only for a named element.
const NAMED_ONLY: &[&str] = &["form", "region"];

/// Heading level. Returns the `aria-level` value first if present and
/// parseable, otherwise 1..=6 for h1..h6, otherwise `None`.
pub fn level(doc: &Document, id: NodeId, el: &Element) -> Option<u32> {
    if let Some(s) = doc.aria(id, el, "aria-level") {
        if let Ok(n) = s.parse::<u32>() {
            if n >= 1 {
                return Some(n);
            }
        }
    }
    match el.name.as_str() {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
        _ => None,
    }
}

/// Whether `id` is a part of a `grid` or `treegrid` rather than of a plain
/// `table` ([`context::in_grid`]).
pub fn in_grid(doc: &Document, id: NodeId) -> bool {
    context::in_grid(doc, id)
}

/// Every role an author `role=` token may claim. The `doc-*` rows are the
/// DPUB-ARIA 1.1 module, all 41 of which Chrome 158 exposes under their own
/// names (`bl-fc83`).
const KNOWN_ROLES: &[&str] = &[
    "alert",
    "alertdialog",
    "application",
    "article",
    "banner",
    "blockquote",
    "button",
    "caption",
    "cell",
    "checkbox",
    "code",
    "columnheader",
    "combobox",
    "complementary",
    "contentinfo",
    "definition",
    "deletion",
    "dialog",
    "directory",
    "doc-abstract",
    "doc-acknowledgments",
    "doc-afterword",
    "doc-appendix",
    "doc-backlink",
    "doc-biblioentry",
    "doc-bibliography",
    "doc-biblioref",
    "doc-chapter",
    "doc-colophon",
    "doc-conclusion",
    "doc-cover",
    "doc-credit",
    "doc-credits",
    "doc-dedication",
    "doc-endnote",
    "doc-endnotes",
    "doc-epigraph",
    "doc-epilogue",
    "doc-errata",
    "doc-example",
    "doc-footnote",
    "doc-foreword",
    "doc-glossary",
    "doc-glossref",
    "doc-index",
    "doc-introduction",
    "doc-noteref",
    "doc-notice",
    "doc-pagebreak",
    "doc-pagefooter",
    "doc-pageheader",
    "doc-pagelist",
    "doc-part",
    "doc-preface",
    "doc-prologue",
    "doc-pullquote",
    "doc-qna",
    "doc-subtitle",
    "doc-tip",
    "doc-toc",
    "document",
    "emphasis",
    "feed",
    "figure",
    "form",
    "generic",
    "graphics-document",
    "grid",
    "gridcell",
    "group",
    "heading",
    "img",
    "insertion",
    "link",
    "list",
    "listbox",
    "listitem",
    "log",
    "main",
    "mark",
    "marquee",
    "math",
    "menu",
    "menubar",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "meter",
    "navigation",
    "none",
    "note",
    "option",
    "paragraph",
    "presentation",
    "progressbar",
    "radio",
    "radiogroup",
    "region",
    "row",
    "rowgroup",
    "rowheader",
    "scrollbar",
    "search",
    "searchbox",
    "sectionfooter",
    "sectionheader",
    "separator",
    "slider",
    "spinbutton",
    "status",
    "strong",
    "subscript",
    "superscript",
    "switch",
    "tab",
    "table",
    "tablist",
    "tabpanel",
    "term",
    "textbox",
    "time",
    "timer",
    "toolbar",
    "tooltip",
    "tree",
    "treegrid",
    "treeitem",
];

#[cfg(test)]
mod tests;
