//! `--out links`: extract anchors and link relations.
//!
//! Covered elements: `<a href>`, `<area href>`, `<link href rel>`. Each href is
//! resolved against the document base URL ([`crate::base`]) — the shared
//! authority, so a `<base href>` means the same thing here as everywhere else.
//!
//! Output shape: `[{"kind": "a"|"area"|"link", "href": <abs>, "text": <str|null>,
//! "rel": [<token>...]}, ...]`.

use crate::base;
use crate::dom::{Document, NodeId, NodeKind};
use serde_json::{json, Value};
use url::Url;

pub fn links(doc: &Document, page_url: &str) -> Value {
    let base = base::base_url(doc, page_url);
    let mut out = Vec::new();
    for &root in doc.roots() {
        collect(doc, root, base.as_ref(), &mut out);
    }
    Value::Array(out)
}

fn normalize_text(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collect(doc: &Document, id: NodeId, base: Option<&Url>, out: &mut Vec<Value>) {
    let entry = doc.node(id);
    if let NodeKind::Element(el) = &entry.kind {
        let kind = el.name.as_str();
        if matches!(kind, "a" | "area" | "link") {
            if let Some(href) = el.attr("href") {
                let resolved = base::resolve(base, href);
                let text = if kind == "link" {
                    Value::Null
                } else {
                    Value::String(normalize_text(&doc.text_content(id)))
                };
                let rel: Vec<Value> = el
                    .attr("rel")
                    .unwrap_or("")
                    .split_whitespace()
                    .map(|t| Value::String(t.to_string()))
                    .collect();
                out.push(json!({
                    "kind": kind,
                    "href": resolved,
                    "text": text,
                    "rel": Value::Array(rel),
                }));
            }
        }
    }
    let kids: Vec<NodeId> = entry.children.clone();
    for c in kids {
        collect(doc, c, base, out);
    }
}

#[cfg(test)]
mod tests;
