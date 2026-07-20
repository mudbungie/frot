//! Building the string a `content:` declaration generates.
//!
//! The supported value grammar is whitespace-separated string literals and
//! `attr(name)` references, concatenated; anything else in the list
//! contributes nothing.

use crate::dom::Element;

pub fn string(raw: &str, el: &Element) -> String {
    let mut out = String::new();
    for tok in tokens(raw) {
        let bytes: Vec<char> = tok.chars().collect();
        if matches!(bytes.first(), Some('"') | Some('\'')) {
            out.push_str(&unquote(&bytes));
        } else if let Some(name) = tok.strip_prefix("attr(").and_then(|t| t.strip_suffix(')')) {
            out.push_str(el.attr(name.trim()).unwrap_or(""));
        }
    }
    out
}

fn unquote(b: &[char]) -> String {
    let q = b[0];
    let mut s = String::new();
    let mut i = 1;
    while i < b.len() && b[i] != q {
        if b[i] == '\\' && i + 1 < b.len() {
            i += 1;
        }
        s.push(b[i]);
        i += 1;
    }
    s
}

/// Split a `content` value into top-level tokens (whitespace separated,
/// quotes and parens kept intact).
fn tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut depth = 0u32;
    let mut quote: Option<char> = None;
    let flush = |buf: &mut String, out: &mut Vec<String>| {
        if !buf.is_empty() {
            out.push(std::mem::take(buf));
        }
    };
    for ch in s.chars() {
        if let Some(qc) = quote {
            buf.push(ch);
            if ch == qc {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => {
                quote = Some(ch);
                buf.push(ch);
            }
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
