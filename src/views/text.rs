//! `--out text`: source-order text extraction.
//!
//! Rules:
//! - Whitespace collapses per the HTML serializer (runs of whitespace fold
//!   to a single space) **outside** `<pre>`.
//! - `<pre>` content is preserved verbatim; a block-level box inside it
//!   still starts and ends a line.
//! - Subtrees under [`SKIP_TAGS`] are skipped.
//! - The walk descends through [`crate::dom::Document::painted_children`], so
//!   what the *parent's* box swallows never reaches it: a `<video>`'s or
//!   `<canvas>`'s fallback content, and a closed `<details>`'s disclosure body.
//!   Neither is an author declaration, so neither waits for a cascade — they
//!   hold with or without `--css` (`bl-0f83`, `bl-d470`).
//! - Block-level boundaries become newlines; `<br>` becomes a newline. Under
//!   `--css` "block-level" is the computed `display`, so a `display:block`
//!   span breaks the line and a `display:inline` div does not; without it,
//!   the tag's own ([`breaks_line`]). A `display:contents` element and a
//!   table part break none: table parts are separated as Chrome's
//!   anonymous table boxes separate them (`text/state.rs`).
//! - With `--css` (a [`Styles`] table is passed): `display:none` subtrees are
//!   dropped — which is how the author-overridable `[hidden]` rule reaches this
//!   view, and why *that* one does need the recipe — an element's own text is
//!   suppressed when its computed `visibility` is hidden (a
//!   `visibility:visible` descendant reappears), and `::before`/`::after`
//!   generated content is emitted as inline text, and text is in the case
//!   `text-transform` paints it (`bl-e48d`). None of it reaches below a
//!   `<select>`, whose options are listed whatever their own style says.

mod state;

use crate::css::{Display, Styles, Visibility};
use crate::dom::{Document, NodeId, NodeKind};
use state::State;

/// Elements whose text a browser never paints, whatever it is for: source and
/// data (`<script>`, `<style>`, `<template>`, `<noscript>`) and the descriptive
/// metadata SVG and MathML hang off rendered markup (`<title>` — the HTML one
/// in `<head>` and the SVG tooltip alike — `<desc>`, `<metadata>`,
/// `<annotation>`, `<annotation-xml>`).
///
/// `<svg>` and `<math>` were in this set until `bl-c0a4`, which is what made
/// `text` the one frot view that disagreed with the other two *and* with the
/// oracle. Chrome 139 headless at 1280×720 (measured 2026-08-12) puts
/// `SVG_LINK_TEXT` and `MATH_MI` in `body.innerText`, gives each a real
/// `Range.getClientRects()` entry, and exposes `link "SVG_LINK_TEXT"` in
/// `Accessibility.getFullAXTree` — and gives every element listed above zero
/// rects and no `innerText`, which is why the list grew as the blanket skip
/// went. `<foreignObject>` needed nothing: its content is ordinary HTML and
/// reads as such once the blanket skip is gone.
const SKIP_TAGS: &[&str] = &[
    "script",
    "style",
    "template",
    "noscript",
    "title",
    "desc",
    "metadata",
    "annotation",
    "annotation-xml",
];

/// Whether the element starts and ends a line: its box is block-level — the
/// computed `display` under a cascade, the tag's own without one
/// ([`Display::of`]) — or it is a `<p>`. Measured in Chrome 158 `innerText`
/// (`bl-fe2c`): `x<span style="display:block">sb</span>y` is three lines and
/// so is a `display:flex` or `list-item` span, while `x<div
/// style="display:inline">di</div>y` is the one token `xdiy`; an inline-block
/// or inline-flex box stays on its line. A box out of flow is blockified by
/// the cascade, so `Before<span style="position:absolute">tip</span>after`
/// is three lines too (`bl-346e`). The `<p>` is `innerText`'s own rule,
/// keyed on the element rather than its box: `<p style="display:inline">`
/// still stands on a line of its own — unless it has no box (`contents`) or
/// is a table part, which are on no line of their own, `<p>` or not
/// (`bl-e3a0`): `x<p style="display:table-cell">pp</p>y` is `xppy`.
fn breaks_line(styles: Option<&Styles>, id: NodeId, tag: &str) -> bool {
    match Display::of(styles, id, tag) {
        Display::Contents | Display::TableCell | Display::TableRow | Display::TableRowGroup => {
            false
        }
        display => display.block_level() || tag == "p",
    }
}

fn visible(styles: Option<&Styles>, id: NodeId) -> bool {
    styles.is_none_or(|s| s.visibility(id) == Visibility::Visible)
}

fn emit(
    doc: &Document,
    id: NodeId,
    state: &mut State,
    styles: Option<&Styles>,
    parent_visible: bool,
) {
    let entry = doc.node(id);
    match &entry.kind {
        NodeKind::Element(el) => {
            if SKIP_TAGS.contains(&el.name.as_str()) && !Styles::source_shown(styles, id, &el.name)
            {
                return;
            }
            if styles.is_some_and(|s| s.display_none(id)) {
                return;
            }
            if el.name == "br" {
                state.push_newline();
                return;
            }
            let vis = visible(styles, id);
            let block = breaks_line(styles, id, &el.name);
            let display = Display::of(styles, id, &el.name);
            // A `<select>` lists every option, whatever the cascade says of
            // the options themselves; only hiding the select hides them.
            // Measured in Chrome 158 `innerText` (`bl-240d`): `<option
            // hidden>`, `<option style="display:none">`, one that is
            // `visibility:hidden` and one in a hidden `<optgroup>` are all
            // listed, in a menu list and a list box alike, while a
            // `visibility:hidden` select lists none — not even an option
            // made `visible` again. So below a select there is no cascade.
            let select = el.name == "select";
            if select && !vis {
                return;
            }
            let styles = if select { None } else { styles };
            let pre = el.name == "pre";
            state.open(display);
            if block {
                state.block_break();
            }
            if pre {
                state.pre_depth += 1;
            }
            if let Some(s) = styles.filter(|_| vis) {
                state.push_generated(s.before(id), s.before_display(id));
            }
            for c in doc.painted_children(id) {
                emit(doc, c, state, styles, vis);
            }
            if let Some(s) = styles.filter(|_| vis) {
                state.push_generated(s.after(id), s.after_display(id));
            }
            if pre {
                state.pre_depth -= 1;
            }
            if block {
                state.block_break();
            }
            state.close(display);
        }
        // An element the cascade hides returns above before the recursion
        // reaches its children, and a node the *parent's* box swallows never
        // comes out of `painted_children`. What is left is a box that paints
        // none of its content though it is drawn itself — a table column
        // (`bl-9dec`) — which the cascade marks on the text node.
        NodeKind::Text(t) => {
            if parent_visible && !styles.is_some_and(|s| s.display_none(id)) {
                state.push_text(styles.map_or(t, |s| s.text(id, t)));
            }
        }
        NodeKind::Comment(_)
        | NodeKind::Doctype(_)
        | NodeKind::Document
        | NodeKind::Fragment
        | NodeKind::Shadow(_) => {}
    }
}

/// Source-order text extraction. With `Some(styles)`, CSS visibility and
/// generated content are applied; with `None`, raw source-order markup text.
pub fn text(doc: &Document, styles: Option<&Styles>) -> String {
    let mut state = State::new();
    for &root in doc.roots() {
        emit(doc, root, &mut state, styles, true);
    }
    state.finish()
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod display_tests;

#[cfg(test)]
mod foreign_tests;

#[cfg(test)]
mod select_tests;
