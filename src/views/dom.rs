//! `--out dom`: serialize the parsed DOM as JSON.
//!
//! Output shape per node:
//! - element  → `{"type": "element", "name": "...", "attrs": {...}, "children": [...]}`
//! - text     → `{"type": "text", "value": "..."}`
//! - comment  → `{"type": "comment", "value": "..."}`
//! - doctype  → `{"type": "doctype"}`
//!
//! The top-level value is an array of root nodes. We do not synthesize a
//! `document` wrapper — callers see the same roots `Document::roots()` exposes.

use crate::dom::{Document, NodeId, NodeKind};
use serde_json::{json, Value};

pub fn dom_json(doc: &Document) -> Value {
    let roots: Vec<Value> = doc.roots().iter().map(|&id| node_json(doc, id)).collect();
    Value::Array(roots)
}

fn node_json(doc: &Document, id: NodeId) -> Value {
    let entry = doc.node(id);
    match &entry.kind {
        NodeKind::Element(el) => {
            let mut attrs = serde_json::Map::new();
            for a in &el.attrs {
                attrs.insert(a.name.clone(), Value::String(a.value.clone()));
            }
            let children: Vec<Value> = entry
                .children
                .iter()
                .map(|&c| node_json(doc, c))
                .collect();
            json!({
                "type": "element",
                "name": el.name,
                "attrs": Value::Object(attrs),
                "children": children,
            })
        }
        NodeKind::Text(t) => json!({ "type": "text", "value": t }),
        NodeKind::Comment(c) => json!({ "type": "comment", "value": c }),
        NodeKind::Doctype => json!({ "type": "doctype" }),
    }
}

#[cfg(test)]
mod tests;
