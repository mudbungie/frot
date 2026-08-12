//! AX tree builder for `--out ax`.
//!
//! Walks the DOM in source order producing a flat JSON shape per node:
//! `{ "role": <str>, "name": <str|null>, "level"?: <int>, "children": [...] }`.
//!
//! Elements whose effective role is `None`, `generic`, `presentation`, or
//! `none` do not contribute a node — their AX children promote into the
//! parent's child list. Subtrees under `<script>`, `<style>`, `<template>`,
//! and `<noscript>` are skipped entirely, as are subtrees excluded by
//! `aria-hidden`/`inert` ([`crate::ax::hidden`]).
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
//! - a `<th>` inside it (not counting cells owned by a *nested* `<table>`),
//! - a `summary` attribute,
//! - a `role` attribute (any explicit role signals author intent),
//! - `aria-label` or `aria-labelledby`.
//!
//! The bias is deliberately conservative: any single signal preserves the
//! full table structure, because losing real structure is worse than keeping
//! some layout noise. The `<th>` scan stops at nested `<table>` boundaries so
//! an inner data table's header cannot rescue an outer layout table. Table
//! nesting likewise *resets* the flag (each `<table>` re-classifies), so a
//! genuine data table nested inside a layout table keeps its own structure.

use crate::ax;
use crate::css::{Display, Styles, Visibility};
use crate::dom::{Document, Element, NodeId, NodeKind};
use crate::layout::Layout;
use serde_json::{json, Value};

const SKIP_TAGS: &[&str] = &["script", "style", "template", "noscript"];

pub fn ax_tree(doc: &Document, styles: Option<&Styles>, layout: Option<&Layout>) -> Value {
    let mut nodes = Vec::new();
    for &root in doc.roots() {
        nodes.extend(build(doc, root, styles, layout, false));
    }
    Value::Array(nodes)
}

/// The child `NodeId`s of `id` in AX emission order. Under a computed `Layout`,
/// a flex container (`Display::Flex`/`InlineFlex`) emits its children in flex
/// reading order — [`Layout::child_order`], the single reorder primitive from
/// subtask 3.5 (`layout.md` §5). Every other node, and *every* node when
/// `layout` is `None` (no `--css`), keeps source order, so `ax` without layout
/// is bit-for-bit unchanged. Reordering stays within a container: children are
/// resequenced among siblings, never moved across containers.
fn ordered_children(
    doc: &Document,
    id: NodeId,
    styles: Option<&Styles>,
    layout: Option<&Layout>,
) -> Vec<NodeId> {
    match (layout, styles) {
        (Some(l), Some(s)) if matches!(s.display(id), Display::Flex | Display::InlineFlex) => {
            l.child_order(id)
        }
        _ => doc.node(id).children.clone(),
    }
}

fn build(
    doc: &Document,
    id: NodeId,
    styles: Option<&Styles>,
    layout: Option<&Layout>,
    in_layout_table: bool,
) -> Vec<Value> {
    let entry = doc.node(id);
    let NodeKind::Element(el) = &entry.kind else {
        return Vec::new();
    };
    if SKIP_TAGS.contains(&el.name.as_str()) {
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
    let mut children = Vec::new();
    for c in ordered_children(doc, id, styles, layout) {
        children.extend(build(doc, c, styles, layout, layout_table));
    }
    if styles.is_some_and(|s| s.visibility(id) == Visibility::Hidden) {
        // `visibility:hidden` removes this element's own node but a
        // `visibility:visible` descendant still surfaces (it built its
        // own node above), so promote the collected children.
        return children;
    }
    let role = ax::role(el);
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
/// module header. Conservative — any single signal preserves structure.
fn is_layout_table(doc: &Document, id: NodeId, el: &Element) -> bool {
    if el.attr("summary").is_some()
        || el.attr("role").is_some()
        || el.attr("aria-label").is_some()
        || el.attr("aria-labelledby").is_some()
    {
        return false;
    }
    if has_caption_child(doc, id) {
        return false;
    }
    !has_header_cell(doc, id)
}

/// Whether a direct child of `id` is a `<caption>` element.
fn has_caption_child(doc: &Document, id: NodeId) -> bool {
    doc.node(id)
        .children
        .iter()
        .any(|&c| matches!(&doc.node(c).kind, NodeKind::Element(e) if e.name == "caption"))
}

/// Whether a `<th>` appears in the subtree, without descending into nested
/// `<table>` elements — an inner data table's header must not rescue an outer
/// layout table.
fn has_header_cell(doc: &Document, id: NodeId) -> bool {
    doc.node(id)
        .children
        .iter()
        .any(|&c| match &doc.node(c).kind {
            NodeKind::Element(e) if e.name == "th" => true,
            NodeKind::Element(e) if e.name == "table" => false,
            _ => has_header_cell(doc, c),
        })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod order_tests;

#[cfg(test)]
mod hidden_tests;
