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
//! - Untouched, focus is on the first `autofocus` element in tree order that
//!   can take it ([`focusable`]); a page whose URL names a target element
//!   (`#id`, [`Document::target`]) autofocuses nothing (`bl-5852`).
//!
//! Two limits, each a filed difference rather than a guess: a candidate
//! hidden only by a stylesheet still takes focus here, because matching runs
//! before styles exist; and Chrome flushes autofocus after the parse, so a
//! script running mid-parse sees no focus there while frot's sees it.

use super::selector::ancestors;
use super::state::{disabled, editing, has};
use crate::dom::live::Focus;
use crate::dom::{Document, Element, NodeId, NodeKind, WalkEvent};

/// The focused element, if any (module doc).
pub fn focused_element(doc: &Document) -> Option<NodeId> {
    match doc.focus() {
        Focus::On(id) => doc
            .element(id)
            .filter(|el| connected(doc, id) && (focusable(doc, id, el) || el.name == "dialog"))
            .map(|_| id),
        Focus::Off => None,
        Focus::Auto => autofocused(doc),
    }
}

/// `:focus` / `:focus-visible`: `id` is the focused element.
pub fn focused(doc: &Document, id: NodeId, el: &Element) -> bool {
    match doc.focus() {
        // The cheap test first: only an `autofocus` element can be the one.
        Focus::Auto if !has(el, "autofocus") => false,
        _ => focused_element(doc) == Some(id),
    }
}

/// `:focus-within`: `id` is the focused element or holds it.
pub fn focus_within(doc: &Document, id: NodeId) -> bool {
    let within = |f| std::iter::successors(Some(f), |&n| doc.node(n).parent).any(|n| n == id);
    focused_element(doc).is_some_and(within)
}

/// The first `autofocus` element that can take focus, unless the URL names
/// a target.
fn autofocused(doc: &Document) -> Option<NodeId> {
    if doc.target().is_some() {
        return None;
    }
    let mut first = None;
    doc.walk(None, &mut |ev, entry| {
        if let (WalkEvent::Enter(n), NodeKind::Element(e)) = (ev, &entry.kind) {
            if first.is_none() && has(e, "autofocus") && focusable(doc, n, e) {
                first = Some(n);
            }
        }
    });
    first
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
