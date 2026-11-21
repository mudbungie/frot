//! SVG roles (`bl-07bd`). Every rule is a Chrome 158 measurement
//! (`Accessibility.getFullAXTree`, `--headless=new`).
//!
//! - **The outermost `<svg>`** is one picture, `img`, unless something inside
//!   it is itself exposed — then it is a `graphics-document` holding those
//!   nodes. "Exposed" is a descendant with a role of its own, or a `<text>`
//!   that has text. `<svg><rect/></svg>`, `<svg><title>t</title></svg>` and
//!   `<svg><text></text></svg>` are `img`; add words to the `<text>`, or a
//!   described `<g>`, and it is a `graphics-document`.
//! - **A part** — a `<g>`, a shape, a `<use>`, a nested `<svg>` — is in the
//!   tree only when the author **described** it: a `<title>` or `<desc>` child
//!   (even an empty one), or an author name (`aria-label`, `aria-labelledby`,
//!   `title`). Undescribed, it has no role and its children stand in for it.
//!
//! The DOM carries no namespace, so the rules are keyed on tag names; all of
//! [`PARTS`] are SVG-only names. The resolver takes no cascade, so content
//! hidden by CSS alone still counts as exposed here.

use super::context;
use crate::ax;
use crate::dom::{Document, Element, NodeId, NodeKind};

/// The role each described part takes.
const PARTS: &[(&str, &str)] = &[
    ("g", "group"),
    ("svg", "group"),
    ("circle", "graphics-symbol"),
    ("ellipse", "graphics-symbol"),
    ("line", "graphics-symbol"),
    ("path", "graphics-symbol"),
    ("polygon", "graphics-symbol"),
    ("polyline", "graphics-symbol"),
    ("rect", "graphics-symbol"),
    ("use", "graphics-object"),
];

/// Whether `name` is an element SVG-AAM names from its `<title>` child: the
/// graphic itself and every part.
pub fn titled(name: &str) -> bool {
    PARTS.iter().any(|(tag, _)| *tag == name)
}

/// The role of `<svg>`: the picture or document when outermost, a part when
/// nested in another.
pub fn svg(doc: &Document, id: NodeId, el: &Element) -> Option<&'static str> {
    if context::has_ancestor(doc, id, &["svg"]) {
        part(doc, id, el)
    } else if exposes(doc, id) {
        Some("graphics-document")
    } else {
        Some("img")
    }
}

/// The role of any other element: a described [`PARTS`] entry's, else none.
pub fn part(doc: &Document, id: NodeId, el: &Element) -> Option<&'static str> {
    let (_, role) = PARTS.iter().find(|(tag, _)| *tag == el.name)?;
    described(doc, id, el).then_some(*role)
}

fn described(doc: &Document, id: NodeId, el: &Element) -> bool {
    let child = |c: &NodeId| matches!(element(doc, *c), Some(e) if is_description(e));
    doc.node(id).children.iter().any(child) || context::author_name(doc, el).is_some()
}

fn is_description(el: &Element) -> bool {
    matches!(el.name.as_str(), "title" | "desc")
}

/// Whether anything under `id` is exposed in its own right.
fn exposes(doc: &Document, id: NodeId) -> bool {
    doc.ax_children(id).any(|c| {
        element(doc, c).is_some_and(|el| {
            !ax::excluded(el)
                && !is_description(el)
                && (has_role(doc, c) || has_text(doc, c, el) || exposes(doc, c))
        })
    })
}

fn has_role(doc: &Document, id: NodeId) -> bool {
    !matches!(
        super::role(doc, id),
        None | Some("generic" | "none" | "presentation")
    )
}

fn has_text(doc: &Document, id: NodeId, el: &Element) -> bool {
    el.name == "text" && !doc.text_content(id).trim().is_empty()
}

fn element(doc: &Document, id: NodeId) -> Option<&Element> {
    match &doc.node(id).kind {
        NodeKind::Element(el) => Some(el),
        _ => None,
    }
}

#[cfg(test)]
#[path = "svg_tests.rs"]
mod tests;
