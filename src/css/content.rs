//! Building the string a `content:` declaration generates.
//!
//! The supported value grammar is whitespace-separated string literals and
//! `attr(name)` references, concatenated; anything else in the list
//! contributes nothing. The literals' CSS escapes — `\e609`, `\"`, `\\`, and
//! the rest — are decoded by [`super::escape`], so what reaches text/AX/layout
//! is the character the author wrote, never its source spelling.

use super::escape::{unquote, Quoting};
use crate::dom::Element;

pub fn string(raw: &str, el: &Element) -> String {
    let mut out = String::new();
    for tok in tokens(raw) {
        if tok.starts_with(['"', '\'']) {
            out.push_str(&unquote(&tok));
        } else if let Some(name) = tok.strip_prefix("attr(").and_then(|t| t.strip_suffix(')')) {
            out.push_str(el.attr(name.trim()).unwrap_or(""));
        }
    }
    out
}

/// Split a `content` value into top-level tokens (whitespace separated,
/// quotes and parens kept intact). String literals are opaque here — an
/// escaped quote inside one does not end it ([`Quoting`]).
fn tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut depth = 0u32;
    let mut quoting = Quoting::default();
    let flush = |buf: &mut String, out: &mut Vec<String>| {
        if !buf.is_empty() {
            out.push(std::mem::take(buf));
        }
    };
    for ch in s.chars() {
        if quoting.feed(ch) {
            buf.push(ch);
            continue;
        }
        match ch {
            '(' => {
                depth += 1;
                buf.push(ch);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                buf.push(ch);
            }
            c if c.is_whitespace() && depth == 0 => flush(&mut buf, &mut out),
            _ => buf.push(ch),
        }
    }
    flush(&mut buf, &mut out);
    out
}

#[cfg(test)]
mod tests;
