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
            .filter(|el| doc.connected(id) && (focusable(doc, id, el) || el.name == "dialog"))
            .map(|_| id),
        Focus::Off | Focus::Auto => None,
    }
}

/// `:focus` / `:focus-visible`: `id` is the focused element, or a host
/// whose shadow tree holds it (Firefox 153, js.md §15.11).
pub fn focused(doc: &Document, id: NodeId) -> bool {
    focused_element(doc).is_some_and(|f| f == id || inside(doc, id, f, false))
}

/// `:focus-within`: `id` is the focused element or holds it, through shadow
/// roots to their hosts.
pub fn focus_within(doc: &Document, id: NodeId) -> bool {
    focused_element(doc).is_some_and(|f| outward(doc, f).any(|n| n == id))
}

/// `n`, then each node holding it: a parent, or a shadow root's host.
fn outward(doc: &Document, n: NodeId) -> impl Iterator<Item = NodeId> + '_ {
    std::iter::successors(Some(n), |&n| {
        doc.node(n).parent.or_else(|| doc.shadow(n).map(|s| s.host))
    })
}

/// Whether focus at `f` is inside the shadow tree of `host` — one whose
/// shadow root delegates focus, when `delegating`.
fn inside(doc: &Document, host: NodeId, f: NodeId, delegating: bool) -> bool {
    let root = doc.shadow_root(host);
    let delegates = root
        .and_then(|r| doc.shadow(r))
        .is_some_and(|s| s.delegates_focus);
    (delegates || !delegating) && outward(doc, f).any(|n| Some(n) == root)
}

/// What `id.blur()` takes focus from: the focused element when it is `id`,
/// or inside the shadow tree of `id` whose root delegates focus.
pub fn blur_target(doc: &Document, id: NodeId) -> Option<NodeId> {
    focused_element(doc).filter(|&f| f == id || inside(doc, id, f, true))
}

/// Where `id.focus()` puts focus (HTML "focus delegate"; Firefox 153, js.md
/// §15.11): for a host whose shadow root delegates focus, where focus already
/// is when that is inside it, else its first `autofocus` element that can
/// take focus, else its first that can, in tree order; for any other
/// element, itself if it can take focus.
pub fn focus_target(doc: &Document, id: NodeId) -> Option<NodeId> {
    let el = doc.element(id)?;
    let root = doc
        .shadow_root(id)
        .filter(|&r| doc.shadow(r).is_some_and(|s| s.delegates_focus));
    let Some(root) = root else {
        return focusable(doc, id, el).then_some(id);
    };
    let now = focused_element(doc).filter(|&f| inside(doc, id, f, true));
    let mut cands = Vec::new();
    doc.walk(Some(root), &mut |ev, entry| {
        if let (WalkEvent::Enter(n), NodeKind::Element(e)) = (ev, &entry.kind) {
            if focusable(doc, n, e) {
                cands.push((n, has(e, "autofocus")));
            }
        }
    });
    let auto = cands.iter().find(|c| c.1).or(cands.first());
    now.or(auto.map(|c| c.0))
}

/// `activeElement` of the document or shadow root `root`: the focused
/// element, or the host in `root`'s tree whose shadow tree holds it (DOM
/// "retarget").
pub fn active_in(doc: &Document, root: NodeId) -> Option<NodeId> {
    let f = focused_element(doc)?;
    let mut n = f;
    loop {
        let top = doc.tree_root(n);
        if top == root {
            return Some(n);
        }
        n = doc.shadow(top)?.host;
    }
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

#[cfg(test)]
mod tests;
