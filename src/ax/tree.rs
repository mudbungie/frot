//! AX tree builder for `--out ax`.
//!
//! Walks the DOM in source order producing a flat JSON shape per node:
//! `{ "role": <str>, "name": <str|null>, "level"?: <int>, "children": [...] }`.
//!
//! Elements whose effective role is `None`, `generic`, `presentation`, or
//! `none` do not contribute a node — their AX children promote into the
//! parent's child list. Subtrees under `<script>`, `<style>`, `<template>`,
//! and `<noscript>` ([`tags::is_source`]) are skipped entirely, as are subtrees excluded by
//! `aria-hidden`/`inert` ([`crate::ax::hidden`]).
//!
//! While a modal dialog is open, only what is inside the topmost one has
//! nodes: the rest of the page is inert (`dom/live/top.rs`, `bl-2dcd`).
//!
//! A `<p>` whose box holds no line — empty, or with a block-level child —
//! contributes no node either, and nor does such a table `<caption>`
//! ([`lineless`]).
//!
//! ## Layout-table demotion
//!
//! Much HTML nests `<table>` purely for visual grid layout (Hacker News is a
//! canonical example). Real browsers run layout-table heuristics in the AX
//! layer and demote such tables to `role=presentation`, so their scaffolding
//! collapses out of the tree and only content surfaces. We apply
//! Chromium's test (`IsDataTable`, [`table`]), measured in Chrome 158
//! (`bl-dfc3`, `bl-7f3a`, `bl-e143`): a `<table>` is treated as *layout* (and its
//! table-structural descendants — `table`, `rowgroup`, `row`, `cell`,
//! `columnheader`, `rowheader`, `gridcell` — collapse to presentation) unless
//!
//! 1. it carries a `role`, a non-empty `summary` or a non-empty `rules`, or a
//!    `<caption>`, `<thead>`, `<tfoot>`, `<colgroup>` or `<col>` child, or
//!    is editable (`contenteditable` on it or above, not turned off) — any
//!    of which keeps even a one-cell table; or, short of that,
//! 2. it is more than one row holding one cell, and it has 20 rows, or a
//!    `<th>`, or a cell with a non-empty `headers`, `abbr`, `axis` or
//!    `scope`; or it has two cells and a `border` that is not zero (Chrome
//!    reads the cell borders the attribute draws: `border=""` and
//!    `border=abc` count, `border=0px` does not); or, under a cascade, a
//!    cell has `empty-cells: hide`, half its cells (rounded down) have
//!    borders or a background of their own, or the first rows' backgrounds
//!    alternate ([`table`], `css/paint.rs`).
//!
//! `aria-label` and `aria-labelledby` are no signal: Chrome names a layout
//! table and still leaves it out. Cells, rows and headers are counted without descending into
//! a nested `<table>`, which re-classifies on its own, so a data table inside
//! a layout table keeps its structure and an inner header cannot rescue the
//! outer table.
//!
//! A part the author roled — or a cell in a row the author roled — is not
//! scaffolding and keeps its node in a layout table ([`authored`]).

use crate::ax;
use crate::css::{Styles, Visibility};
use crate::dom::{Document, Element, NodeId, NodeKind};
use crate::tags;
use serde_json::{json, Value};

pub(in crate::ax) mod lineless;
mod table;

/// Children are emitted in DOM order, always: Chrome's AX tree does not follow
/// flex `order` or `*-reverse` (measured, Chrome 156; `layout.md` §5), so the
/// view needs no layout.
pub fn ax_tree(doc: &Document, styles: Option<&Styles>) -> Value {
    let mut nodes = Vec::new();
    for &root in doc.roots() {
        nodes.extend(build(doc, root, styles, false));
    }
    Value::Array(nodes)
}

fn build(doc: &Document, id: NodeId, styles: Option<&Styles>, in_layout_table: bool) -> Vec<Value> {
    let entry = doc.node(id);
    let NodeKind::Element(el) = &entry.kind else {
        return Vec::new();
    };
    if tags::is_source(&el.name) {
        return Vec::new();
    }
    // Semantic subtree cut (`aria-hidden`/`inert`, [`ax::excluded`]) — before
    // descendants are built, so inheritance and "no escape from an excluded
    // ancestor" fall out of the recursion instead of needing their own rules.
    if ax::excluded(doc, id, el) {
        return Vec::new();
    }
    if styles.is_some_and(|s| s.display_none(id)) {
        return Vec::new();
    }
    // Behind a modal dialog the page is inert (`bl-2dcd`): no node, except
    // that the dialog's own ancestors pass their children through.
    if let Some(m) = doc.top_modal().filter(|&m| !doc.inside(id, m)) {
        if !doc.inside(m, id) {
            return Vec::new();
        }
        let below = doc.ax_children(id);
        return below
            .flat_map(|c| build(doc, c, styles, in_layout_table))
            .collect();
    }
    // A `<table>` re-classifies the nearest-table context (nesting resets, so a
    // data table inside a layout table keeps its structure); other elements
    // inherit the ancestor flag.
    let layout_table = if el.name == "table" {
        table::is_layout_table(doc, id, el, styles)
    } else {
        in_layout_table
    };
    // A menu list's popup holds every option, whatever the cascade says of
    // them: measured in Chrome 158 (`bl-240d`), `<option hidden>`, one that is
    // `display:none` or `visibility:hidden` and a hidden `<optgroup>` all keep
    // their nodes under a collapsed `<select>`, and lose them in a list box.
    let hidden = styles.is_some_and(|s| s.visibility(id) == Visibility::Hidden);
    let menu_list = el.name == "select" && !el.list_box() && !hidden;
    let below = if menu_list { None } else { styles };
    let mut children = Vec::new();
    // `Document::ax_children` decides which children exist — the accessor
    // `ax::name::contents` descends through too, so nothing cut from the tree
    // can still name it (`bl-0aaf`).
    for c in doc.ax_children(id) {
        children.extend(build(doc, c, below, layout_table));
    }
    if hidden {
        // `visibility:hidden` removes this element's own node but a
        // `visibility:visible` descendant still surfaces (it built its
        // own node above), so promote the collected children.
        return children;
    }
    let role = ax::role::role_under(doc, id, styles);
    if layout_table
        && matches!(
            role,
            Some("table" | "rowgroup" | "row" | "cell" | "columnheader" | "rowheader" | "gridcell")
        )
        && !authored(doc, id, el)
    {
        // Demoted layout-table scaffolding: emit no node, promote content.
        return children;
    }
    match role {
        None | Some("generic") | Some("presentation") | Some("none") => children,
        Some("paragraph" | "caption") if lineless::ignored(doc, id, el, styles) => children,
        Some(role_name) => {
            let name = ax::accessible_name(doc, id, styles);
            let mut obj = serde_json::Map::new();
            obj.insert("role".into(), Value::String(role_name.to_string()));
            obj.insert("name".into(), name.map_or(Value::Null, Value::String));
            if let Some(level) = ax::level(doc, id, el) {
                obj.insert("level".into(), json!(level));
            }
            obj.insert("children".into(), Value::Array(children));
            vec![Value::Object(obj)]
        }
    }
}

/// Whether the author roled a part of a layout table, which Chrome then keeps
/// (`bl-d61c`, Chrome 158): an explicit role on the part, or on the row a
/// cell sits in — `<tr role=row><td>x</td></tr>` is `row` > `cell "x"` with
/// no table node, while a `<tr>` under `<tbody role=rowgroup>` is demoted.
fn authored(doc: &Document, id: NodeId, el: &Element) -> bool {
    let explicit = |n: NodeId| doc.element(n).and_then(|e| ax::role::explicit(doc, n, e));
    let row = || doc.node(id).parent.and_then(explicit) == Some("row");
    explicit(id).is_some() || (matches!(el.name.as_str(), "td" | "th") && row())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod hidden_tests;

#[cfg(test)]
mod oracle_tests;

#[cfg(test)]
mod lineless_tests;

#[cfg(test)]
mod table_tests;
