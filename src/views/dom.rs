//! `--out dom`: serialize the parsed DOM as JSON.
//!
//! Output shape per node:
//! - element  → `{"type": "element", "name": "...", "attrs": {...}, "children": [...]}`
//! - text     → `{"type": "text", "value": "..."}`
//! - comment  → `{"type": "comment", "value": "..."}`
//! - doctype  → `{"type": "doctype"}`
//!
//! The top-level value is an array of root nodes: the document node serializes
//! as its children, with no `document` wrapper — callers see the same roots
//! `Document::roots()` exposes.

use crate::dom::{Document, NodeId, NodeKind};
use serde_json::{json, Value};

pub fn dom_json(doc: &Document) -> Value {
    node_json(doc, Document::ROOT)
}

fn children_json(doc: &Document, id: NodeId) -> Value {
    let kids = &doc.node(id).children;
    Value::Array(kids.iter().map(|&c| node_json(doc, c)).collect())
}

fn node_json(doc: &Document, id: NodeId) -> Value {
    let entry = doc.node(id);
    match &entry.kind {
        NodeKind::Element(el) => {
            let mut attrs = serde_json::Map::new();
            for a in &el.attrs {
                attrs.insert(a.name.clone(), Value::String(a.value.clone()));
            }
            json!({
                "type": "element",
                "name": el.name,
                "attrs": Value::Object(attrs),
                "children": children_json(doc, id),
            })
        }
        NodeKind::Text(t) => json!({ "type": "text", "value": t }),
        NodeKind::Comment(c) => json!({ "type": "comment", "value": c }),
        NodeKind::Doctype => json!({ "type": "doctype" }),
        NodeKind::Document => children_json(doc, id),
    }
}

#[cfg(test)]
mod tests;
