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
//! collapses out of the tree and only content surfaces. We apply a
//! **DOM-only** subset of Chromium's rules — no layout engine, purely the
//! markup: a `<table>` is treated as *layout* (and its table-structural
//! descendants — `table`, `rowgroup`, `row`, `cell`, `columnheader`,
//! `rowheader`, `gridcell` — collapse to presentation) when it carries
//! **none** of these author signals of a real data table:
//!
//! - a `<caption>` direct child,
//! - a `summary` attribute that is not empty,
//! - a `role` attribute (any explicit role signals author intent),
//! - a `<th>` inside it (not counting cells owned by a *nested* `<table>`),
//! - `aria-label` or `aria-labelledby`.
//!
//! The last two do not rescue a table of **one row holding one cell**:
//! measured in Chrome 158 (`bl-dfc3`), `<table><tr><th>x</th></tr></table>`
//! is a layout table — with `title`, `summary=""` or `aria-label` too — while
//! a caption or a `summary=S` keeps even that one cell a table.
//!
//! The bias is otherwise conservative: any single signal preserves the
//! full table structure, because losing real structure is worse than keeping
//! some layout noise. The `<th>` scan stops at nested `<table>` boundaries so
//! an inner data table's header cannot rescue an outer layout table. Table
//! nesting likewise *resets* the flag (each `<table>` re-classifies), so a
//! genuine data table nested inside a layout table keeps its own structure.

use crate::ax;
use crate::css::{Styles, Visibility};
use crate::dom::{Document, Element, NodeId, NodeKind};
use crate::tags;
use serde_json::{json, Value};

mod lineless;

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

/// DOM-only layout-table test: `true` (demote to presentation) when a
/// `<table>` carries none of the data-table author signals documented in the
/// module header.
fn is_layout_table(doc: &Document, id: NodeId, el: &Element) -> bool {
    if el.attr("summary").is_some_and(|s| !s.is_empty())
        || el.attr("role").is_some()
        || has_caption_child(doc, id)
    {
        return false;
    }
    let count = |tags: &[&str]| {
        let mut n = 0;
        count_parts(doc, id, tags, &mut n);
        n
    };
    let labelled = el.attr("aria-label").is_some() || el.attr("aria-labelledby").is_some();
    let one_cell = count(&["tr"]) == 1 && count(&["td", "th"]) == 1;
    one_cell || !(labelled || count(&["th"]) > 0)
}

/// Whether a direct child of `id` is a `<caption>` element.
fn has_caption_child(doc: &Document, id: NodeId) -> bool {
    doc.node(id)
        .children
        .iter()
        .any(|&c| matches!(&doc.node(c).kind, NodeKind::Element(e) if e.name == "caption"))
}

/// Count the `tags` elements in the subtree, without descending into nested
/// `<table>` elements — an inner data table's header must not rescue an outer
/// layout table, and its cells are not the outer table's.
fn count_parts(doc: &Document, id: NodeId, tags: &[&str], n: &mut usize) {
    for &c in &doc.node(id).children {
        match &doc.node(c).kind {
            NodeKind::Element(e) if tags.contains(&e.name.as_str()) => *n += 1,
            NodeKind::Element(e) if e.name != "table" => count_parts(doc, c, tags, n),
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
