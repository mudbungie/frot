//! `--out forms`: extract `<form>` elements and their controls.
//!
//! Output shape: an array of form objects.
//! ```text
//! { "action": <abs url>, "method": "get"|"post"|...,
//!   "enctype": "application/x-www-form-urlencoded" (or supplied),
//!   "fields": [
//!      { "tag": "input"|"textarea"|"select"|"button",
//!        "name": <str|null>, "type": <str|null>, "value": <str|null>,
//!        "required": bool, "options": [<str>...]? }
//!   ]
//! }
//! ```

use crate::dom::{Document, Element, NodeId, NodeKind, WalkEvent};
use serde_json::{json, Value};
use url::Url;

pub fn forms(doc: &Document, page_url: &str) -> Value {
    let base = effective_base(doc, page_url);
    let base_url = Url::parse(&base).ok();
    let mut out = Vec::new();
    doc.walk(None, &mut |ev, e| {
        if let (WalkEvent::Enter(id), NodeKind::Element(el)) = (ev, &e.kind) {
            if el.name == "form" {
                out.push(build_form(doc, id, el, base_url.as_ref()));
            }
        }
    });
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

fn resolve(base: Option<&Url>, href: &str) -> String {
    base.and_then(|b| b.join(href).ok())
        .map(|u| u.to_string())
        .unwrap_or_else(|| href.to_string())
}

fn build_form(doc: &Document, id: NodeId, form_el: &Element, base: Option<&Url>) -> Value {
    let action_raw = form_el.attr("action").unwrap_or("");
    let action = resolve(base, action_raw);
    let method = form_el
        .attr("method")
        .map(|m| m.to_ascii_lowercase())
        .unwrap_or_else(|| "get".to_string());
    let enctype = form_el
        .attr("enctype")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "application/x-www-form-urlencoded".to_string());

    let mut fields = Vec::new();
    doc.walk(Some(id), &mut |ev, e| {
        if let (WalkEvent::Enter(node_id), NodeKind::Element(el)) = (ev, &e.kind) {
            if matches!(el.name.as_str(), "input" | "textarea" | "select" | "button") {
                fields.push(build_field(doc, node_id, el));
            }
        }
    });

    json!({
        "action": action,
        "method": method,
        "enctype": enctype,
        "fields": fields,
    })
}

fn build_field(doc: &Document, id: NodeId, el: &Element) -> Value {
    let name = el.attr("name").map(|s| s.to_string());
    let typ = if matches!(el.name.as_str(), "input" | "button") {
        el.attr("type").map(|s| s.to_string())
    } else {
        None
    };
    let value = if el.name == "textarea" {
        Some(doc.text_content(id))
    } else {
        el.attr("value").map(|s| s.to_string())
    };
    let required = el.attr("required").is_some();
    let mut obj = serde_json::Map::new();
    obj.insert("tag".into(), Value::String(el.name.clone()));
    obj.insert("name".into(), name.map_or(Value::Null, Value::String));
    obj.insert("type".into(), typ.map_or(Value::Null, Value::String));
    obj.insert("value".into(), value.map_or(Value::Null, Value::String));
    obj.insert("required".into(), Value::Bool(required));
    if el.name == "select" {
        obj.insert("options".into(), Value::Array(collect_options(doc, id)));
    }
    Value::Object(obj)
}

fn collect_options(doc: &Document, select_id: NodeId) -> Vec<Value> {
    let mut out = Vec::new();
    doc.walk(Some(select_id), &mut |ev, e| {
        if let (WalkEvent::Enter(opt_id), NodeKind::Element(el)) = (ev, &e.kind) {
            if el.name == "option" {
                let label = doc.text_content(opt_id);
                let value = el
                    .attr("value")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| label.trim().to_string());
                out.push(json!({"value": value, "label": label.trim()}));
            }
        }
    });
    out
}

#[cfg(test)]
mod tests;
