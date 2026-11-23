//! Where focus is once an unattended page has loaded: on the first
//! `autofocus` element that can take it, else nowhere.
//!
//! Measured in Chrome 158 driven by the fidelity recorder
//! (`tests/fixtures/fidelity/focus.html`): candidates are tried in tree
//! order, and one that cannot take focus is skipped — a disabled or
//! `type=hidden` control, an element that is not focusable at all, one under
//! `hidden` or `inert`, one inside a closed `<details>`. A page whose URL
//! names a target element (`#id`, [`Document::target`]) is scrolled to it and
//! autofocuses nothing (`bl-5852`).
//!
//! Three limits, each a filed difference rather than a guess (`bl-bc6d`):
//! focus a script moves is not seen; a candidate hidden only by a stylesheet
//! still takes focus here, because matching runs before styles exist; and of
//! the elements a browser can focus only form controls, `<a href>`, an
//! integer `tabindex` and `contenteditable` hosts are counted.

use super::selector::ancestors;
use super::state::{disabled, editing, has};
use crate::dom::{Document, Element, NodeId, NodeKind, WalkEvent};

/// `:focus` / `:focus-visible`: `id` is the element focus landed on.
pub fn focused(doc: &Document, id: NodeId, el: &Element) -> bool {
    if !candidate(doc, id, el) || doc.target().is_some() {
        return false;
    }
    let mut first = None;
    doc.walk(None, &mut |ev, entry| {
        if let (WalkEvent::Enter(n), NodeKind::Element(e)) = (ev, &entry.kind) {
            if first.is_none() && candidate(doc, n, e) {
                first = Some(n);
            }
        }
    });
    first == Some(id)
}

/// `:focus-within`: `id` is the focused element or holds it.
pub fn focus_within(doc: &Document, id: NodeId) -> bool {
    let mut within = false;
    doc.walk(Some(id), &mut |ev, entry| {
        if let (WalkEvent::Enter(n), NodeKind::Element(e)) = (ev, &entry.kind) {
            within |= focused(doc, n, e);
        }
    });
    within
}

/// An `autofocus` element that can take focus where it stands.
fn candidate(doc: &Document, id: NodeId, el: &Element) -> bool {
    if !has(el, "autofocus") {
        return false;
    }
    let focusable = match el.name.as_str() {
        "button" | "select" | "textarea" => !disabled(doc, id, el),
        "input" => !el.hidden_input() && !disabled(doc, id, el),
        name => {
            let tabindex = el
                .attr("tabindex")
                .and_then(|t| t.trim().parse::<i32>().ok());
            name == "a" && has(el, "href") || tabindex.is_some() || editing(el) == Some(true)
        }
    };
    let mut chain = std::iter::once((id, el)).chain(ancestors(doc, id));
    focusable && !chain.any(|(n, e)| has(e, "hidden") || has(e, "inert") || doc.concealed(n))
}

#[cfg(test)]
mod tests;
