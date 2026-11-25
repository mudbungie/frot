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
    label(doc, control_id, el).map(|id| contents::element_name(doc, id, styles, &[control_id]))
}

/// A control's `<label>`: the first whose `for` names its `id`, else the
/// outermost one around it.
pub(super) fn label(doc: &Document, control_id: NodeId, el: &Element) -> Option<NodeId> {
    let target = el.attr("id");
    let mut for_match: Option<NodeId> = None;
    let mut label_stack: Vec<NodeId> = Vec::new();
    let mut wrapping_label: Option<NodeId> = None;
    // Labels are found in the control's own tree (a shadow tree's labels
    // label its controls, js.md §15).
    doc.walk(Some(doc.tree_root(control_id)), &mut |ev, e| match ev {
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
    for_match.or(wrapping_label)
}

pub fn fieldset_legend_text(
    doc: &Document,
    fieldset_id: NodeId,
    styles: Option<&Styles>,
) -> Option<String> {
    legend(doc, fieldset_id).map(|id| contents::element_name(doc, id, styles, &[]))
}

/// The `<legend>` of a `<fieldset>`: the first one inside it.
pub(super) fn legend(doc: &Document, fieldset_id: NodeId) -> Option<NodeId> {
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
    found
}

/// HTML-AAM: a `<table>` is named by its first `<caption>` child. Unlike a
/// `<legend>`, the caption is read as a *descendant* — measured in Chrome 156,
/// `<caption aria-label=A>t</caption>` names the table "A" but
/// `<caption title=T></caption>` names it nothing.
pub fn table_caption_text(
    doc: &Document,
    table: NodeId,
    styles: Option<&Styles>,
) -> Option<String> {
    caption(doc, table, styles).map(|c| contents::descendant_name(doc, c, styles))
}

/// The `<caption>` that names a `<table>`: its first `<caption>` child that
/// is not hidden. Measured in Chrome 158 (`bl-dfc3`): a caption that is
/// `display:none`, `visibility:hidden`, `hidden` or `aria-hidden` is no
/// caption — the next one names the table (`<caption hidden></caption>
/// <caption>second</caption>` is "second"), and with none left `summary` and
/// `title` do.
fn caption(doc: &Document, table: NodeId, styles: Option<&Styles>) -> Option<NodeId> {
    let shown = |&c: &NodeId| {
        matches!(&doc.node(c).kind,
            NodeKind::Element(e) if e.name == "caption" && !contents::hidden(c, e, styles))
    };
    doc.node(table).children.iter().copied().find(shown)
}

/// Whether `el` has the element that is its **whole** native name, so that
/// `title` is never consulted. Measured in Chrome 158 (`bl-dfc3`):
/// `<table title=T><caption></caption>` and `<fieldset title=T><legend>
/// </legend>` are both named "". For a table that is a caption that is not
/// hidden ([`caption`]); for a fieldset any `<legend>`, hidden or not.
pub fn names_it_alone(doc: &Document, id: NodeId, el: &Element, styles: Option<&Styles>) -> bool {
    match el.name.as_str() {
        "table" => caption(doc, id, styles).is_some(),
        "fieldset" => legend(doc, id).is_some(),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
