//! The text view's line state: pending breaks and spaces, `<pre>`, and the
//! separators of table parts an author made of other elements.
//!
//! **Table parts** (`bl-34f1`, Chrome 158 `innerText`, Firefox agreeing but
//! on one case). CSS wraps a run of sibling table parts in anonymous table,
//! row and cell boxes, and `innerText` separates by those: a cell is followed
//! by a tab when another cell follows in its row, a row *element* by a line
//! when another row follows in its table — an anonymous row, holding a bare
//! cell or text, included; nothing else in the run separates anything. So
//! `m1<cell>m2</cell><row>m3</row><cell>m4</cell>m5` is `m1m2m3` / `m4m5`, two
//! row groups of bare text are `kakb`, and the run ends at the first sibling
//! that is no table part, with nothing between. White space beside a run
//! collapses away (the anonymous table is a block): `<p contents>p1</p>
//! <p cell>p2</p>` is `p1p2`, and a cell, white space and an inline-block are
//! one token (`bl-37f5`; Firefox 153 agrees there, not on `p1p2`).

use crate::css::Display;

/// What the last closed table part owes the content after it, if that
/// content is still in its anonymous table.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Debt {
    /// A cell: a tab before the next cell of its row.
    Tab,
    /// A row element: a line before the next row of its table.
    Line,
    /// Nothing, but the run is open.
    Run,
}

pub(super) struct State {
    out: String,
    pub(super) pre_depth: u32,
    pending_break: bool,
    pending_space: bool,
    debt: Option<Debt>,
    /// The `display` of every element open around the current point.
    boxes: Vec<Display>,
}

impl State {
    pub(super) fn new() -> Self {
        Self {
            out: String::new(),
            pre_depth: 0,
            pending_break: true,
            pending_space: false,
            debt: None,
            boxes: Vec::new(),
        }
    }

    /// The start of an element whose box is `display`.
    pub(super) fn open(&mut self, display: Display) {
        match display {
            Display::TableCell | Display::TableRow | Display::TableRowGroup => {
                let debt = self.debt.take();
                self.pending_space = false;
                match (debt, display) {
                    (Some(Debt::Tab), Display::TableCell) => self.pending_space = true,
                    (Some(Debt::Line), Display::TableCell | Display::TableRow) => {
                        self.block_break()
                    }
                    // A row group passes a row's debt to its first row.
                    (Some(Debt::Line), _) => self.debt = debt,
                    _ => {}
                }
            }
            _ => self.content(),
        }
        self.boxes.push(display);
    }

    pub(super) fn close(&mut self, display: Display) {
        self.boxes.pop();
        self.debt = match display {
            Display::TableCell => Some(Debt::Tab),
            Display::TableRow => Some(Debt::Line),
            Display::TableRowGroup if self.debt == Some(Debt::Line) => self.debt,
            Display::TableRowGroup => Some(Debt::Run),
            // A run inside an element ends with it.
            _ => None,
        };
    }

    /// Content that is no table part arrives: inside a row or row group it is
    /// an anonymous cell, which takes the debt; anywhere else it ends the run,
    /// and white space before it goes with the run.
    fn content(&mut self) {
        let Some(debt) = self.debt.take() else { return };
        let in_table = matches!(
            self.boxes.last(),
            Some(Display::TableRow | Display::TableRowGroup)
        );
        match (in_table, debt) {
            (false, _) => self.pending_space = false,
            (true, Debt::Tab) => self.pending_space = true,
            (true, Debt::Line) => self.block_break(),
            (true, Debt::Run) => {}
        }
    }

    /// Whitespace never needs an "is there anything before me?" guard: the
    /// state starts with a break pending — the start of the document is the
    /// start of a line — so the first text flushes through the break arm of
    /// [`State::flush_pending`], which writes nothing while `out` is empty
    /// and drops any pending space. It does not rest on `<html>` or `<body>`
    /// being block-level, which a stylesheet can undo (`bl-fe2c`).
    pub(super) fn push_text(&mut self, s: &str) {
        if self.pre_depth > 0 {
            self.content();
            self.flush_pending();
            self.out.push_str(s);
            return;
        }
        for ch in s.chars() {
            if ch.is_whitespace() {
                self.pending_space = true;
            } else {
                self.content();
                self.flush_pending();
                self.out.push(ch);
            }
        }
    }

    /// `::before`/`::after` text: inline, unless the pseudo-element's own box
    /// is block-level — then it stands on a line of its own, as an element
    /// would (`bl-346e`: an absolutely positioned `::after` label).
    pub(super) fn push_generated(&mut self, text: Option<&str>, display: Display) {
        let Some(text) = text else { return };
        if display.block_level() {
            self.block_break();
        }
        self.push_text(text);
        if display.block_level() {
            self.block_break();
        }
    }

    pub(super) fn push_newline(&mut self) {
        self.content();
        if self.pre_depth > 0 {
            self.out.push('\n');
            return;
        }
        self.pending_space = false;
        self.pending_break = true;
    }

    /// A block boundary, inside `<pre>` as much as outside it: `<pre>one
    /// <div>two</div>three</pre>` is three lines in Chrome 158 (`bl-2f42`).
    /// The break is only pending, so a newline the source already wrote
    /// there is not doubled.
    pub(super) fn block_break(&mut self) {
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

    pub(super) fn finish(mut self) -> String {
        while self.out.ends_with([' ', '\n']) {
            self.out.pop();
        }
        self.out
    }
}
