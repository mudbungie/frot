//! Which roles are named from their contents — the gate on the §2F recursion
//! as a node's *own* native name ([`super::contents`]).

use crate::dom::{Document, NodeId};

/// Roles whose accessible name is computed from descendant text — the
/// "Name From: contents" set of WAI-ARIA 1.2 (roles listed with
/// `nameFrom: contents` in §5.4 Definition of Roles). Only these take the
/// subtree-text fallback (`row` too, conditionally: [`names_from_contents`]);
/// container roles (`table`, `rowgroup`, `list`,
/// `listitem`, `paragraph`, `navigation`, `region`, `group`, …) are absent,
/// so they yield no native name instead of echoing their whole subtree.
///
/// `term` is the one entry ARIA does not list (it is name-from-author there):
/// Chrome 156 names a `<dt>`, a `<dfn>` and a `role=term` from their contents
/// (`bl-8ad7`), and the browser is the oracle. `definition` stays out — Chrome
/// names a `<dd>` nothing. The five `doc-*` rows are the DPUB-ARIA roles
/// Chrome 158 names from contents (`bl-fc83`): the three link kinds, the
/// back-link, and the subtitle; the other 36 are sections and name nothing.
const NAME_FROM_CONTENTS: &[&str] = &[
    "button",
    "cell",
    "checkbox",
    "columnheader",
    "doc-backlink",
    "doc-biblioref",
    "doc-glossref",
    "doc-noteref",
    "doc-subtitle",
    "gridcell",
    "heading",
    "link",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "option",
    "radio",
    "rowheader",
    "switch",
    "tab",
    "term",
    "tooltip",
    "treeitem",
];

/// Whether the node's effective role is in [`NAME_FROM_CONTENTS`] — and, for
/// a `row`, whether its table is a `grid`/`treegrid`. ARIA lists `row` as
/// name-from-contents without condition, but a browser computes that name only
/// where a row can be the thing in focus: measured in Chrome 156 (`bl-3072`),
/// a row of a `table` is `""` — `<tr>` or `role=row`, focusable or not — while
/// the same row in a grid is named from its cells.
pub fn names_from_contents(doc: &Document, id: NodeId) -> bool {
    match crate::ax::role(doc, id) {
        Some("row") => crate::ax::role::in_grid(doc, id),
        Some(r) => NAME_FROM_CONTENTS.contains(&r),
        None => false,
    }
}
