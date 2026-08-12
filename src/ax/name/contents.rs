//! Name from contents — the accname §2F recursion.
//!
//! A role that names from its contents does *not* name from its raw text: the
//! algorithm walks the descendants and takes each one's **text alternative**,
//! which for a replaced element is `alt`/`<title>`/a control's value, not the
//! (empty) text it contains. That is why Chrome names the Wikipedia logo link
//! "Wikipedia The Free Encyclopedia" from two `<img alt>` children with no DOM
//! text between them, and why the W3C search button is named from its graphic.
//!
//! Per node, in tree order:
//!
//! - **Text** — its characters, verbatim, unless CSS hid it.
//! - **An element that supplies its own alternative** — `aria-label`,
//!   `aria-labelledby`, `alt`, an SVG `<title>`, or an embedded control's value
//!   — contributes *that* and is not descended into.
//! - **Any other element** — its `::before`, its children (recursively, whatever
//!   its own role), then its `::after`. Rule 2F applies to every descendant in
//!   the recursion, not only to name-from-contents roles.
//! - **Excluded subtrees** — `aria-hidden`/`inert` ([`crate::ax::hidden`]) and
//!   anything CSS made invisible — contribute nothing, matching what a screen
//!   reader would read.
//!
//! The recursion descends through [`Document::ax_children`], never through the
//! raw child list, so a node the AX *tree* does not contain cannot name it
//! either: a `<video>`'s fallback prose is not the name of the `<a>` around it,
//! in any recipe, and a closed `<details>`'s body is not the name of its
//! heading (`bl-0aaf`). That is one accessor rather than two walks each
//! remembering to ask the same question.
//!
//! Each node contributes **at most once**: the visited set is the spec's cycle
//! guard, so `aria-labelledby` pointing back into the subtree (or at itself)
//! terminates instead of looping.
//!
//! Spacing: raw text is concatenated verbatim (`<button>Hello<span>world</span>`
//! is "Helloworld" in browsers too), while an alternative is a *word* standing
//! in for an element and is fenced with spaces. The caller collapses runs, so
//! the fences never double up.

use crate::ax;
use crate::css::Styles;
use crate::dom::{Document, Element, NodeId, NodeKind, WalkEvent};

/// The name `id`'s descendants compute for it. `spent` names nodes that must
/// not contribute — the accname rule that a `<label>` does not read back the
/// control it labels, which would otherwise name the control after itself.
pub fn contents_name(
    doc: &Document,
    id: NodeId,
    styles: Option<&Styles>,
    spent: &[NodeId],
) -> String {
    let mut visited = vec![id];
    visited.extend_from_slice(spent);
    let mut out = String::new();
    children_text(doc, id, styles, &mut visited, &mut out);
    out
}

/// `::before` + every child's contribution + `::after`.
fn children_text(
    doc: &Document,
    id: NodeId,
    styles: Option<&Styles>,
    visited: &mut Vec<NodeId>,
    out: &mut String,
) {
    out.push_str(styles.and_then(|s| s.before(id)).unwrap_or(""));
    for c in doc.ax_children(id) {
        node_text(doc, c, styles, visited, out);
    }
    out.push_str(styles.and_then(|s| s.after(id)).unwrap_or(""));
}

fn node_text(
    doc: &Document,
    id: NodeId,
    styles: Option<&Styles>,
    visited: &mut Vec<NodeId>,
    out: &mut String,
) {
    match &doc.node(id).kind {
        // A text node needs no check of its own: the only ways it can be
        // outside the name are its parent element being hidden (caught below,
        // before the recursion reaches here) and structural concealment
        // (caught by [`Document::ax_children`], which is the only way in).
        NodeKind::Text(t) => out.push_str(t),
        NodeKind::Element(el) => element_text(doc, id, el, styles, visited, out),
        NodeKind::Comment(_) | NodeKind::Doctype => {}
    }
}

fn element_text(
    doc: &Document,
    id: NodeId,
    el: &Element,
    styles: Option<&Styles>,
    visited: &mut Vec<NodeId>,
    out: &mut String,
) {
    if hidden(id, el, styles) || visited.contains(&id) {
        return;
    }
    // Not popped: once counted, a node is spent for this whole computation.
    visited.push(id);
    match alternative(doc, id, el, styles, visited) {
        Some(text) => {
            out.push(' ');
            out.push_str(&text);
            out.push(' ');
        }
        None => children_text(doc, id, styles, visited, out),
    }
}

/// Whether the element is outside the accessible name: semantically excluded,
/// or invisible. `visibility:hidden` counts — an invisible label is not a label
/// — even though it leaves its box in place for geometry.
fn hidden(id: NodeId, el: &Element, styles: Option<&Styles>) -> bool {
    ax::excluded(el)
        || styles.is_some_and(|s| {
            s.display_none(id) || s.visibility(id) == crate::css::Visibility::Hidden
        })
}

/// What an element contributes *instead of* its contents, if anything.
fn alternative(
    doc: &Document,
    id: NodeId,
    el: &Element,
    styles: Option<&Styles>,
    visited: &mut Vec<NodeId>,
) -> Option<String> {
    super::trimmed_attr(el, "aria-label")
        .or_else(|| labelledby(doc, el, styles, visited))
        .or_else(|| native_alternative(doc, id, el))
}

/// `aria-labelledby` on a *descendant*, resolved through the same recursion so
/// the visited set guards it. A reference that resolves to nothing is no
/// alternative, and the element falls through to its own contents.
fn labelledby(
    doc: &Document,
    el: &Element,
    styles: Option<&Styles>,
    visited: &mut Vec<NodeId>,
) -> Option<String> {
    let raw = el.attr("aria-labelledby")?;
    let mut out = String::new();
    for target in raw
        .split_whitespace()
        .filter_map(|t| super::find_by_id(doc, t))
    {
        node_text(doc, target, styles, visited, &mut out);
    }
    Some(out).filter(|s| !s.trim().is_empty())
}

/// The replaced-element and embedded-control alternatives. `alt=""` is an
/// alternative — the empty one — which is exactly how a decorative image
/// contributes nothing without its `src` leaking in.
fn native_alternative(doc: &Document, id: NodeId, el: &Element) -> Option<String> {
    match el.name.as_str() {
        "img" | "area" => el.attr("alt").map(str::to_string),
        "input" => Some(input_alternative(el)),
        "textarea" => Some(doc.text_content(id)),
        "select" => Some(selected_option_text(doc, id)),
        "svg" => svg_title_text(doc, id),
        _ => None,
    }
}

/// An `<input>` is a leaf: `type=image` speaks through `alt`, everything else
/// (button labels and embedded controls alike) through its `value`.
fn input_alternative(el: &Element) -> String {
    let attr = match el.attr("type").unwrap_or("").to_ascii_lowercase().as_str() {
        "image" => "alt",
        _ => "value",
    };
    el.attr(attr).unwrap_or("").to_string()
}

/// An embedded `<select>` speaks its selected option — the first one when the
/// author marked none, since that is the one the UA selects.
fn selected_option_text(doc: &Document, id: NodeId) -> String {
    let (mut first, mut selected) = (None, None);
    doc.walk(Some(id), &mut |ev, e| {
        if let (WalkEvent::Enter(n), NodeKind::Element(el)) = (ev, &e.kind) {
            if el.name == "option" {
                first = first.or(Some(n));
                if selected.is_none() && el.attr("selected").is_some() {
                    selected = Some(n);
                }
            }
        }
    });
    selected
        .or(first)
        .map(|n| doc.text_content(n))
        .unwrap_or_default()
}

/// SVG-AAM: an `<svg>` is named by its `<title>` child.
fn svg_title_text(doc: &Document, id: NodeId) -> Option<String> {
    let mut found = None;
    doc.walk(Some(id), &mut |ev, e| {
        if let (WalkEvent::Enter(n), NodeKind::Element(el)) = (ev, &e.kind) {
            if found.is_none() && el.name == "title" {
                found = Some(n);
            }
        }
    });
    found.map(|t| doc.text_content(t))
}
