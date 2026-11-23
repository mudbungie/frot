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

/// `<section>`: the landmark when named, `generic` when not — an unnamed
/// landmark is noise a screen reader cannot navigate to.
pub fn named_landmark(doc: &Document, el: &Element, landmark: &'static str) -> &'static str {
    if claims_landmark(doc, el) {
        landmark
    } else {
        "generic"
    }
}

/// Whether the author named `el` as far as a landmark role asks: an author
/// name, or a `title` attribute at all. Measured in Chrome 158 (`bl-dfc3`):
/// `<div role=form title="">`, `<div role=region title="">`,
/// `<section title="">` and a nested `<aside title="">` keep their landmarks
/// with the name "", while `aria-label=""` and an `aria-labelledby` that
/// resolves to nothing do not.
pub fn claims_landmark(doc: &Document, el: &Element) -> bool {
    author_name(doc, el).is_some() || el.attr("title").is_some()
}

/// `<tbody>`: a `rowgroup` only when the author named it — unnamed, it groups
/// nothing a `<table>` did not. An empty `title` is no name here.
pub fn named_rowgroup(doc: &Document, el: &Element) -> &'static str {
    if author_name(doc, el).is_some() {
        "rowgroup"
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

/// `<summary>`: the disclosure `button` only as a child of a `<details>`.
/// Measured in Chrome 158 (`bl-3fc9`): every `<summary>` child of an open
/// `<details>` is one, not only the first; a `<summary>` deeper inside, or
/// with no `<details>` at all, has no node.
pub fn summary(doc: &Document, id: NodeId) -> &'static str {
    match doc.node(id).parent.and_then(|p| tag(doc, p)) {
        Some("details") => "button",
        _ => "generic",
    }
}

/// `<li>`: a list item only inside a `<ul>`, `<ol>` or `<menu>` whose `role`
/// attribute, if it has one, says exactly `list`, `directory` or nothing.
/// Measured in Chrome 158 (`bl-89a6`): the attribute decides, not the list's
/// role — under `role=foo`, `role=" "`, `role=LIST`, `role="foo list"` or a
/// `role=none` that a label overrules, the list stands and its items have no
/// node; under `role=""` they are list items.
pub fn listitem_or_generic(doc: &Document, id: NodeId) -> &'static str {
    let list = doc.node(id).parent.and_then(|p| match &doc.node(p).kind {
        NodeKind::Element(e) if matches!(e.name.as_str(), "ul" | "ol" | "menu") => Some(e),
        _ => None,
    });
    match list.map(|e| e.attr("role")) {
        Some(None | Some("" | "list" | "directory")) => "listitem",
        _ => "generic",
    }
}

/// A part of a `<table>` — a row group, a row, a cell — has `role` only while
/// its table, the nearest `<table>` ancestor, still is a table, grid or
/// treegrid, else `generic`. Measured in Chrome 158 (`bl-2ea9`): under
/// `<table role=presentation>` — and equally `role=none`, `group` or `list` —
/// no `<thead>`, `<tr>`, `<td>` or `<th>` has a node, named or not; under a
/// `role=presentation` a label overrules they do (`bl-89a6`), unlike the
/// items of a list ([`listitem_or_generic`]). An author role on the part
/// itself is not this rule's (`<tr role=row>` stays a row), a nested
/// `<table>` starts over, and a `<caption>` keeps its node.
pub fn table_part(doc: &Document, id: NodeId, role: &'static str) -> &'static str {
    let table = ancestors(doc, id).find(|&a| tag(doc, a) == Some("table"));
    match table.and_then(|t| super::role(doc, t)) {
        Some("table" | "grid" | "treegrid") => role,
        _ => "generic",
    }
}

/// `<td>`: `cell` in a table, `gridcell` in a grid — the table's role decides,
/// so an author `role=grid` reaches its cells.
pub fn cell_role(
    doc: &Document,
    id: NodeId,
    plain: &'static str,
    grid: &'static str,
) -> &'static str {
    if in_grid(doc, id) {
        grid
    } else {
        plain
    }
}

/// Whether the table `id` belongs to — its nearest ancestor with a table role
/// — is an interactive one, a `grid` or `treegrid`. The one fact a table's
/// role hands down to its parts: the role of a cell, and whether a row is
/// named ([`crate::ax::name`]).
pub fn in_grid(doc: &Document, id: NodeId) -> bool {
    let table = ancestors(doc, id)
        .filter_map(|a| super::role(doc, a))
        .find(|r| matches!(*r, "table" | "grid" | "treegrid"));
    matches!(table, Some("grid" | "treegrid"))
}

/// Whether a scope-less `<th>` heads its *row* — Chromium's reading of its
/// element siblings, measured in Chrome 158 (`bl-7f3a`). A *full cell* is a
/// `<td>` with any child node at all (a space or a comment counts):
///
/// 1. alone in its row, or between two `<th>`: a column header;
/// 2. a full cell beside it: a row header;
/// 3. a full cell first, last, second or second-to-last in the row (tables
///    often leave the corner cell empty): a row header;
/// 4. otherwise a column header.
///
/// So `<th>x</th><td></td>` heads a column and `<td>v</td><th></th>` a row.
pub fn heads_its_row(doc: &Document, id: NodeId) -> bool {
    let row: Vec<NodeId> = doc
        .node(id)
        .parent
        .into_iter()
        .flat_map(|p| doc.node(p).children.iter().copied())
        .filter(|&c| tag(doc, c).is_some())
        .collect();
    let at = row.iter().position(|&c| c == id).unwrap_or_default();
    let cell = |i: Option<usize>| i.and_then(|i| row.get(i).copied());
    let (prev, next) = (cell(at.checked_sub(1)), cell(Some(at + 1)));
    let th = |c: Option<NodeId>| c.is_some_and(|c| tag(doc, c) == Some("th"));
    let full = |c: Option<NodeId>| {
        c.is_some_and(|c| tag(doc, c) == Some("td") && !doc.node(c).children.is_empty())
    };
    let n = row.len();
    if (prev.is_none() && next.is_none()) || (th(prev) && th(next)) {
        return false;
    }
    [
        prev,
        next,
        cell(Some(0)),
        cell(n.checked_sub(1)),
        cell(Some(1)),
        cell(n.checked_sub(2)),
    ]
    .into_iter()
    .any(full)
}

pub fn has_ancestor(doc: &Document, id: NodeId, tags: &[&str]) -> bool {
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
