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
//! A `<p>` whose box holds no line — empty, or with a block-level child —
//! contributes no node either, and nor does such a table `<caption>`
//! ([`lineless`]).
//!
//! ## Layout-table demotion
//!
//! Much HTML nests `<table>` purely for visual grid layout (Hacker News is a
//! canonical example). Real browsers run layout-table heuristics in the AX
//! layer and demote such tables to `role=presentation`, so their scaffolding
//! collapses out of the tree and only content surfaces. We apply the
//! **DOM-only** part of Chromium's test (`IsDataTable`), measured in Chrome 158
//! (`bl-dfc3`, `bl-7f3a`): a `<table>` is treated as *layout* (and its
//! table-structural descendants — `table`, `rowgroup`, `row`, `cell`,
//! `columnheader`, `rowheader`, `gridcell` — collapse to presentation) unless
//!
//! 1. it carries a `role`, a non-empty `summary` or a non-empty `rules`, or a
//!    `<caption>`, `<thead>`, `<tfoot>`, `<colgroup>` or `<col>` child — any
//!    of which keeps even a one-cell table; or, short of that,
//! 2. it is more than one row holding one cell, and it has 20 rows, or a
//!    `<th>`, or a cell with a non-empty `headers`, `abbr`, `axis` or
//!    `scope`; or it has two cells and a `border` that is not zero (Chrome
//!    reads the cell borders the attribute draws: `border=""` and
//!    `border=abc` count, `border=0px` does not).
//!
//! `aria-label` and `aria-labelledby` are no signal: Chrome names a layout
//! table and still leaves it out. Cell borders and backgrounds set by CSS,
//! and `contenteditable`, are signals in Chrome that this DOM-only reading
//! does not see. Cells, rows and headers are counted without descending into
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
    if ax::excluded(el) {
        return Vec::new();
    }
    if styles.is_some_and(|s| s.display_none(id)) {
        return Vec::new();
    }
    // A `<table>` re-classifies the nearest-table context (nesting resets, so a
    // data table inside a layout table keeps its structure); other elements
    // inherit the ancestor flag.
    let layout_table = if el.name == "table" {
        is_layout_table(doc, id, el)
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
            if let Some(level) = ax::level(el) {
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

/// DOM-only layout-table test: `true` (demote to presentation) when a
/// `<table>` carries none of the data-table signals in the module header.
fn is_layout_table(doc: &Document, id: NodeId, el: &Element) -> bool {
    let said = |a: &str| el.attr(a).is_some_and(|s| !s.is_empty());
    let structure = ["caption", "thead", "tfoot", "colgroup", "col"];
    let child = doc.node(id).children.iter().any(|&c| {
        matches!(&doc.node(c).kind, NodeKind::Element(e) if structure.contains(&e.name.as_str()))
    });
    if said("summary") || said("rules") || el.attr("role").is_some() || child {
        return false;
    }
    let parts = |tags: &[&str]| {
        let mut found = Vec::new();
        parts(doc, id, tags, &mut found);
        found
    };
    let (rows, cells) = (parts(&["tr"]).len(), parts(&["td", "th"]));
    let marked = |c: &&Element| {
        c.name == "th"
            || ["headers", "abbr", "axis", "scope"]
                .iter()
                .any(|a| c.attr(a).is_some_and(|v| !v.is_empty()))
    };
    let data = rows >= 20 || cells.iter().any(marked) || (cells.len() > 1 && bordered(el));
    (rows == 1 && cells.len() == 1) || !data
}

/// Whether the `border` attribute draws cell borders: present, and not a
/// non-negative integer equal to zero — an unparsable value means 1.
fn bordered(el: &Element) -> bool {
    el.attr("border").is_some_and(|v| {
        let digits: String = v
            .trim_start()
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        digits.is_empty() || digits.bytes().any(|b| b != b'0')
    })
}

/// The `tags` elements in the subtree, without descending into nested
/// `<table>` elements — an inner data table's header must not rescue an outer
/// layout table, and its cells are not the outer table's.
fn parts<'d>(doc: &'d Document, id: NodeId, tags: &[&str], found: &mut Vec<&'d Element>) {
    for &c in &doc.node(id).children {
        match &doc.node(c).kind {
            NodeKind::Element(e) if tags.contains(&e.name.as_str()) => found.push(e),
            NodeKind::Element(e) if e.name != "table" => parts(doc, c, tags, found),
            _ => {}
        }
    }
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
