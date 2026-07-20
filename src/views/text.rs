//! `--out text`: source-order text extraction.
//!
//! Rules:
//! - Whitespace collapses per the HTML serializer (runs of whitespace fold
//!   to a single space) **outside** `<pre>`.
//! - `<pre>` content is preserved verbatim.
//! - Subtrees under `<script>`, `<style>`, `<template>`, `<noscript>`, `<iframe>`,
//!   `<svg>`, and `<math>` are skipped.
//! - Block-level boundaries become newlines; `<br>` becomes a newline.
//! - With `--css` (a [`Styles`] table is passed): `display:none` subtrees are
//!   dropped, an element's own text is suppressed when its computed
//!   `visibility` is hidden (a `visibility:visible` descendant reappears),
//!   and `::before`/`::after` generated content is emitted as inline text.

use crate::css::{Styles, Visibility};
use crate::dom::{Document, NodeId, NodeKind};
use crate::tags::is_block;

const SKIP_TAGS: &[&str] = &[
    "script", "style", "template", "noscript", "iframe", "svg", "math",
];

fn is_skip(name: &str) -> bool {
    SKIP_TAGS.contains(&name)
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
            if parent_visible {
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
