//! Constraint validation as a page shows it before anyone touches it:
//! `:valid`, `:invalid`, `:in-range` and `:out-of-range` (`bl-51e9`).
//! Measured in Chrome 158; like the rest of [`super::state`] the rules read
//! attributes, never script-set properties, and no user edit is assumed — so
//! `maxlength`/`minlength` (checked only after an edit) never fail.
//!
//! - **Candidates** — an `<input>` that is not `hidden`, `reset`, `button` or
//!   `image`, a `<select>`, a `<textarea>`, a submit `<button>`; barred when
//!   disabled, inside a `<datalist>`, or an input or textarea with `readonly`
//!   (a checkbox too, though `readonly` does not apply to it). Neither
//!   pseudo-class matches anything else, except:
//! - **`<form>` and `<fieldset>`** — `:invalid` when a candidate they hold (a
//!   form: one it owns; a fieldset: one inside it) is invalid, else `:valid`.
//! - **Missing** — `required` with an empty value (as the type sanitizes it),
//!   an unchecked checkbox, a named radio group none of whose radios is
//!   checked while one is `required` (every radio of it is invalid; a radio
//!   with no name is never missing), a file input, a
//!   select whose pick is its placeholder option or nothing.
//! - **The value's own constraints** ([`value`]): type, `pattern`, range and
//!   step mismatches, all skipped for an empty value.

use super::checked::{group, selected};
use super::control::elements;
use super::state::{disabled, has, input_type, required};
use crate::dom::{Document, Element, NodeId, NodeKind, WalkEvent};

mod value;

/// `Some(true)` for `:valid`, `Some(false)` for `:invalid`, `None` for neither.
pub(super) fn valid(doc: &Document, id: NodeId, el: &Element) -> Option<bool> {
    let mut clean = true;
    match el.name.as_str() {
        "form" => elements(doc, &mut |n, e| {
            clean &= doc.form_owner(n) != Some(id) || !invalid(doc, n, e);
        }),
        "fieldset" => doc.walk(Some(id), &mut |ev, entry| {
            if let (WalkEvent::Enter(n), NodeKind::Element(e)) = (ev, &entry.kind) {
                clean &= !invalid(doc, n, e);
            }
        }),
        _ => return candidate(doc, id, el).then(|| !suffering(doc, id, el)),
    }
    Some(clean)
}

/// `Some(true)` for `:in-range`, `Some(false)` for `:out-of-range`: a
/// candidate whose type has range limitations ([`value::range`]).
pub(super) fn in_range(doc: &Document, id: NodeId, el: &Element) -> Option<bool> {
    let kind = value::kind(el);
    let input = el.name == "input";
    value::range(el, &kind).filter(|_| input && candidate(doc, id, el))
}

fn invalid(doc: &Document, id: NodeId, el: &Element) -> bool {
    candidate(doc, id, el) && suffering(doc, id, el)
}

fn candidate(doc: &Document, id: NodeId, el: &Element) -> bool {
    let kind = input_type(el);
    let (can, readonly) = match el.name.as_str() {
        "input" => (
            !matches!(kind.as_str(), "hidden" | "reset" | "button" | "image"),
            true,
        ),
        "button" => (!matches!(kind.as_str(), "button" | "reset"), false),
        name => (matches!(name, "select" | "textarea"), name == "textarea"),
    };
    let in_datalist = super::selector::ancestors(doc, id).any(|(_, e)| e.name == "datalist");
    let barred = readonly && has(el, "readonly") || disabled(doc, id, el) || in_datalist;
    can && !barred
}

fn suffering(doc: &Document, id: NodeId, el: &Element) -> bool {
    let kind = value::kind(el);
    let input = el.name == "input";
    missing(doc, id, el) || input && value::bad(el, &kind)
}

fn missing(doc: &Document, id: NodeId, el: &Element) -> bool {
    match (el.name.as_str(), input_type(el).as_str()) {
        ("input", "radio") => {
            let mine = group(doc, id, el);
            let (mut on, mut asked) = (false, false);
            elements(doc, &mut |n, e| {
                let radio = e.name == "input" && input_type(e) == "radio";
                let same = radio && group(doc, n, e) == mine;
                on |= same && has(e, "checked");
                asked |= same && has(e, "required");
            });
            mine.is_some() && asked && !on
        }
        _ if !required(el) => false,
        ("input", "checkbox") => !has(el, "checked"),
        ("input", "file") => true,
        ("input", _) => value::sanitized(el, &value::kind(el)).is_empty(),
        ("select", _) => select_missing(doc, id, el),
        _ => doc.text_content(id).is_empty(),
    }
}

/// A required select's pick is nothing, or — for a drop-down (no `multiple`,
/// `size` at most 1) — its placeholder: a first option that is the select's
/// own child and whose value is empty.
fn select_missing(doc: &Document, id: NodeId, el: &Element) -> bool {
    let mut options = Vec::new();
    doc.walk(Some(id), &mut |ev, entry| {
        if let (WalkEvent::Enter(n), NodeKind::Element(e)) = (ev, &entry.kind) {
            if e.name == "option" {
                options.push((n, selected(doc, n, e), e.attr("value").map(str::to_owned)));
            }
        }
    });
    let size = el.attr("size").and_then(|s| s.trim().parse().ok());
    let list = has(el, "multiple") || size > Some(1u32);
    let placeholder = options.first().filter(|(n, _, v)| {
        let text = || doc.text_content(*n).trim().to_owned();
        !list && doc.node(*n).parent == Some(id) && v.clone().unwrap_or_else(text).is_empty()
    });
    let picked = options.iter().find(|o| o.1);
    picked.is_none() || picked == placeholder
}

#[cfg(test)]
mod tests;
