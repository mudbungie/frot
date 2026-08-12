//! The document context conditional HTML-AAM mappings need: ancestors,
//! siblings, and the author-supplied name.
//!
//! Every predicate here is a pure function of the DOM. Roles are deliberately
//! **CSS-independent** — nothing in this module consults a `Styles` table, so
//! `--css` never changes a role, only which nodes survive to be roled.

use crate::ax::name;
use crate::dom::{Document, Element, NodeId, NodeKind};

/// Sectioning content plus `<main>`: the ancestors that scope an `<aside>`
/// (HTML-AAM) — inside one, an aside is a landmark only when named.
const SECTIONING_CONTENT: &[&str] = &["article", "aside", "main", "nav", "section"];

/// The HTML *sectioning roots*. `<header>`/`<footer>` are page-level only when
/// their nearest sectioning ancestor is `<body>` itself, and the HTML sectioning
/// algorithm counts these roots alongside sectioning content — which is why a
/// `<footer>` inside a `<blockquote>` is a `sectionfooter`, not the page's
/// `contentinfo`.
const SECTIONING_ROOTS: &[&str] = &[
    "blockquote",
    "details",
    "dialog",
    "fieldset",
    "figure",
    "td",
];

/// `<header>`/`<footer>`: the page-level landmark outside any sectioning
/// ancestor, the section-level role inside one.
pub fn header_footer(
    doc: &Document,
    id: NodeId,
    page: &'static str,
    section: &'static str,
) -> &'static str {
    if has_ancestor(doc, id, SECTIONING_CONTENT) || has_ancestor(doc, id, SECTIONING_ROOTS) {
        section
    } else {
        page
    }
}

/// `<aside>`: a top-level aside is always `complementary`; a nested one has to
/// earn its landmark with a name.
pub fn scoped_aside(doc: &Document, id: NodeId, el: &Element) -> &'static str {
    if has_ancestor(doc, id, SECTIONING_CONTENT) {
        named_landmark(doc, el, "complementary")
    } else {
        "complementary"
    }
}

/// `<section>`/`<form>`: the landmark when named, `generic` when not — an
/// unnamed landmark is noise a screen reader cannot navigate to.
pub fn named_landmark(doc: &Document, el: &Element, landmark: &'static str) -> &'static str {
    if author_name(doc, el).is_some() {
        landmark
    } else {
        "generic"
    }
}

/// The author-supplied accessible name, CSS-independent by construction (the
/// `None` styles argument). For every element that asks, the accname
/// algorithm's native step contributes nothing — a `<section>` does not name
/// from its contents — so this *is* the accessible name.
pub fn author_name(doc: &Document, el: &Element) -> Option<String> {
    name::author_name(doc, el, None)
}

/// `<li>`: a list item only inside a list.
pub fn listitem_or_generic(doc: &Document, id: NodeId) -> &'static str {
    match doc.node(id).parent.and_then(|p| tag(doc, p)) {
        Some("ul" | "ol" | "menu") => "listitem",
        _ => "generic",
    }
}

/// `<td>`: `cell` in a table, `gridcell` in a grid — the ancestor table's role
/// decides, so an author `role=grid` reaches its cells.
pub fn cell_role(
    doc: &Document,
    id: NodeId,
    plain: &'static str,
    grid: &'static str,
) -> &'static str {
    let table = ancestors(doc, id).find(|&a| tag(doc, a) == Some("table"));
    match table.and_then(|a| super::role(doc, a)) {
        Some("grid" | "treegrid") => grid,
        _ => plain,
    }
}

/// Whether a scope-less `<th>` heads its *row*: it opens the row and at least
/// one data cell follows it. A row of nothing but `<th>` is a header row, so
/// its cells head columns.
pub fn heads_its_row(doc: &Document, id: NodeId) -> bool {
    let cells: Vec<NodeId> = doc
        .node(id)
        .parent
        .into_iter()
        .flat_map(|p| doc.node(p).children.iter().copied())
        .filter(|&c| matches!(tag(doc, c), Some("td" | "th")))
        .collect();
    cells.first() == Some(&id) && cells.iter().any(|&c| tag(doc, c) == Some("td"))
}

fn has_ancestor(doc: &Document, id: NodeId, tags: &[&str]) -> bool {
    ancestors(doc, id).any(|a| tag(doc, a).is_some_and(|t| tags.contains(&t)))
}

fn ancestors(doc: &Document, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
    std::iter::successors(doc.node(id).parent, move |&a| doc.node(a).parent)
}

fn tag(doc: &Document, id: NodeId) -> Option<&str> {
    match &doc.node(id).kind {
        NodeKind::Element(el) => Some(el.name.as_str()),
        _ => None,
    }
}
