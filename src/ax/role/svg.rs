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
//! [`PARTS`] are SVG-only names.
//!
//! "Exposed" reads the cascade when the tree has one (`bl-1c54`): `<svg
//! aria-label=s><text style="display:none">x</text></svg>` is an `img`, and
//! so is one whose `<text>` is `visibility:hidden` — though a `visible`
//! `<tspan>` inside it brings the document back. One Chrome quirk is not
//! followed: it still exposes what is inside a `display:none` `<g>` — a stated
//! residual (`layout.md` §2, `bl-1177`).

use super::context;
use crate::ax;
use crate::css::{Styles, Visibility};
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
    // `<svg><image><title>t</title></image></svg>` is a document holding
    // `img "t"`; undescribed, the image has no node (`bl-1c54`).
    ("image", "img"),
];

/// Whether `id`, a `name` element, is one SVG-AAM names from its `<title>`
/// child: the graphic itself, every part, and the [`TITLED`] text elements —
/// and an `<a>` inside an `<svg>` (an HTML `<a><title>t</title>x</a>` is "x").
pub fn titled(doc: &Document, id: NodeId, name: &str) -> bool {
    PARTS.iter().any(|(tag, _)| *tag == name)
        || TITLED.contains(&name)
        || (name == "a" && context::has_ancestor(doc, id, &["svg"]))
}

/// Elements that are no part of [`PARTS`] and still speak their `<title>`
/// child instead of their contents when one names them or they contribute to
/// a name. Measured in Chrome 158 (`bl-1c54`): `<svg><a href=q><title>at
/// </title><text>la</text></a></svg>` is `link "at"`, and `a<svg><text>
/// <title>tt</title>tx</text></svg>c` names its heading "a tt c".
const TITLED: &[&str] = &["text", "tspan"];

/// The role of `<svg>`: the picture or document when outermost, a part when
/// nested in another.
pub fn svg(
    doc: &Document,
    id: NodeId,
    el: &Element,
    styles: Option<&Styles>,
) -> Option<&'static str> {
    if context::has_ancestor(doc, id, &["svg"]) {
        part(doc, id, el)
    } else if exposes(doc, id, styles) {
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

/// An SVG `<a>` with no `href`: a `group` when described, like a `<g>`
/// (`implicit::link_or_generic`, `bl-b3db`).
pub fn anchor(doc: &Document, id: NodeId, el: &Element) -> &'static str {
    if described(doc, id, el) {
        "group"
    } else {
        "generic"
    }
}

fn described(doc: &Document, id: NodeId, el: &Element) -> bool {
    let child = |c: &NodeId| matches!(element(doc, *c), Some(e) if is_description(e));
    doc.node(id).children.iter().any(child) || context::author_name(doc, el).is_some()
}

fn is_description(el: &Element) -> bool {
    matches!(el.name.as_str(), "title" | "desc")
}

/// Whether anything under `id` is exposed in its own right. `display:none`
/// cuts a subtree; `visibility:hidden` silences the element it is on and no
/// further, since a descendant may be `visible` again.
fn exposes(doc: &Document, id: NodeId, styles: Option<&Styles>) -> bool {
    doc.ax_children(id).any(|c| {
        element(doc, c).is_some_and(|el| {
            let shown = styles.is_none_or(|s| s.visibility(c) == Visibility::Visible);
            !ax::excluded(el)
                && !is_description(el)
                && !styles.is_some_and(|s| s.display_none(c))
                && (shown && (has_role(doc, c) || has_text(doc, c, el)) || exposes(doc, c, styles))
        })
    })
}

fn has_role(doc: &Document, id: NodeId) -> bool {
    !matches!(
        super::role(doc, id),
        None | Some("generic" | "none" | "presentation")
    )
}

/// A `<text>` with words of its own. Its `<title>` counts as words: a titled
/// `<text>` is exposed even when it is otherwise empty. A `<tspan>` that
/// turned `visible` again inside a hidden `<text>` is text too.
fn has_text(doc: &Document, id: NodeId, el: &Element) -> bool {
    matches!(el.name.as_str(), "text" | "tspan") && !doc.text_content(id).trim().is_empty()
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
