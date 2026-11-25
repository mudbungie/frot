//! Where focus is: where a script put it, else — once an unattended page
//! has loaded — on the first `autofocus` element that can take it, else
//! nowhere. One answer for `:focus` and for `document.activeElement`
//! ([`focused_element`]); the state a script sets lives in the arena
//! ([`crate::dom::live`], `bl-bc6d`).
//!
//! Measured in Chrome 158 driven by the fidelity recorder
//! (`tests/fixtures/fidelity/focus.html`, `focusjs.html`):
//!
//! - `el.focus()` from a script moves focus there and the `autofocus` rule is
//!   then skipped; `blur()` leaves nothing focused. Focus on an element that
//!   has since left the document or can no longer take focus is nowhere. A
//!   dialog with nothing focusable inside takes focus itself when shown.
//! - Untouched, focus is nowhere until autofocus is flushed — once, after
//!   `DOMContentLoaded` under `--js` (a script running mid-parse, or a
//!   `DOMContentLoaded` handler, sees `<body>` active), and before the views
//!   otherwise ([`settle_autofocus`], `bl-4248`). The flush puts it on the
//!   first `autofocus` element in tree order that can take it
//!   ([`focusable`]) and is rendered under the styles as they stand with
//!   nothing focused — a candidate a stylesheet sets `display:none` (on it
//!   or an ancestor) or `visibility:hidden` is skipped. A page whose URL
//!   names a target element (`#id`, [`Document::target`]) autofocuses
//!   nothing (`bl-5852`).

use super::selector::ancestors;
use super::state::{disabled, editing, has};
use crate::dom::live::Focus;
use crate::dom::{Document, Element, NodeId, NodeKind, WalkEvent};

/// Flush autofocus (module doc): while no script has placed focus, put it on
/// the first `autofocus` candidate that can take it and is rendered under
/// the author sheets (`Some(external)`, `--css`) or none (`None`). With no
/// such candidate focus stays unplaced, so a later flush may still find one.
pub fn settle_autofocus(doc: &mut Document, external: Option<&[String]>, js: bool) {
    if doc.focus() != Focus::Auto || doc.target().is_some() {
        return;
    }
    let mut candidates = Vec::new();
    doc.walk(None, &mut |ev, entry| {
        if let (WalkEvent::Enter(n), NodeKind::Element(e)) = (ev, &entry.kind) {
            if has(e, "autofocus") && focusable(doc, n, e) {
                candidates.push(n);
            }
        }
    });
    if let Some(id) = super::first_rendered(doc, external, js, candidates) {
        doc.set_focus(Focus::On(id));
    }
}

/// The focused element, if any (module doc).
pub fn focused_element(doc: &Document) -> Option<NodeId> {
    match doc.focus() {
        Focus::On(id) => doc
            .element(id)
            .filter(|el| connected(doc, id) && (focusable(doc, id, el) || el.name == "dialog"))
            .map(|_| id),
        Focus::Off | Focus::Auto => None,
    }
}

/// `:focus` / `:focus-visible`: `id` is the focused element.
pub fn focused(doc: &Document, id: NodeId) -> bool {
    focused_element(doc) == Some(id)
}

/// `:focus-within`: `id` is the focused element or holds it.
pub fn focus_within(doc: &Document, id: NodeId) -> bool {
    let within = |f| std::iter::successors(Some(f), |&n| doc.node(n).parent).any(|n| n == id);
    focused_element(doc).is_some_and(within)
}

/// Whether `el` can take focus where it stands: a button, select or textarea
/// that is not disabled; an input that is not disabled nor `type=hidden`; an
/// `<a>` with `href`; the first `<summary>` of a `<details>`; an
/// `<iframe>`; `<video>` or `<audio>` with `controls`; an element with an
/// integer `tabindex`; a `contenteditable` host — and not under `hidden` or
/// `inert`, nor inside a closed `<details>`.
pub fn focusable(doc: &Document, id: NodeId, el: &Element) -> bool {
    let parent = doc.node(id).parent;
    let kind = match el.name.as_str() {
        "button" | "select" | "textarea" => !disabled(doc, id, el),
        "input" => !el.hidden_input() && !disabled(doc, id, el),
        "a" => has(el, "href"),
        "summary" => parent.is_some_and(|p| {
            doc.element(p).is_some_and(|d| d.name == "details") && doc.first_summary(p) == Some(id)
        }),
        "iframe" => true,
        "video" | "audio" => has(el, "controls"),
        _ => false,
    };
    let tabindex = el
        .attr("tabindex")
        .and_then(|t| t.trim().parse::<i32>().ok());
    let focusable = kind || tabindex.is_some() || editing(el) == Some(true);
    let mut chain = std::iter::once((id, el)).chain(ancestors(doc, id));
    let inert =
        |(n, e): (NodeId, &Element)| has(e, "hidden") || has(e, "inert") || doc.concealed(n);
    focusable && !doc.blocked(id) && !chain.any(inert)
}

/// Whether `id` is in the document: its parent chain ends on the document
/// node.
fn connected(doc: &Document, id: NodeId) -> bool {
    std::iter::successors(Some(id), |&n| doc.node(n).parent).any(|n| n == Document::ROOT)
}

#[cfg(test)]
mod tests;
