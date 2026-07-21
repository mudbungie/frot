//! `--out meta`: page metadata.
//!
//! Output shape:
//! ```text
//! { "title": <str|null>,
//!   "lang": <str|null>,
//!   "charset": <str|null>,
//!   "canonical": <abs url|null>,
//!   "meta": [
//!     { "name": <str>?, "property": <str>?, "http-equiv": <str>?, "content": <str> }
//!   ]
//! }
//! ```

use crate::dom::{Document, NodeKind, WalkEvent};
use serde_json::{json, Value};
use url::Url;

pub fn meta(doc: &Document, page_url: &str) -> Value {
    let mut title: Option<String> = None;
    let mut lang: Option<String> = None;
    let mut charset: Option<String> = None;
    let mut canonical: Option<String> = None;
    let mut metas: Vec<Value> = Vec::new();
    let mut base_href: Option<String> = None;

    doc.walk(None, &mut |ev, e| {
        if let (WalkEvent::Enter(id), NodeKind::Element(el)) = (ev, &e.kind) {
            match el.name.as_str() {
                "title" if title.is_none() => {
                    title = Some(doc.text_content(id).trim().to_string());
                }
                "html" if lang.is_none() => {
                    lang = el.attr("lang").map(|s| s.to_string());
                }
                "base" if base_href.is_none() => {
                    base_href = el
                        .attr("href")
                        .filter(|h| !h.is_empty())
                        .map(|s| s.to_string());
                }
                "link" => {
                    let rel = el.attr("rel").unwrap_or("");
                    if rel
                        .split_whitespace()
                        .any(|t| t.eq_ignore_ascii_case("canonical"))
                    {
                        canonical = el.attr("href").map(|s| s.to_string());
                    }
                }
                "meta" => {
                    if let Some(cs) = el.attr("charset") {
                        if charset.is_none() {
                            charset = Some(cs.to_ascii_lowercase());
                        }
                    }
                    if let Some(http_equiv) = el.attr("http-equiv") {
                        if charset.is_none() && http_equiv.eq_ignore_ascii_case("content-type") {
                            if let Some(content) = el.attr("content") {
                                if let Some(cs) = extract_charset(content) {
                                    charset = Some(cs.to_ascii_lowercase());
                                }
                            }
                        }
                    }
                    if let Some(m) = build_meta_entry(el) {
                        metas.push(m);
                    }
                }
                _ => {}
            }
        }
    });

    let effective_base = base_href.as_deref().unwrap_or(page_url);
    let base_url = Url::parse(effective_base).ok();
    let canonical_resolved = canonical.as_deref().map(|c| resolve(base_url.as_ref(), c));

    json!({
        "title": title,
        "lang": lang,
        "charset": charset,
        "canonical": canonical_resolved,
        "meta": metas,
    })
}

fn extract_charset(content: &str) -> Option<&str> {
    let lower = content.to_ascii_lowercase();
    let key = "charset=";
    let idx = lower.find(key)?;
    let after = &content[idx + key.len()..];
    let end = after
        .find(|c: char| c == ';' || c.is_whitespace())
        .unwrap_or(after.len());
    let cs = after[..end].trim_matches(|c: char| c == '"' || c == '\'');
    if cs.is_empty() {
        None
    } else {
        Some(cs)
    }
}

fn build_meta_entry(el: &crate::dom::Element) -> Option<Value> {
    let content = el.attr("content")?;
    let mut obj = serde_json::Map::new();
    if let Some(n) = el.attr("name") {
        obj.insert("name".into(), Value::String(n.to_string()));
    }
    if let Some(p) = el.attr("property") {
        obj.insert("property".into(), Value::String(p.to_string()));
    }
    if let Some(h) = el.attr("http-equiv") {
        obj.insert("http-equiv".into(), Value::String(h.to_string()));
    }
    if obj.is_empty() {
        return None;
    }
    obj.insert("content".into(), Value::String(content.to_string()));
    Some(Value::Object(obj))
}

fn resolve(base: Option<&Url>, href: &str) -> String {
    base.and_then(|b| b.join(href).ok())
        .map(|u| u.to_string())
        .unwrap_or_else(|| href.to_string())
}

#[cfg(test)]
mod tests;
