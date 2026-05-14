//! `--out links`: extract anchors and link relations.
//!
//! Covered elements: `<a href>`, `<area href>`, `<link href rel>`. Each href
//! is resolved against the document's base URL (the `<base href>` element if
//! present, otherwise the page URL supplied by the caller).
//!
//! Output shape: `[{"kind": "a"|"area"|"link", "href": <abs>, "text": <str|null>,
//! "rel": [<token>...]}, ...]`.

use crate::dom::{Document, NodeId, NodeKind, WalkEvent};
use serde_json::{json, Value};
use url::Url;

pub fn links(doc: &Document, page_url: &str) -> Value {
    let effective_base = effective_base(doc, page_url);
    let base = Url::parse(&effective_base).ok();
    let mut out = Vec::new();
    for &root in doc.roots() {
        collect(doc, root, base.as_ref(), &mut out);
    }
    Value::Array(out)
}

fn effective_base(doc: &Document, fallback: &str) -> String {
    let mut found: Option<String> = None;
    doc.walk(None, &mut |ev, entry| {
        if found.is_some() {
            return;
        }
        if let (WalkEvent::Enter(_), NodeKind::Element(el)) = (ev, &entry.kind) {
            if el.name == "base" {
                if let Some(href) = el.attr("href").filter(|h| !h.is_empty()) {
                    found = Some(href.to_string());
                }
            }
        }
    });
    found.unwrap_or_else(|| fallback.to_string())
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
                let resolved = base
                    .and_then(|b| b.join(href).ok())
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| href.to_string());
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
