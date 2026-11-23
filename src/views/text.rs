//! `--out text`: source-order text extraction.
//!
//! Rules:
//! - Whitespace collapses per the HTML serializer (runs of whitespace fold
//!   to a single space) **outside** `<pre>`.
//! - `<pre>` content is preserved verbatim.
//! - Subtrees under [`SKIP_TAGS`] are skipped.
//! - The walk descends through [`crate::dom::Document::painted_children`], so
//!   what the *parent's* box swallows never reaches it: a `<video>`'s or
//!   `<canvas>`'s fallback content, and a closed `<details>`'s disclosure body.
//!   Neither is an author declaration, so neither waits for a cascade — they
//!   hold with or without `--css` (`bl-0f83`, `bl-d470`).
//! - Block-level boundaries become newlines; `<br>` becomes a newline. Under
//!   `--css` "block-level" is the computed `display`, so a `display:block`
//!   span breaks the line and a `display:inline` div does not; without it,
//!   the tag's own ([`breaks_line`]).
//! - With `--css` (a [`Styles`] table is passed): `display:none` subtrees are
//!   dropped — which is how the author-overridable `[hidden]` rule reaches this
//!   view, and why *that* one does need the recipe — an element's own text is
//!   suppressed when its computed `visibility` is hidden (a
//!   `visibility:visible` descendant reappears), and `::before`/`::after`
//!   generated content is emitted as inline text. None of it reaches below a
//!   `<select>`, whose options are listed whatever their own style says.

use crate::css::{Display, Styles, Visibility};
use crate::dom::{Document, NodeId, NodeKind};

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

struct State {
    out: String,
    pre_depth: u32,
    pending_break: bool,
    pending_space: bool,
}

impl State {
    fn new() -> Self {
        Self {
            out: String::new(),
            pre_depth: 0,
            pending_break: true,
            pending_space: false,
        }
    }

    /// Whitespace never needs an "is there anything before me?" guard: the
    /// state starts with a break pending — the start of the document is the
    /// start of a line — so the first text flushes through the break arm of
    /// [`State::flush_pending`], which writes nothing while `out` is empty
    /// and drops any pending space. It does not rest on `<html>` or `<body>`
    /// being block-level, which a stylesheet can undo (`bl-fe2c`).
    fn push_text(&mut self, s: &str) {
        if self.pre_depth > 0 {
            self.flush_pending();
            self.out.push_str(s);
            return;
        }
        for ch in s.chars() {
            if ch.is_whitespace() {
                self.pending_space = true;
            } else {
                self.flush_pending();
                self.out.push(ch);
            }
        }
    }

    /// `::before`/`::after` text: inline, unless the pseudo-element's own box
    /// is block-level — then it stands on a line of its own, as an element
    /// would (`bl-346e`: an absolutely positioned `::after` label).
    fn push_generated(&mut self, text: Option<&str>, display: Display) {
        let Some(text) = text else { return };
        if display.block_level() {
            self.block_break();
        }
        self.push_text(text);
        if display.block_level() {
            self.block_break();
        }
    }

    fn push_newline(&mut self) {
        if self.pre_depth > 0 {
            self.out.push('\n');
            return;
        }
        self.pending_space = false;
        self.pending_break = true;
    }

    fn block_break(&mut self) {
        if self.pre_depth > 0 {
            return;
        }
        self.pending_space = false;
        self.pending_break = true;
    }

    fn flush_pending(&mut self) {
        if self.pending_break {
            if !self.out.is_empty() && !self.out.ends_with('\n') {
                self.out.push('\n');
            }
            self.pending_break = false;
            self.pending_space = false;
            return;
        }
        if self.pending_space {
            self.out.push(' ');
            self.pending_space = false;
        }
    }

    fn finish(mut self) -> String {
        while self.out.ends_with([' ', '\n']) {
            self.out.pop();
        }
        self.out
    }
}

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
/// still stands on a line of its own.
fn breaks_line(styles: Option<&Styles>, id: NodeId, tag: &str) -> bool {
    Display::of(styles, id, tag).block_level() || tag == "p"
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
            if SKIP_TAGS.contains(&el.name.as_str()) {
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
        }
        // A text node needs no gate of its own: an element the cascade hides
        // returns above before the recursion reaches its children, and a node
        // the *parent's* box swallows never comes out of `painted_children`.
        NodeKind::Text(t) => {
            if parent_visible {
                state.push_text(t);
            }
        }
        NodeKind::Comment(_) | NodeKind::Doctype | NodeKind::Document | NodeKind::Fragment => {}
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
mod foreign_tests;

#[cfg(test)]
mod select_tests;
