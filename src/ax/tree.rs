//! AX tree builder for `--out ax`.
//!
//! Walks the DOM in source order producing a flat JSON shape per node:
//! `{ "role": <str>, "name": <str|null>, "level"?: <int>, "children": [...] }`.
//!
//! Elements whose effective role is `None`, `generic`, `presentation`, or
//! `none` do not contribute a node — their AX children promote into the
//! parent's child list. Subtrees under `<script>`, `<style>`, `<template>`,
//! and `<noscript>` are skipped entirely.

use crate::ax;
use crate::dom::{Document, NodeId, NodeKind};
use serde_json::{json, Value};

const SKIP_TAGS: &[&str] = &["script", "style", "template", "noscript"];

pub fn ax_tree(doc: &Document) -> Value {
    let mut nodes = Vec::new();
    for &root in doc.roots() {
        nodes.extend(build(doc, root));
    }
    Value::Array(nodes)
}

fn build(doc: &Document, id: NodeId) -> Vec<Value> {
    let entry = doc.node(id);
    let NodeKind::Element(el) = &entry.kind else {
        return Vec::new();
    };
    if SKIP_TAGS.contains(&el.name.as_str()) {
        return Vec::new();
    }
    let mut children = Vec::new();
    for &c in &entry.children {
        children.extend(build(doc, c));
    }
    match ax::role(el) {
        None | Some("generic") | Some("presentation") | Some("none") => children,
        Some(role_name) => {
            let name = ax::accessible_name(doc, id);
            let mut obj = serde_json::Map::new();
            obj.insert("role".into(), Value::String(role_name.to_string()));
            obj.insert(
                "name".into(),
                name.map_or(Value::Null, Value::String),
            );
            if let Some(level) = ax::level(el) {
                obj.insert("level".into(), json!(level));
            }
            obj.insert("children".into(), Value::Array(children));
            vec![Value::Object(obj)]
        }
    }
}

#[cfg(test)]
mod tests;
