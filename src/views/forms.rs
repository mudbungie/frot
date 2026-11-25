//! `--out forms`: extract `<form>` elements and their controls.
//!
//! A form's fields are the controls it *owns* ([`Document::form_owner`]), in
//! tree order: the ones inside it, less any whose `form` attribute points
//! elsewhere, plus any outside it whose `form` attribute names it
//! (`bl-e8fa`).
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

use crate::base;
use crate::dom::{Document, Element, NodeId, NodeKind, WalkEvent};
use serde_json::{json, Value};
use url::Url;

pub fn forms(doc: &Document, page_url: &str) -> Value {
    let base_url = base::base_url(doc, page_url);
    // Every control with the form that owns it, in tree order: a form's
    // fields are the controls it owns, wherever they stand.
    let mut controls = Vec::new();
    let mut found = Vec::new();
    doc.walk_flat(None, &mut |ev, e| {
        if let (WalkEvent::Enter(id), NodeKind::Element(el)) = (ev, &e.kind) {
            match el.name.as_str() {
                "form" => found.push((id, build_form(el, base_url.as_ref()))),
                "input" | "textarea" | "select" | "button" => {
                    controls.push((doc.form_owner(id), build_field(doc, id, el)));
                }
                _ => {}
            }
        }
    });
    let form = |(id, mut form): (NodeId, Value)| {
        let owned = controls.iter().filter(|(owner, _)| *owner == Some(id));
        form["fields"] = owned.map(|(_, field)| field.clone()).collect();
        form
    };
    Value::Array(found.into_iter().map(form).collect())
}

fn build_form(form_el: &Element, base: Option<&Url>) -> Value {
    let action_raw = form_el.attr("action").unwrap_or("");
    let action = base::resolve(base, action_raw);
    let method = form_el
        .attr("method")
        .map(|m| m.to_ascii_lowercase())
        .unwrap_or_else(|| "get".to_string());
    let enctype = form_el
        .attr("enctype")
        .map(|s| s.to_string())
        .unwrap_or_else(|| "application/x-www-form-urlencoded".to_string());

    json!({
        "action": action,
        "method": method,
        "enctype": enctype,
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
    doc.walk_flat(Some(select_id), &mut |ev, e| {
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
