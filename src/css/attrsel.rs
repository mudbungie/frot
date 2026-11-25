//! Attribute selectors (Selectors 4 §6, `bl-1519`): `[a]`, and `[a OP v]`
//! with OP one of `=` `~=` `|=` `^=` `$=` `*=`, and an optional `i` (ASCII
//! case-insensitive) or `s` (case-sensitive) flag before the `]`.

use super::selector::Simple;
use super::selparse::{ident, skip_ws};

/// How an attribute's value is compared with the selector's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttrOp {
    /// `=`: the whole value.
    Equals,
    /// `~=`: one of its whitespace-separated words.
    Includes,
    /// `|=`: the whole value, or its prefix before a `-`.
    DashMatch,
    /// `^=`: a prefix.
    Prefix,
    /// `$=`: a suffix.
    Suffix,
    /// `*=`: a substring.
    Substring,
}

/// The value half of `[a OP v]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttrTest {
    pub op: AttrOp,
    pub val: String,
    /// The `i` flag: compare ASCII case-insensitively.
    pub fold: bool,
}

impl AttrTest {
    /// Whether the attribute value `v` passes. An empty selector value never
    /// matches `~=`, `^=`, `$=` or `*=` (Selectors 4 §6.1–6.2), and a `~=`
    /// value holding whitespace names no single word, so it never matches.
    pub fn holds(&self, v: &str) -> bool {
        let (v, want) = if self.fold {
            (v.to_ascii_lowercase(), self.val.to_ascii_lowercase())
        } else {
            (v.to_string(), self.val.clone())
        };
        let some = !want.is_empty();
        match self.op {
            AttrOp::Equals => v == want,
            AttrOp::Includes => {
                some && !want.contains(char::is_whitespace)
                    && v.split_ascii_whitespace().any(|w| w == want)
            }
            AttrOp::DashMatch => v == want || v.starts_with(&format!("{want}-")),
            AttrOp::Prefix => some && v.starts_with(&want),
            AttrOp::Suffix => some && v.ends_with(&want),
            AttrOp::Substring => some && v.contains(&want),
        }
    }
}

/// Parse an attribute selector from just after its `[`, returning the simple
/// and the index just past its `]`; `None` when it is malformed.
pub(super) fn parse_attr(b: &[char], start: usize) -> Option<(Simple, usize)> {
    let (name, i) = ident(b, start);
    if name.is_empty() {
        return None;
    }
    let name = name.to_ascii_lowercase();
    let mut i = skip_ws(b, i);
    if b.get(i) == Some(&']') {
        return Some((Simple::Attr { name, test: None }, i + 1));
    }
    let (op, n) = operator(b, i)?;
    i = skip_ws(b, n);
    let val = if matches!(b.get(i), Some('"') | Some('\'')) {
        let q = b[i];
        let s: String = b[i + 1..].iter().take_while(|&&c| c != q).collect();
        i += s.chars().count() + 1;
        b.get(i)?; // the closing quote
        i += 1;
        s
    } else {
        let (v, n) = ident(b, i);
        i = n;
        v
    };
    i = skip_ws(b, i);
    let (flag, n) = ident(b, i);
    let fold = match flag.to_ascii_lowercase().as_str() {
        "" => false,
        "i" => true,
        "s" => false,
        _ => return None,
    };
    i = skip_ws(b, n);
    if b.get(i) != Some(&']') {
        return None;
    }
    let test = AttrTest { op, val, fold };
    Some((
        Simple::Attr {
            name,
            test: Some(test),
        },
        i + 1,
    ))
}

/// The operator at `i`, and the index past it.
fn operator(b: &[char], i: usize) -> Option<(AttrOp, usize)> {
    let op = match b.get(i)? {
        '=' => return Some((AttrOp::Equals, i + 1)),
        '~' => AttrOp::Includes,
        '|' => AttrOp::DashMatch,
        '^' => AttrOp::Prefix,
        '$' => AttrOp::Suffix,
        '*' => AttrOp::Substring,
        _ => return None,
    };
    (b.get(i + 1) == Some(&'=')).then_some((op, i + 2))
}

#[cfg(test)]
mod tests;
