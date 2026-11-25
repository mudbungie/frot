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
//!
//! A shadow host's shadow root serializes as its first child, a `template`
//! element carrying `shadowrootmode` (and the `shadowroot*` attributes of the
//! options it was attached with) whose children are the root's — the form
//! Chrome's `getHTML({serializableShadowRoots})` writes and the parser turns
//! back into the same root (js.md §15). A page with no shadow root serializes
//! as it always has.

use crate::dom::{Document, NodeId, NodeKind};
use serde_json::{json, Value};

pub fn dom_json(doc: &Document) -> Value {
    node_json(doc, Document::ROOT)
}

fn children_json(doc: &Document, id: NodeId) -> Value {
    let kids = &doc.node(id).children;
    let shadow = doc.shadow_root(id).map(|root| shadow_json(doc, root));
    Value::Array(
        shadow
            .into_iter()
            .chain(kids.iter().map(|&c| node_json(doc, c)))
            .collect(),
    )
}

fn shadow_json(doc: &Document, root: NodeId) -> Value {
    let s = doc.shadow(root).unwrap_or_default();
    let mut attrs = serde_json::Map::new();
    let mode = if s.closed { "closed" } else { "open" };
    attrs.insert("shadowrootmode".into(), mode.into());
    let flags = [
        ("shadowrootdelegatesfocus", s.delegates_focus),
        ("shadowrootserializable", s.serializable),
        ("shadowrootclonable", s.clonable),
    ];
    for (name, _) in flags.iter().filter(|f| f.1) {
        attrs.insert((*name).into(), "".into());
    }
    if s.manual {
        attrs.insert("shadowrootslotassignment".into(), "manual".into());
    }
    json!({
        "type": "element",
        "name": "template",
        "attrs": Value::Object(attrs),
        "children": children_json(doc, root),
    })
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
        NodeKind::Doctype(_) => json!({ "type": "doctype" }),
        NodeKind::Document | NodeKind::Fragment | NodeKind::Shadow(_) => children_json(doc, id),
    }
}

#[cfg(test)]
mod tests;
