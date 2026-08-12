//! Semantic AX subtree exclusion: `aria-hidden` and `inert`.
//!
//! Two author signals remove an element **and its entire subtree** from the
//! accessibility tree without removing it from the page. They are *semantic*
//! rules, not CSS: an `aria-hidden` element still paints, still has boxes, and
//! still contributes to `--out text`/`dom`. Only `--out ax` honours them.
//!
//! - **`aria-hidden`** is an ARIA enumerated `true`/`false` attribute
//!   (ASCII-case-insensitive, matching Chromium's comparison). It defaults to
//!   `false`, so an unparseable value (`aria-hidden="yes"`) is *not* hiding.
//!   `aria-hidden="false"` is likewise not hiding — and, because exclusion is
//!   applied as a subtree cut before descendants are built, it cannot rescue a
//!   node under an `aria-hidden="true"` ancestor. That matches browsers: the
//!   spec-defined "hidden" state is inherited and `false` never un-hides.
//! - **`inert`** is an HTML boolean attribute — presence alone inerts the
//!   subtree, so `inert=""` and even `inert="false"` are inert. Inert content
//!   is not exposed to assistive technology, so it is cut the same way.
//!
//! Two edges are deliberately *not* special-cased, because a subtree cut
//! already gives the browser answer:
//!
//! - **An explicit `role=` inside an excluded ancestor.** Author intent does
//!   not reach outside the cut; the node is gone, role or no role.
//! - **Focusable content under `aria-hidden="true"`.** Chromium's only carve-out
//!   is for the *focused* element, and frot never drives a page — it has no
//!   focus state, so `document.activeElement` is never inside the subtree and
//!   the carve-out cannot fire. The remaining "aria-hidden hides a focusable
//!   descendant" case is an authoring error that browsers report as an issue
//!   while still excluding the node, which is what we do.

use crate::dom::Element;

/// Whether `el` and everything beneath it is excluded from the AX tree.
///
/// Call this *before* building descendants: recursion then delivers inheritance
/// for free, and no descendant — hidden, named, or explicitly roled — can
/// escape an excluded ancestor.
pub fn excluded(el: &Element) -> bool {
    el.attr("aria-hidden")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"))
        || el.attr("inert").is_some()
}
