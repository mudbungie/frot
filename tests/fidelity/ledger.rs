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

/// Why a delta is expected. There is no third kind (F4). Either kind may have
/// no row at a given moment — an empty ledger is the healthy state — so a
/// variant may stand unconstructed.
#[allow(dead_code)]
pub enum Why {
    /// frot is right and the channel is blind, or the fact is outside frot's
    /// declared model. The reason, with its source.
    Residual(&'static str),
    /// frot differs from the browser and the question is filed: the ball. The
    /// row pins the divergence until that ball lands.
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

/// `text-transform` is not applied.
const TEXT_TRANSFORM: Why = Why::Defect("bl-e48d");

/// The `form=` attribute does not associate a control with its form.
const FORM_ATTR: Why = Why::Defect("bl-e8fa");
/// Chrome makes a span carrying `autofocus` a group.
const AUTOFOCUS_GROUP: Why = Why::Defect("bl-0725");

pub const LEDGER: &[Row] = &[
    // Open divergences, one ball per root cause.
    // text
    row("reach", "text", "- LOUD", TEXT_TRANSFORM),
    row("reach", "text", "- WORDS", TEXT_TRANSFORM),
    row("reach", "text", "+ loud", TEXT_TRANSFORM),
    row("reach", "text", "+ words", TEXT_TRANSFORM),
    row("saved/bootstrap-forms", "text", "- HTML", TEXT_TRANSFORM),
    row("saved/bootstrap-forms", "text", "+ html", TEXT_TRANSFORM),
    row("saved/bootstrap-forms", "text", "- HTML", TEXT_TRANSFORM),
    row("saved/bootstrap-forms", "text", "+ html", TEXT_TRANSFORM),
    row(
        "state",
        "forms",
        "-   input type=\"radio\" name=\"g\"",
        FORM_ATTR,
    ),
    // ax
    row("focus", "ax", "- group \"\"", AUTOFOCUS_GROUP),
    row("focus", "ax", "- group \"\"", AUTOFOCUS_GROUP),
    row("focus", "ax", "- group \"\"", AUTOFOCUS_GROUP),
    row("focus", "ax", "- group \"\"", AUTOFOCUS_GROUP),
    // bboxes
];
