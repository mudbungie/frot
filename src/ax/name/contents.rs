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
//! guard, so a reference pointing back into the subtree terminates instead of
//! looping. It is *not* the rule that stops the walk at an element with its own
//! alternative — that is rule 2F above, and conflating the two is what made an
//! `aria-labelledby` target the one node exempt from 2F (`bl-0482`).
//!
//! Three doors lead in, one recursion behind them ([`contents_name`],
//! [`element_name`], [`referenced_name`]); which one a name source knocks on is
//! the whole of what distinguishes the sources.
//!
//! Spacing: raw text is concatenated verbatim (`<button>Hello<span>world</span>`
//! is "Helloworld" in browsers too), while an alternative is a *word* standing
//! in for an element and is fenced with spaces. The caller collapses runs, so
//! the fences never double up.

mod native;

use crate::ax;
use crate::css::Styles;
use crate::dom::{Document, Element, NodeId, NodeKind};
use native::native_alternative;

/// One traversal's fixed context: the tree, the cascade, and how the traversal
/// reached the node it is reading.
struct Walk<'a> {
    doc: &'a Document,
    styles: Option<&'a Styles>,
    via: Via,
}

/// How a node was reached — the two accname rules that single out an
/// `aria-labelledby` target, and nothing else.
enum Via {
    /// Ordinary descent: a name-from-contents subtree, a `<label>`, a
    /// `<legend>`. Every rule applies as written.
    Contents,
    /// The node a reference names *directly*: exempt from the hidden check
    /// (§2A excludes hidden nodes "unless directly referenced" — measured,
    /// Chrome 139 names a button after a `display:none`, `visibility:hidden`
    /// or `aria-hidden` target), and its own `aria-labelledby` is not followed
    /// (§2B does not recurse: a target with `aria-labelledby` of its own names
    /// from its contents).
    Target,
    /// Below such a node: hidden excludes again (an `aria-hidden` child of a
    /// target is still dropped), while the no-recursion rule holds for the
    /// whole traversal — one reference hop is all there is.
    UnderTarget,
}

impl<'a> Walk<'a> {
    /// The same traversal one step further in. A target's exemption is spent on
    /// the target itself; its refusal to follow references is not.
    fn below(&self) -> Walk<'a> {
        let via = match self.via {
            Via::Contents => Via::Contents,
            _ => Via::UnderTarget,
        };
        Walk {
            doc: self.doc,
            styles: self.styles,
            via,
        }
    }

    /// The traversal that reads a reference's target.
    fn target(&self) -> Walk<'a> {
        Walk {
            doc: self.doc,
            styles: self.styles,
            via: Via::Target,
        }
    }
}

/// The name `id`'s **descendants** compute for it: rule 2F over its children,
/// for a role that names from its contents. `spent` names nodes that must not
/// contribute — the accname rule that a `<label>` does not read back the
/// control it labels, which would otherwise name the control after itself.
pub fn contents_name(
    doc: &Document,
    id: NodeId,
    styles: Option<&Styles>,
    spent: &[NodeId],
) -> String {
    let w = Walk {
        doc,
        styles,
        via: Via::Contents,
    };
    let mut visited = vec![id];
    visited.extend_from_slice(spent);
    let mut out = String::new();
    children_text(&w, id, &mut visited, &mut out);
    out
}

/// The text alternative of the element **itself** — rule 2F applied *to* `id`,
/// so an element that supplies its own alternative contributes that and is not
/// descended into. The door a `<label>` and a `<legend>` enter by: measured,
/// Chrome 139 names a fieldset "LA" for `<legend aria-label=LA>LT</legend>`
/// and a control "LB" for `<label for aria-label=LB>LT</label>`, and names
/// neither when the label or legend is `display:none`.
pub fn element_name(
    doc: &Document,
    id: NodeId,
    styles: Option<&Styles>,
    spent: &[NodeId],
) -> String {
    let w = Walk {
        doc,
        styles,
        via: Via::Contents,
    };
    let mut out = String::new();
    node_text(&w, id, &mut spent.to_vec(), &mut out);
    out
}

/// The text alternative of an `aria-labelledby` target: [`element_name`] under
/// [`Via::Target`], which is the whole difference a reference makes.
pub fn referenced_name(doc: &Document, id: NodeId, styles: Option<&Styles>) -> String {
    let w = Walk {
        doc,
        styles,
        via: Via::Target,
    };
    let mut out = String::new();
    node_text(&w, id, &mut Vec::new(), &mut out);
    out
}

/// `::before` + every child's contribution + `::after`.
fn children_text(w: &Walk, id: NodeId, visited: &mut Vec<NodeId>, out: &mut String) {
    out.push_str(w.styles.and_then(|s| s.before(id)).unwrap_or(""));
    for c in w.doc.ax_children(id) {
        node_text(w, c, visited, out);
    }
    out.push_str(w.styles.and_then(|s| s.after(id)).unwrap_or(""));
}

fn node_text(w: &Walk, id: NodeId, visited: &mut Vec<NodeId>, out: &mut String) {
    match &w.doc.node(id).kind {
        // A text node needs no check of its own: the only ways it can be
        // outside the name are its parent element being hidden (caught below,
        // before the recursion reaches here) and structural concealment
        // (caught by [`Document::ax_children`], which is the only way in).
        NodeKind::Text(t) => out.push_str(t),
        NodeKind::Element(el) => element_text(w, id, el, visited, out),
        NodeKind::Comment(_) | NodeKind::Doctype => {}
    }
}

fn element_text(w: &Walk, id: NodeId, el: &Element, visited: &mut Vec<NodeId>, out: &mut String) {
    let concealed = !matches!(w.via, Via::Target) && hidden(id, el, w.styles);
    if concealed || visited.contains(&id) {
        return;
    }
    // Not popped: once counted, a node is spent for this whole computation.
    visited.push(id);
    let w = &w.below();
    match alternative(w, id, el, visited) {
        Some(text) => {
            out.push(' ');
            out.push_str(&text);
            out.push(' ');
        }
        None => children_text(w, id, visited, out),
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
fn alternative(w: &Walk, id: NodeId, el: &Element, visited: &mut Vec<NodeId>) -> Option<String> {
    super::trimmed_attr(el, "aria-label")
        .or_else(|| labelledby(w, el, visited))
        .or_else(|| native_alternative(w.doc, id, el))
}

/// `aria-labelledby`, resolved through the same recursion so the visited set
/// guards it — but only from outside a reference: one hop is the whole of §2B,
/// so neither a target nor anything under it follows a further reference.
/// A reference that resolves to nothing is no alternative, and the element
/// falls through to its own contents.
fn labelledby(w: &Walk, el: &Element, visited: &mut Vec<NodeId>) -> Option<String> {
    if !matches!(w.via, Via::Contents) {
        return None;
    }
    let raw = el.attr("aria-labelledby")?;
    let w = &w.target();
    let mut out = String::new();
    for target in raw
        .split_whitespace()
        .filter_map(|t| super::find_by_id(w.doc, t))
    {
        node_text(w, target, visited, &mut out);
    }
    Some(out).filter(|s| !s.trim().is_empty())
}
