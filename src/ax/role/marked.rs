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
use crate::css::{Display, Styles};
use crate::dom::{Document, Element, NodeId};

/// The elements measured to take the rule: those with no role and no
/// handling of their own in Blink. A custom element — any name with a `-`
/// (`<x-el>`) — takes it too (`bl-824c`).
const PLAIN: &[&str] = &[
    "acronym", "b", "bdi", "bdo", "big", "blink", "center", "cite", "data", "div", "font", "i",
    "kbd", "listing", "nobr", "picture", "pre", "q", "samp", "small", "span", "strike", "tt", "u",
    "var", "xmp",
];

/// `Some("group")` when `el` is a plain container the rule applies to, and
/// `Some("listitem")` when that container's box is a list item: Chrome 158
/// gives any plain `display:list-item` box the role, empty, unmarked, or with
/// `list-style:none`, or under a `role` that is not one (`bl-d61c`) — but
/// not a `<p>`, a heading, a link or a `<section>`, which keep their own. A
/// `display:table-cell` box is no group (`bl-824c`): Blink makes it a cell
/// of an anonymous table.
pub fn group(
    doc: &Document,
    id: NodeId,
    el: &Element,
    styles: Option<&Styles>,
) -> Option<&'static str> {
    let has = |name: &str| el.attr(name).is_some();
    if !PLAIN.contains(&el.name.as_str()) && !el.name.contains('-') {
        return None;
    }
    let display = Display::of(styles, id, &el.name);
    if display == Display::ListItem {
        return Some("listitem");
    }
    let marked = !has("role") && (has("autofocus") || has("draggable"));
    let kept = || display != Display::TableCell && lineless::kept(doc, id, el, styles);
    (marked && kept()).then_some("group")
}

#[cfg(test)]
mod tests;
