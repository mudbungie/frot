//! The block Chrome leaves out: a `<p>` or a table `<caption>` whose box
//! holds no line.
//!
//! Blink exposes a `<p>` as `paragraph` only when its box is a block
//! container *with a line in it*. Measured in Chrome 158
//! (`Accessibility.getFullAXTree`, `--headless=new`; `bl-5c70`), each of these
//! has no `paragraph` node, and whatever it holds sits in its parent instead:
//!
//! - an empty `<p>`, one holding only collapsible white space or a comment,
//!   and one whose every child is `display:none`;
//! - a `<p>` with **any** block-level child — `<p><span style="display:block">
//!   x</span></p>`, and equally `<p>text<span style="display:block">x</span>
//!   </p>`, where the text lands in an anonymous block beside it;
//! - a `<p>` that is not a block container at all: `display:inline`, `flex`
//!   or `inline-flex`, whatever it holds (`<p style="display:inline">text</p>`
//!   has no node) — nor one with no box (`display:contents`) or a table row's.
//!   A `display:table-cell` paragraph is a block container and keeps its
//!   node; a table part or a block *through* a `display:contents` child is a
//!   block-level child like any other (`bl-e3a0`). Nor is a grid or a table
//!   (`display:grid`, `display:table`), whatever it holds; `inline-grid` and
//!   `inline-table` are block containers inside and keep it (`bl-08c6`).
//!
//! An absolutely positioned `<p>` (`position:absolute` or `fixed`) keeps its
//! node whatever it holds — empty, a block, a grid; a floated one does not,
//! and is a block like any other (`bl-08c6`).
//!
//! A line is any inline-level box: text that is not all collapsible white
//! space (a no-break space counts), an inline element even when empty or
//! `visibility:hidden` (`<p><span></span></p>` is a `paragraph`), `<br>`, a
//! replaced element, `::before`/`::after` content that is not itself
//! block-level in flow (`p::after { content: "g"; display: block }` leaves no
//! paragraph; a floated or absolutely positioned one keeps it), a float or an absolutely positioned box. A block *inside* an inline
//! child does not count against the paragraph (`<p><a><span
//! style="display:block">`), and a `display:list-item` paragraph's marker is
//! a line — when it has one: with `list-style-type:none`, inherited too
//! (`css/list.rs`), it is a block like any other (`bl-08c6`).
//!
//! The author can hold the node in place: an explicit `role=paragraph` (a
//! `role` that is no role, `<p role=foo>`, does not: `bl-50ec`), or any of
//! the attributes in [`held`], keeps a `<p>` that has no line. An
//! empty heading, blockquote, list item or `<em>` keeps its node, and a
//! `<div role=paragraph>` is never dropped.
//!
//! A table `<caption>` is the one other element with the rule, case for case
//! (`bl-dfc3`, which moved it here from the role resolver so that it reads
//! the cascade): `<caption></caption>`, `<caption><span style="display:none">
//! h</span></caption>`, `<caption><span style="display:block">x</span>
//! </caption>` and `<caption style="display:inline">x</caption>` have no
//! node, while `<caption><span></span></caption>`, a caption whose only
//! content is `::before` text, and an empty `<caption id=x>` keep theirs. A
//! `<figcaption>` and a `role=caption` do not have it. The table is still
//! named from a caption that has no node.
//!
//! Without a cascade the boxes are the tags' own ([`Display::of`]), so the
//! empty `<p>` goes in every recipe.

use crate::css::{Display, Styles};
use crate::dom::{Attr, Document, Element, NodeId, NodeKind};

/// Whether `el` is a `<p>` or `<caption>` Chrome leaves out of the tree
/// (module docs).
pub fn ignored(doc: &Document, id: NodeId, el: &Element, styles: Option<&Styles>) -> bool {
    matches!(el.name.as_str(), "p" | "caption") && !kept(doc, id, el, styles)
}

/// Whether Chrome keeps a node for an element it would otherwise find
/// uninteresting: the author holds it ([`held`]), or its box is a block
/// container with a line in it. The second half is not a `<span>`'s — Blink
/// never keeps one for its box, even as `display:block` (`bl-0725`).
pub fn kept(doc: &Document, id: NodeId, el: &Element, styles: Option<&Styles>) -> bool {
    let marked = || styles.is_none_or(|s| s.marked(id));
    let line = || match Display::of(styles, id, &el.name) {
        Display::ListItem if marked() => true,
        Display::Block
        | Display::ListItem
        | Display::InlineBlock
        | Display::InlineTable
        | Display::InlineGrid
        | Display::TableCell => has_line(doc, id, styles),
        _ => false,
    };
    let positioned = || styles.is_some_and(|s| s.positioned(id));
    let authored = || crate::ax::role::explicit(doc, id, el).is_some();
    el.attrs.iter().any(held) || authored() || (el.name != "span" && (positioned() || line()))
}

/// The attributes that keep a lineless element in the tree — beside a usable
/// `role`, which is the author's node ([`kept`]): `id` — even an empty one —
/// any `aria-*` attribute (`aria-label=""` included), `tabindex`,
/// `contenteditable`, a `title` that is not empty, and a click handler —
/// `onclick`, `onmousedown` or `onmouseup`, even an empty one, though no other
/// `on*` (`bl-824c`: `<p onclick></p>` is `paragraph ""`, `<p onkeydown></p>`
/// no node).
fn held(attr: &Attr) -> bool {
    matches!(
        attr.name.as_str(),
        "id" | "tabindex" | "contenteditable" | "onclick" | "onmousedown" | "onmouseup"
    ) || attr.name.starts_with("aria-")
        || (attr.name == "title" && !attr.value.is_empty())
}

/// Whether the block container `id` holds a line: at least one inline-level
/// box and no block-level one.
fn has_line(doc: &Document, id: NodeId, styles: Option<&Styles>) -> bool {
    let mut line = false;
    for (display, in_flow) in styles.into_iter().flat_map(|s| s.generated_boxes(id)) {
        if display.block_level() && in_flow {
            return false;
        }
        line = true;
    }
    children_level(doc, id, styles).unwrap_or(line)
}

/// What `id`'s children are between them: inline-level boxes only
/// (`Some(true)`), at least one block-level box (`Some(false)`), or no box
/// (`None`).
fn children_level(doc: &Document, id: NodeId, styles: Option<&Styles>) -> Option<bool> {
    let mut level = None;
    for c in doc.painted_children(id) {
        let child = inline_level(doc, c, styles);
        if child == Some(false) {
            return child;
        }
        level = level.or(child);
    }
    level
}

/// Whether the child `id` is an inline-level box (`Some(true)`), a block-level
/// one (`Some(false)`), or no box at all (`None`).
fn inline_level(doc: &Document, id: NodeId, styles: Option<&Styles>) -> Option<bool> {
    match &doc.node(id).kind {
        NodeKind::Text(t) => t
            .contains(|c: char| !c.is_ascii_whitespace())
            .then_some(true),
        NodeKind::Element(el) if !crate::tags::is_source(&el.name) => {
            // A box out of flow is blockified and still leaves the line it
            // was written in alone: `<p><span style="float:left">f</span>t
            // </p>` is a `paragraph` (`bl-346e`).
            let floats = styles.is_some_and(|s| s.out_of_flow(id));
            match Display::of(styles, id, &el.name) {
                Display::None => None,
                // No box: its children are this container's (`bl-e3a0`).
                Display::Contents => children_level(doc, id, styles),
                // A table part is no line box either: the table Chrome
                // wraps it in is a block.
                display => Some(floats || !display.stacks()),
            }
        }
        _ => None,
    }
}
