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
//!   has no node).
//!
//! A line is any inline-level box: text that is not all collapsible white
//! space (a no-break space counts), an inline element even when empty or
//! `visibility:hidden` (`<p><span></span></p>` is a `paragraph`), `<br>`, a
//! replaced element, `::before`/`::after` content. A block *inside* an inline
//! child does not count against the paragraph (`<p><a><span
//! style="display:block">`), and a `display:list-item` paragraph always has
//! its marker.
//!
//! The author can hold the node in place: an explicit `role=paragraph`, or
//! any of the attributes in [`held`], keeps a `<p>` that has no line. An
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

use super::SKIP_TAGS;
use crate::css::{Display, Styles};
use crate::dom::{Attr, Document, Element, NodeId, NodeKind};

/// Whether `el` is a `<p>` or `<caption>` Chrome leaves out of the tree
/// (module docs).
pub fn ignored(doc: &Document, id: NodeId, el: &Element, styles: Option<&Styles>) -> bool {
    if !matches!(el.name.as_str(), "p" | "caption") || el.attrs.iter().any(held) {
        return false;
    }
    match Display::of(styles, id, &el.name) {
        Display::ListItem => false,
        Display::Block | Display::InlineBlock => !has_line(doc, id, styles),
        _ => true,
    }
}

/// The attributes that keep a lineless element in the tree: `role` (an author
/// role is the author's node), `id` — even an empty one — any `aria-*`
/// attribute (`aria-label=""` included), `tabindex`, `contenteditable`, and a
/// `title` that is not empty.
fn held(attr: &Attr) -> bool {
    matches!(
        attr.name.as_str(),
        "role" | "id" | "tabindex" | "contenteditable"
    ) || attr.name.starts_with("aria-")
        || (attr.name == "title" && !attr.value.is_empty())
}

/// Whether the block container `id` holds a line: at least one inline-level
/// box and no block-level one.
fn has_line(doc: &Document, id: NodeId, styles: Option<&Styles>) -> bool {
    let mut line = styles.is_some_and(|s| s.before(id).is_some() || s.after(id).is_some());
    for c in doc.painted_children(id) {
        match inline_level(doc, c, styles) {
            Some(true) => line = true,
            Some(false) => return false,
            None => {}
        }
    }
    line
}

/// Whether the child `id` is an inline-level box (`Some(true)`), a block-level
/// one (`Some(false)`), or no box at all (`None`).
fn inline_level(doc: &Document, id: NodeId, styles: Option<&Styles>) -> Option<bool> {
    match &doc.node(id).kind {
        NodeKind::Text(t) => t
            .contains(|c: char| !c.is_ascii_whitespace())
            .then_some(true),
        NodeKind::Element(el) if !SKIP_TAGS.contains(&el.name.as_str()) => {
            match Display::of(styles, id, &el.name) {
                Display::None => None,
                Display::Inline | Display::InlineBlock | Display::InlineFlex => Some(true),
                _ => Some(false),
            }
        }
        _ => None,
    }
}
