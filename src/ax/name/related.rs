//! Names an element takes from a *related* element rather than from itself:
//! a control from its `<label>`, a `<fieldset>` from its `<legend>`, a
//! `<table>` from its `<caption>`. Each finds the related node and hands it to
//! the door of [`contents`] that matches how a browser reads it.

use super::contents;
use crate::css::Styles;
use crate::dom::{Document, Element, NodeId, NodeKind, WalkEvent};

pub fn control_label_text(
    doc: &Document,
    control_id: NodeId,
    el: &Element,
    styles: Option<&Styles>,
) -> Option<String> {
    let target = el.attr("id");
    let mut for_match: Option<NodeId> = None;
    let mut label_stack: Vec<NodeId> = Vec::new();
    let mut wrapping_label: Option<NodeId> = None;
    doc.walk(None, &mut |ev, e| match ev {
        WalkEvent::Enter(id) => {
            if let NodeKind::Element(elx) = &e.kind {
                if elx.name == "label" {
                    label_stack.push(id);
                    if let Some(t) = target {
                        if for_match.is_none() && elx.attr("for") == Some(t) {
                            for_match = Some(id);
                        }
                    }
                } else if id == control_id && wrapping_label.is_none() {
                    wrapping_label = label_stack.first().copied();
                }
            }
        }
        WalkEvent::Exit(id) => {
            if label_stack.last() == Some(&id) {
                label_stack.pop();
            }
        }
    });
    for_match
        .or(wrapping_label)
        .map(|id| contents::element_name(doc, id, styles, &[control_id]))
}

pub fn fieldset_legend_text(
    doc: &Document,
    fieldset_id: NodeId,
    styles: Option<&Styles>,
) -> Option<String> {
    let mut found: Option<NodeId> = None;
    doc.walk(Some(fieldset_id), &mut |ev, e| {
        if found.is_some() {
            return;
        }
        if let (WalkEvent::Enter(id), NodeKind::Element(el)) = (ev, &e.kind) {
            if id != fieldset_id && el.name == "legend" {
                found = Some(id);
            }
        }
    });
    found.map(|id| contents::element_name(doc, id, styles, &[]))
}

/// HTML-AAM: a `<table>` is named by its first `<caption>` child. Unlike a
/// `<legend>`, the caption is read as a *descendant* — measured in Chrome 156,
/// `<caption aria-label=A>t</caption>` names the table "A" but
/// `<caption title=T></caption>` names it nothing, and a caption CSS hides
/// names nothing either.
pub fn table_caption_text(
    doc: &Document,
    table: NodeId,
    styles: Option<&Styles>,
) -> Option<String> {
    let caption = doc
        .node(table)
        .children
        .iter()
        .copied()
        .find(|&c| matches!(&doc.node(c).kind, NodeKind::Element(e) if e.name == "caption"))?;
    Some(contents::descendant_name(doc, caption, styles))
}
