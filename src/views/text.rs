//! `--out text`: source-order text extraction.
//!
//! Rules:
//! - Whitespace collapses per the HTML serializer (runs of whitespace fold
//!   to a single space) **outside** `<pre>`.
//! - `<pre>` content is preserved verbatim.
//! - Subtrees under [`SKIP_TAGS`] are skipped, as is fallback content — the
//!   children a UA that implements the element paints nowhere
//!   ([`crate::tags::renders_children`]: `<video>`, `<audio>`, `<iframe>`,
//!   `<canvas>`). That is a content-model fact, so it holds with or without
//!   `--css`.
//! - Block-level boundaries become newlines; `<br>` becomes a newline.
//! - With `--css` (a [`Styles`] table is passed): `display:none` nodes are
//!   dropped with their subtrees — text nodes included, which is how a closed
//!   `<details>`'s raw disclosure text goes with its elements
//!   ([`crate::dom::Document::concealed`]) — an element's own text is
//!   suppressed when its computed `visibility` is hidden (a
//!   `visibility:visible` descendant reappears), and `::before`/`::after`
//!   generated content is emitted as inline text.

use crate::css::{Styles, Visibility};
use crate::dom::{Document, NodeId, NodeKind};
use crate::tags::{is_block, renders_children};

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

/// Whether this element contributes no text at all. Two sources, both
/// recipe-independent: this view's own skip set above, and the shared
/// fallback-content fact ([`renders_children`] — a `<video>`'s or `<canvas>`'s
/// contents are markup for a UA that cannot paint the box, and its own text is
/// nothing). `<iframe>` lives in the shared fact rather than the local set, so
/// geometry and the AX tree cut it at the same place this view does.
fn is_skip(name: &str) -> bool {
    SKIP_TAGS.contains(&name) || !renders_children(name)
}

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
            pending_break: false,
            pending_space: false,
        }
    }

    /// Whitespace never needs an "is there anything before me?" guard, because
    /// there always is: [`emit`] calls [`State::block_break`] on entering any
    /// block element *before* pushing any text, `Document::parse` always yields
    /// an `html` root, and `tags::BLOCK_TAGS` holds both `html` and `body`. So
    /// `pending_break` is set before the first `push_text` runs, and
    /// [`State::flush_pending`] takes the break arm — which returns early —
    /// while `out` is still empty. A pending *space* therefore only ever
    /// survives to be flushed once `out` is non-empty, and `out` never shrinks
    /// before `finish`. `text_never_starts_with_whitespace` pins the tag
    /// membership this rests on.
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
            if is_skip(&el.name) {
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
            let pre = el.name == "pre";
            let block = is_block(&el.name);
            if block {
                state.block_break();
            }
            if pre {
                state.pre_depth += 1;
            }
            if vis {
                if let Some(b) = styles.and_then(|s| s.before(id)) {
                    state.push_text(b);
                }
            }
            for &c in &entry.children {
                emit(doc, c, state, styles, vis);
            }
            if vis {
                if let Some(a) = styles.and_then(|s| s.after(id)) {
                    state.push_text(a);
                }
            }
            if pre {
                state.pre_depth -= 1;
            }
            if block {
                state.block_break();
            }
        }
        NodeKind::Text(t) => {
            if parent_visible && !styles.is_some_and(|s| s.display_none(id)) {
                state.push_text(t);
            }
        }
        NodeKind::Comment(_) | NodeKind::Doctype => {}
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
