//! Media-query evaluation against the fixed viewport (`layout.rs`
//! `VIEWPORT_WIDTH`/`VIEWPORT_HEIGHT`).
//!
//! The single authority for media-query semantics: `@media` preludes in the
//! cascade ([`super::parse`]) and JS `matchMedia` (via the
//! `__frot_media_matches` syscall) both evaluate here, so CSS and JS can
//! never disagree.
//!
//! Supported: media types `all`/`screen` (match) and `print` (does not);
//! `min-`/`max-`/bare `width` and `height` features with `px`/`em`/`rem`
//! values (`em`/`rem` at a 16px root font-size, so `80em == 1280px`); the
//! `and` combinator, comma (OR over the query list), `not` (whole-query
//! negation), and the no-op `only`. Anything unknown — feature, media type,
//! unit, or grammar (`or`, range syntax, boolean features) — makes its query
//! not match, even under `not`: the spec's "malformed is `not all`",
//! conservative.

use super::selparse::split_top;
use crate::layout::{VIEWPORT_HEIGHT, VIEWPORT_WIDTH};

/// Whether a media-query list matches the fixed viewport. Comma is OR; an
/// empty list means `all` (matches).
pub fn matches(list: &str) -> bool {
    if list.trim().is_empty() {
        return true;
    }
    split_top(list, ',')
        .iter()
        .any(|q| eval_query(q) == Some(true))
}

enum Tok {
    Ident(String),
    Cond(String),
}

/// One media query: `[only|not]? <type-or-cond> [and <cond>]*`. `None` is
/// malformed/unknown — it never matches, even under `not`.
fn eval_query(q: &str) -> Option<bool> {
    let toks = tokenize(q)?;
    let mut i = 0;
    let mut negate = false;
    match toks.first() {
        Some(Tok::Ident(w)) if w == "only" => i = 1,
        Some(Tok::Ident(w)) if w == "not" => {
            negate = true;
            i = 1;
        }
        _ => {}
    }
    let mut pass = match toks.get(i)? {
        Tok::Ident(t) => match t.as_str() {
            "all" | "screen" => true,
            "print" => false,
            _ => return None,
        },
        Tok::Cond(c) => eval_cond(c)?,
    };
    while i + 1 < toks.len() {
        match &toks[i + 1] {
            Tok::Ident(w) if w == "and" => {}
            _ => return None,
        }
        // Evaluate before AND-folding so an unknown condition poisons the
        // whole query (spec: `not all`) even when `pass` is already false.
        match toks.get(i + 2)? {
            Tok::Cond(c) => pass = eval_cond(c)? && pass,
            Tok::Ident(_) => return None,
        }
        i += 2;
    }
    Some(pass != negate)
}

/// Lex a query into lowercased idents and parenthesized conditions (the text
/// inside the parens). `None` on an unbalanced `(` or a stray `)`.
fn tokenize(q: &str) -> Option<Vec<Tok>> {
    let b: Vec<char> = q.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_whitespace() {
            i += 1;
        } else if b[i] == ')' {
            return None;
        } else if b[i] == '(' {
            let start = i + 1;
            let mut depth = 1u32;
            i += 1;
            while i < b.len() && depth > 0 {
                match b[i] {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    _ => {}
                }
                i += 1;
            }
            if depth > 0 {
                return None;
            }
            toks.push(Tok::Cond(b[start..i - 1].iter().collect()));
        } else {
            let start = i;
            while i < b.len() && !b[i].is_whitespace() && b[i] != '(' && b[i] != ')' {
                i += 1;
            }
            let w: String = b[start..i].iter().collect();
            toks.push(Tok::Ident(w.to_ascii_lowercase()));
        }
    }
    Some(toks)
}

/// One `(feature: value)` condition: width features compare against
/// [`VIEWPORT_WIDTH`], height against [`VIEWPORT_HEIGHT`]. Unknown features
/// (and valueless boolean forms) are `None`.
fn eval_cond(c: &str) -> Option<bool> {
    let (name, value) = c.split_once(':')?;
    let px = to_px(value.trim())?;
    let (vw, vh) = (f64::from(VIEWPORT_WIDTH), f64::from(VIEWPORT_HEIGHT));
    match name.trim().to_ascii_lowercase().as_str() {
        "min-width" => Some(vw >= px),
        "max-width" => Some(vw <= px),
        "width" => Some(vw == px),
        "min-height" => Some(vh >= px),
        "max-height" => Some(vh <= px),
        "height" => Some(vh == px),
        _ => None,
    }
}

/// A `<number><unit>` length in px; `em`/`rem` at the 16px root font-size
/// (js.md §7 — the same conversion `matchMedia` always used). Unknown or
/// missing units are `None`.
fn to_px(v: &str) -> Option<f64> {
    let v = v.to_ascii_lowercase();
    let (num, scale) = if let Some(n) = v.strip_suffix("rem") {
        (n, 16.0)
    } else if let Some(n) = v.strip_suffix("em") {
        (n, 16.0)
    } else if let Some(n) = v.strip_suffix("px") {
        (n, 1.0)
    } else {
        return None;
    };
    num.trim().parse::<f64>().ok().map(|n| n * scale)
}

#[cfg(test)]
mod tests;
