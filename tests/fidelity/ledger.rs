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
//! projection (a `text` token, an `ax` outline line, a `bboxes` painted
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

const PSEUDO: Why = Why::Residual(
    "String ::before/::after content is painted (Chrome computes it and generates the box) but \
     `innerText` never reports it: the channel under-reports paint. frot's inclusion is the \
     --css contract (VISION.md, docs/design/css.md; fidelity.md §3.3, §12.1).",
);
const ICON: Why = Why::Residual(
    "A private-use icon glyph set by ::before: frot emits the raw codepoint, `innerText` \
     nothing. The same blindness as any generated content (fidelity.md §3.3).",
);
const TEXTAREA: Why = Why::Residual(
    "A <textarea>'s default value is painted in the control, but `innerText` omits a form \
     control's value: the channel under-reports paint. frot keeps the document's own copy \
     (measured 2026-11-18, Chrome 156).",
);
const COLLAPSED_SELECT: Why = Why::Residual(
    "A collapsed select's options have no client rects in Chrome (the closed popup is a UA \
     widget); frot does not model select modes and lays them out in flow — an over-report of \
     paint that keeps the document's own copy (fidelity.md §3.3, layout.md §2.4).",
);
const CLOSED_DETAILS: Why = Why::Residual(
    "A closed <details> hides its body with `content-visibility: hidden`, which skips paint but \
     keeps the boxes, so `getClientRects()` still answers for content nothing paints: here the \
     channel over-reports paint. frot's omission is the painted truth — the same body is absent \
     from Chrome's own innerText and AX tree (measured 2026-11-18, Chrome 156).",
);
const UNNAMED_FORM: Why = Why::Defect("bl-2a3c");
const FLEX_ORDER: Why = Why::Defect("bl-7430");

pub const LEDGER: &[Row] = &[
    // text — generated content and control values `innerText` cannot see.
    row("article", "text", "+ By", PSEUDO),
    row("generated", "text", "+ NEW:", PSEUDO),
    row("generated", "text", "+ (done)", PSEUDO),
    row("generated", "text", "+ NEW:", PSEUDO),
    row("generated", "text", "+ \u{e609}", ICON),
    row("forms", "text", "+ draft", TEXTAREA),
    row("forms", "text", "+ text", TEXTAREA),
    // bboxes — where `getClientRects()` is not paintedness.
    row("forms", "bboxes", "+ 25 option", COLLAPSED_SELECT),
    row("forms", "bboxes", "+ 26 option", COLLAPSED_SELECT),
    row("structure", "bboxes", "+ 35 option", COLLAPSED_SELECT),
    row("structure", "bboxes", "+ 36 option", COLLAPSED_SELECT),
    row("disclosure", "bboxes", "- 8 p", CLOSED_DETAILS),
    row("disclosure", "bboxes", "- 9 a", CLOSED_DETAILS),
    // ax — open divergences, one ball each.
    row("forms", "ax", "- form \"\"", UNNAMED_FORM),
    row("forms", "ax", "- (", UNNAMED_FORM),
    row("forms", "ax", "- )", UNNAMED_FORM),
    row("forms", "ax", "- form \"\"", UNNAMED_FORM),
    row("forms", "ax", "- (", UNNAMED_FORM),
    row("forms", "ax", "- )", UNNAMED_FORM),
    row("flex", "ax", "- link \"source first\"", FLEX_ORDER),
    row("flex", "ax", "+ link \"source first\"", FLEX_ORDER),
    row("flex", "ax", "- button \"one\"", FLEX_ORDER),
    row("flex", "ax", "- button \"two\"", FLEX_ORDER),
    row("flex", "ax", "+ button \"two\"", FLEX_ORDER),
    row("flex", "ax", "+ button \"one\"", FLEX_ORDER),
];
