//! The plain container an attribute makes a `group` (`bl-0725`).
//!
//! Blink gives an HTML element it has no role for — a `<div>`, a `<span>`,
//! a `<b>` — the role `group` instead of `generic` when it carries
//! `autofocus` or `draggable`, whatever the attribute's value
//! (`autofocus="false"` and `draggable="false"` included). Measured in Chrome
//! 158 (`Accessibility.getFullAXTree`):
//!
//! - the node must be one Chrome keeps at all
//!   ([`lineless::kept`](crate::ax::tree::lineless::kept)): `<span
//!   autofocus>x</span>` is no node, and `<span id=s autofocus>`, `<span
//!   tabindex=0 autofocus>`, `<span autofocus aria-label=N>` (`group "N"`) and
//!   `<div autofocus>text</div>` are groups; a `<div autofocus>` holding only
//!   a block is not;
//! - a `role` attribute, valid or not, turns it off: `<div autofocus
//!   role=bogus>` is `generic`;
//! - an element with a role of its own keeps it (`<em autofocus>` is
//!   `emphasis`), and so do the elements Blink makes `generic` on purpose —
//!   an unnamed `<section>`, an `<a>` without `href`, a `<summary>` outside
//!   `<details>`, `<slot>`, SVG's `<g>`.
//!
//! No other attribute does it: `tabindex`, `contenteditable`, `onclick`,
//! `title`, `lang`, `popover`, `accesskey` and every `aria-*` attribute tried
//! leave a kept span `generic`.

use crate::ax::tree::lineless;
use crate::css::Styles;
use crate::dom::{Document, Element, NodeId};

/// The elements measured to take the rule: those with no role and no
/// handling of their own in Blink.
const PLAIN: &[&str] = &[
    "acronym", "b", "bdi", "bdo", "big", "blink", "center", "cite", "data", "div", "font", "i",
    "kbd", "listing", "nobr", "picture", "pre", "q", "samp", "small", "span", "strike", "tt", "u",
    "var", "xmp",
];

/// `Some("group")` when `el` is a plain container the rule applies to.
pub fn group(
    doc: &Document,
    id: NodeId,
    el: &Element,
    styles: Option<&Styles>,
) -> Option<&'static str> {
    let has = |name: &str| el.attr(name).is_some();
    let marked =
        PLAIN.contains(&el.name.as_str()) && !has("role") && (has("autofocus") || has("draggable"));
    (marked && lineless::kept(doc, id, el, styles)).then_some("group")
}

#[cfg(test)]
mod tests;
