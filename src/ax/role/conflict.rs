//! When `role=none` / `role=presentation` is overruled (`bl-89a6`).
//!
//! The ARIA presentational-role conflict rule, as Chrome 158 applies it
//! (`Accessibility.getFullAXTree`): the author's `none`/`presentation` is
//! ignored, and the element keeps its *implicit* role — not the next token
//! of its `role` (`<h3 role="none button" aria-label=y>` is `heading "y"`) —
//! when it carries
//!
//! - a global ARIA attribute ([`GLOBAL`]), whatever its value (`aria-label=""`
//!   included); `title`, `aria-hidden`, `aria-disabled`, `aria-invalid`,
//!   `aria-haspopup`, `aria-errormessage` and the widget states do not count;
//! - or focus: an integer `tabindex` (not `tabindex=""` or `tabindex=foo`),
//!   a link, an enabled form control other than a hidden input, or the
//!   summary of a `<details>`. A disabled `<button role=none>` stays out.
//!
//! An element whose implicit role is `generic` gains nothing by it: `<div
//! role=none aria-label=L>` is still no node.

use crate::css::matches_any;
use crate::dom::{Document, Element, NodeId};

/// The global ARIA attributes Chrome counts as a conflict, each measured.
const GLOBAL: &[&str] = &[
    "aria-atomic",
    "aria-braillelabel",
    "aria-brailleroledescription",
    "aria-busy",
    "aria-controls",
    "aria-current",
    "aria-describedby",
    "aria-description",
    "aria-details",
    "aria-flowto",
    "aria-keyshortcuts",
    "aria-label",
    "aria-labelledby",
    "aria-live",
    "aria-owns",
    "aria-relevant",
    "aria-roledescription",
];

/// The natively focusable elements, as a selector over the DOM.
/// A hidden `<input>` is left to [`Element::hidden_input`], whose `type` test
/// ignores case as HTML's does.
const FOCUSABLE: &str = ":any-link, button:enabled, select:enabled, textarea:enabled, \
    input:enabled, details > summary:first-of-type";

/// Whether `el` overrules a presentational role its author gave it.
pub fn overruled(doc: &Document, id: NodeId, el: &Element) -> bool {
    GLOBAL.iter().any(|a| el.attr(a).is_some())
        || el.attr("tabindex").is_some_and(integer)
        || (matches_any(doc, id, FOCUSABLE).unwrap_or(false) && !el.hidden_input())
}

/// The HTML rules for parsing integers, as far as "does it parse": optional
/// whitespace and sign, then a digit.
fn integer(v: &str) -> bool {
    let v = v.trim_start_matches([' ', '\t', '\n', '\r', '\x0c']);
    let v = v.strip_prefix(['-', '+']).unwrap_or(v);
    v.starts_with(|c: char| c.is_ascii_digit())
}
