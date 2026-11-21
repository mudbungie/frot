//! Select modes: whether a `<select>` is a **list box** or a **menu list**
//! (the collapsed drop-down). One fact with two readers — layout, where a menu
//! list's contents generate no box (`layout/select.rs`), and the accessibility
//! role, `listbox` or `combobox` — and so one home.
//!
//! The mode is the element's attributes alone, measured in Chrome 158
//! (`getClientRects()` on every descendant, and the AX role; `bl-2932`):
//!
//! | `multiple` | `size` | mode |
//! |---|---|---|
//! | no | absent, invalid, 0 or 1 | menu list |
//! | no | 2 or more | list box |
//! | yes | exactly 1 | menu list |
//! | yes | anything else | list box |
//!
//! `size` is read by HTML's non-negative-integer rules: leading whitespace and
//! one `+` are skipped and trailing junk ignored (`" 4 "`, `+3` and `2abc` are
//! list boxes), while `x`, `-2`, `1.5` and a value past the integer range are
//! not sizes above 1. Author CSS does not change the mode: `display: block` on
//! the select and its options still gives a menu list's options no rects.

use super::Element;

impl Element {
    /// Whether this `<select>` is a list box — the mode table in the module
    /// docs. Meaningful only for a `<select>`.
    pub fn list_box(&self) -> bool {
        let size = self.attr("size").and_then(size).unwrap_or(0);
        if self.attr("multiple").is_some() {
            size != 1
        } else {
            size > 1
        }
    }
}

/// HTML's "rules for parsing non-negative integers", as far as the mode needs
/// them: a negative or overflowing value is no size, like a non-number.
fn size(raw: &str) -> Option<u32> {
    let raw = raw.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let raw = raw.strip_prefix('+').unwrap_or(raw);
    let digits = raw.len() - raw.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    raw[..digits].parse().ok()
}
