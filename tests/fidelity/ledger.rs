//! The ledger (`fidelity.md` §3.3): every delta between frot and
//! Chrome-as-recorded that is expected, with the reason it is.
//!
//! A row is an assertion, not an exclusion. The comparator fails on a delta
//! with no row *and* on a row whose delta no longer appears, so nothing here
//! can drift silently: a residual that stops differing, or a defect that gets
//! fixed, turns the gate red until its row is deleted.
//!
//! A delta is written as the comparator prints it — `- x`: Chrome has line
//! `x` and frot lacks it; `+ x`: the reverse — in the terms of the channel's
//! projection (a `text` token, an `ax` outline line, a `bboxes` boxed
//! element as `index tag`).
//!
//! Rule 2 (`fidelity.md` §11) watches this file: if rows grow with the corpus,
//! the projection is misdrawn and is what should change.

/// Why a delta is expected. There is no third kind (F4).
pub enum Why {
    /// frot is right and the channel is blind, or the fact is outside frot's
    /// declared model. The reason, with its source.
    Residual(&'static str),
    /// frot differs from the browser and the question is filed: the ball. The
    /// row pins the divergence until that ball lands. A ledger with no defect
    /// row is the healthy state, so the variant may stand unconstructed.
    #[allow(dead_code)]
    Defect(&'static str),
}

pub struct Row {
    pub fixture: &'static str,
    pub channel: &'static str,
    pub delta: &'static str,
    pub why: Why,
}

const fn row(fixture: &'static str, channel: &'static str, delta: &'static str, why: Why) -> Row {
    Row {
        fixture,
        channel,
        delta,
        why,
    }
}

const COLLAPSED_SELECT: Why = Why::Residual(
    "A collapsed select's options have no client rects in Chrome (the closed popup is a UA \
     widget); frot does not model select modes and lays them out in flow — an over-report of \
     paint that keeps the document's own copy (fidelity.md §3.3, layout.md §2.4).",
);

pub const LEDGER: &[Row] = &[
    // bboxes — where frot boxes what Chrome does not.
    row("forms", "bboxes", "+ 25 option", COLLAPSED_SELECT),
    row("forms", "bboxes", "+ 26 option", COLLAPSED_SELECT),
    row("structure", "bboxes", "+ 35 option", COLLAPSED_SELECT),
    row("structure", "bboxes", "+ 36 option", COLLAPSED_SELECT),
];
