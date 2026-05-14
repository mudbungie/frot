//! Capability gap detection.
//!
//! When the current recipe (Phase 0 = no `--css`, no `--js`) is insufficient
//! to faithfully produce a content-dependent view, [`detect`] reports which
//! capabilities the page wants. The envelope's `status` flips to `needs` and
//! the listed capabilities are surfaced to the caller.
//!
//! Phase 1 lights up the `js` capability via a coarse SPA-shell heuristic.

use crate::dom::{Document, NodeId, NodeKind, WalkEvent};
use crate::envelope::{NeedsKind, View};

/// Views whose output materially depends on rendered body content.
fn view_depends_on_content(view: View) -> bool {
    matches!(
        view,
        View::Text | View::Ax | View::Links | View::Forms
    )
}

pub fn detect(view: View, doc: &Document) -> Vec<NeedsKind> {
    if !view_depends_on_content(view) {
        return Vec::new();
    }
    let mut out = Vec::new();
    if needs_js(doc) {
        out.push(NeedsKind::Js);
    }
    out
}

fn needs_js(doc: &Document) -> bool {
    let Some(body) = find_body(doc) else {
        return false;
    };
    let body_text = doc.text_content(body);
    if !body_text.trim().is_empty() {
        return false;
    }
    has_scripts(doc) && body_has_few_descendants(doc, body)
}

fn body_has_few_descendants(doc: &Document, body: NodeId) -> bool {
    let mut count = 0usize;
    doc.walk(Some(body), &mut |ev, e| {
        if let WalkEvent::Enter(_) = ev {
            if let NodeKind::Element(el) = &e.kind {
                if !matches!(
                    el.name.as_str(),
                    "body" | "script" | "style" | "noscript" | "template"
                ) {
                    count += 1;
                }
            }
        }
    });
    count <= 3
}

fn find_body(doc: &Document) -> Option<NodeId> {
    doc.find_by_tag("body").first().copied()
}

fn has_scripts(doc: &Document) -> bool {
    !doc.find_by_tag("script").is_empty()
}

#[cfg(test)]
mod tests;
