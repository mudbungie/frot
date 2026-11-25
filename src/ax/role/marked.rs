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
//!   `<details>`, SVG's `<g>` (a `<slot>` is no box, `display:contents`,
//!   until a stylesheet gives it one — then it is a group, `bl-4bc5`).
//!
//! `popover` does it too, and with no condition on the box (`bl-cf7b`,
//! Chrome 158): a shown `<div popover>`, an empty `<span popover>` and one
//! made `display:inline` are each `group ""`, and so is a `<div popover>`
//! never shown that a stylesheet displays. A `role` turns it off here as
//! well (`role=none` stays out), an unnamed `<section popover>` stays
//! `generic`, and an element with a role of its own keeps it (`<ul popover>`
//! is a `list`, `<button popover>` a `button`).
//!
//! No other attribute does it: `tabindex`, `contenteditable`, `onclick`,
//! `title`, `lang`, `accesskey` and every `aria-*` attribute tried leave a
//! kept span `generic`. (**Corrected 2026-11-24, `bl-cf7b`:** `popover` was
//! in that list — measured on a popover never shown, which has no box.)

use crate::ax::tree::lineless;
use crate::css::{Display, Styles};
use crate::dom::{Document, Element, NodeId};

/// The HTML elements with no role that still do not take the rule: Blink
/// handles each its own way (`LabelText`, `Abbr`, `Canvas`, `Video`, a
/// plugin, a frame, a ruby part, an inert template, …). Measured in Chrome
/// 158 with `autofocus` and `display:block` on each (`bl-4bc5`). Every other
/// HTML element that reaches the rule takes it — a `<div>`, a `<b>`, an
/// obsolete `<center>`, `<marquee>`, `<keygen>` or `<applet>`, a `<source>`,
/// `<track>`, `<param>` or `<slot>` given a box, a custom element, and an
/// element HTML does not know (`<unknownel>`, Blink's `HTMLUnknownElement`).
/// An SVG or MathML element never does ([`crate::css::foreign`]).
const UNMARKED: &[&str] = &[
    "abbr",
    "audio",
    "base",
    "br",
    "canvas",
    "col",
    "colgroup",
    "embed",
    "fencedframe",
    "frame",
    "frameset",
    "head",
    "html",
    "iframe",
    "label",
    "legend",
    "link",
    "map",
    "meta",
    "noembed",
    "noscript",
    "object",
    "rp",
    "rt",
    "ruby",
    "script",
    "style",
    "template",
    "title",
    "video",
    "wbr",
];

/// `Some("group")` when `el` is a plain container the rule applies to, and
/// `Some("listitem")` when that container's box is a list item: Chrome 158
/// gives any plain `display:list-item` box the role, empty, unmarked, or with
/// `list-style:none`, or under a `role` that is not one (`bl-d61c`) — but
/// not a `<p>`, a heading, a link or a `<section>`, which keep their own. A
/// `display:table-cell` box is no group (`bl-824c`): Blink makes it a cell
/// of an anonymous table; nor is a `table` or `inline-table` box, which is
/// Blink's layout table (`bl-08c6`).
pub fn group(
    doc: &Document,
    id: NodeId,
    el: &Element,
    styles: Option<&Styles>,
) -> Option<&'static str> {
    let has = |name: &str| el.attr(name).is_some();
    if UNMARKED.contains(&el.name.as_str()) || crate::css::foreign(doc, id) {
        return None;
    }
    let display = Display::of(styles, id, &el.name);
    if matches!(display, Display::ListItem | Display::InlineListItem) {
        return Some("listitem");
    }
    let marked = || has("autofocus") || has("draggable");
    let table = matches!(
        display,
        Display::TableCell | Display::Table | Display::InlineTable
    );
    let kept = || !table && lineless::kept(doc, id, el, styles);
    (!has("role") && (has("popover") || marked() && kept())).then_some("group")
}

#[cfg(test)]
mod tests;
