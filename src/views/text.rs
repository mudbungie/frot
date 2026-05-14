//! `--out text`: source-order text extraction.
//!
//! Phase 0 rules:
//! - No visibility filter (that arrives with `--css`).
//! - Whitespace collapses per the HTML serializer (runs of whitespace fold
//!   to a single space) **outside** `<pre>`.
//! - `<pre>` content is preserved verbatim.
//! - Subtrees under `<script>`, `<style>`, `<template>`, `<noscript>`, `<iframe>`,
//!   `<svg>`, and `<math>` are skipped.
//! - Block-level boundaries become newlines; `<br>` becomes a newline.

use crate::dom::{Document, NodeId, NodeKind};

const SKIP_TAGS: &[&str] = &[
    "script", "style", "template", "noscript", "iframe", "svg", "math",
];

const BLOCK_TAGS: &[&str] = &[
    "address", "article", "aside", "blockquote", "dd", "dialog", "div", "dl",
    "dt", "fieldset", "figcaption", "figure", "footer", "form", "h1", "h2",
    "h3", "h4", "h5", "h6", "header", "hr", "li", "main", "nav", "ol", "p",
    "pre", "section", "table", "tr", "td", "th", "ul",
];

fn is_block(name: &str) -> bool {
    BLOCK_TAGS.contains(&name)
}

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

    fn push_text(&mut self, s: &str) {
        if self.pre_depth > 0 {
            self.flush_pending();
            self.out.push_str(s);
            return;
        }
        for ch in s.chars() {
            if ch.is_whitespace() {
                if !self.out.is_empty() || self.pending_break {
                    self.pending_space = true;
                }
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
            if !self.out.is_empty() {
                self.out.push(' ');
            }
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

fn emit(doc: &Document, id: NodeId, state: &mut State) {
    let entry = doc.node(id);
    match &entry.kind {
        NodeKind::Element(el) => {
            if is_skip(&el.name) {
                return;
            }
            if el.name == "br" {
                state.push_newline();
                return;
            }
            let pre = el.name == "pre";
            let block = is_block(&el.name);
            if block {
                state.block_break();
            }
            if pre {
                state.pre_depth += 1;
            }
            for &c in &entry.children {
                emit(doc, c, state);
            }
            if pre {
                state.pre_depth -= 1;
            }
            if block {
                state.block_break();
            }
        }
        NodeKind::Text(t) => state.push_text(t),
        NodeKind::Comment(_) | NodeKind::Doctype => {}
    }
}

/// Source-order text extraction.
pub fn text(doc: &Document) -> String {
    let mut state = State::new();
    for &root in doc.roots() {
        emit(doc, root, &mut state);
    }
    state.finish()
}

#[cfg(test)]
mod tests;
