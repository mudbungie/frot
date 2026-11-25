//! The `srcset` and `sizes` attributes (`bl-0e4a`), parsed as Firefox
//! 153.0esr parses them — the HTML "parse a srcset attribute" tokenizer, with
//! the descriptor grammar measured on the rig (js.md §7 "Images").

use super::length;
use crate::css::media;
use crate::css::split_top;
use crate::layout::VIEWPORT_WIDTH;

/// What one candidate says about its density.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Desc {
    /// `Nx` — the density itself.
    X(f64),
    /// `Nw` — a width, the density being it over the source size.
    W(u32),
}

/// One image candidate: its URL as written, and its descriptor.
#[derive(Clone, Debug, PartialEq)]
pub struct Cand {
    pub url: String,
    pub desc: Desc,
}

/// `srcset`'s candidates in order, the invalid ones dropped and a candidate
/// whose descriptor repeats an earlier one's (same kind, same value) skipped —
/// the first wins (measured: `a 1x, b 1x` and `a 400w, b 400w` pick `a`).
pub fn parse(srcset: &str) -> Vec<Cand> {
    let mut out: Vec<Cand> = Vec::new();
    let b = srcset.as_bytes();
    let mut i = 0;
    loop {
        while b
            .get(i)
            .is_some_and(|c| c.is_ascii_whitespace() || *c == b',')
        {
            i += 1;
        }
        if i == b.len() {
            return out;
        }
        let start = i;
        while b.get(i).is_some_and(|c| !c.is_ascii_whitespace()) {
            i += 1;
        }
        let mut url = &srcset[start..i];
        let mut descs = Vec::new();
        if url.ends_with(',') {
            url = url.trim_end_matches(',');
        } else {
            i = descriptors(srcset, i, &mut descs);
        }
        if let Some(desc) = descriptor(&descs).filter(|d| !out.iter().any(|c| c.desc == *d)) {
            out.push(Cand {
                url: url.to_string(),
                desc,
            });
        }
    }
}

/// Tokenize the descriptors after a URL (the spec's three-state machine): a
/// comma outside parentheses ends the candidate. Returns where it stopped.
fn descriptors(s: &str, mut i: usize, out: &mut Vec<String>) -> usize {
    let mut cur = String::new();
    let mut depth = false;
    for c in s[i..].chars() {
        i += c.len_utf8();
        match c {
            ')' if depth => depth = false,
            '(' if !depth => depth = true,
            ',' if !depth => break,
            c if c.is_ascii_whitespace() && !depth => {
                out.extend((!cur.is_empty()).then(|| std::mem::take(&mut cur)));
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    out.extend((!cur.is_empty()).then_some(cur));
    i
}

/// The descriptor a token list makes, or `None` for an invalid one: at most
/// one `x` or one `w`, an `h` only beside a `w`; none at all is `1x`.
fn descriptor(tokens: &[String]) -> Option<Desc> {
    let (mut x, mut w, mut h) = (None, None, None);
    for t in tokens {
        let (num, kind) = t.split_at(t.len() - t.chars().last().map_or(0, char::len_utf8));
        let slot = match kind {
            "x" if x.is_none() && w.is_none() => {
                x = Some(density(num)?);
                continue;
            }
            "w" => &mut w,
            "h" => &mut h,
            _ => return None,
        };
        if slot.is_some() || x.is_some() {
            return None;
        }
        *slot = Some(integer(num)?);
    }
    match (x, w, h) {
        (Some(d), None, None) => Some(Desc::X(d)),
        (None, Some(w), _) => Some(Desc::W(w)),
        (None, None, None) => Some(Desc::X(1.0)),
        _ => None,
    }
}

/// A width or height: positive digits with no leading zero (measured: `0400w`,
/// `+400w`, `400.0w`, `1e2w` and `0w` are all invalid).
fn integer(n: &str) -> Option<u32> {
    let ok = n.bytes().all(|c| c.is_ascii_digit()) && !n.starts_with('0');
    n.parse().ok().filter(|_| ok)
}

/// A density: `-`? then digits with an optional fraction or a bare fraction,
/// then an optional exponent; never negative (measured: `.5x`, `1e-1x`, `0x`
/// and `-0x` are valid, `+1x`, `1.x` and `-1x` are not).
fn density(n: &str) -> Option<f64> {
    let body = n.strip_prefix('-').unwrap_or(n);
    let (mant, exp) = body.split_once(['e', 'E']).unwrap_or((body, "0"));
    let exp = exp.strip_prefix(['+', '-']).unwrap_or(exp);
    let digits = |s: &str| s.bytes().all(|c| c.is_ascii_digit());
    let ok = match mant.split_once('.') {
        Some((int, frac)) => digits(int) && !frac.is_empty() && digits(frac),
        None => !mant.is_empty() && digits(mant),
    };
    let v: f64 = n
        .parse()
        .ok()
        .filter(|_| ok && !exp.is_empty() && digits(exp))?;
    (v >= 0.0).then_some(v.abs())
}

/// The source size `sizes` gives, in CSS pixels: the first entry whose media
/// condition holds (or that has none) and whose length is valid; `100vw` when
/// none does. `auto` is a size only when `auto` is `Some` — the width of a
/// `loading=lazy` image (measured: ignored on an eager one).
pub fn size(sizes: Option<&str>, auto: Option<f64>) -> f64 {
    split_top(sizes.unwrap_or_default(), ',')
        .iter()
        .find_map(|entry| entry_size(entry.trim(), auto))
        .unwrap_or(f64::from(VIEWPORT_WIDTH))
}

/// One `sizes` entry: `<media-condition>? <source-size-value>`.
fn entry_size(entry: &str, auto: Option<f64>) -> Option<f64> {
    let cut = value_start(entry);
    let (cond, value) = (entry[..cut].trim(), &entry[cut..]);
    let px = if value.eq_ignore_ascii_case("auto") {
        auto?
    } else {
        length::px(value)?
    };
    (cond.is_empty() || condition(cond)).then_some(px)
}

/// Where the trailing size value of `entry` begins: a function call's name,
/// or the last run after whitespace or a `)`.
fn value_start(entry: &str) -> usize {
    let b = entry.as_bytes();
    let mut end = b.len();
    if entry.ends_with(')') {
        let mut depth = 0;
        while end > 0 {
            end -= 1;
            depth += i32::from(b[end] == b')') - i32::from(b[end] == b'(');
            if depth == 0 {
                break;
            }
        }
        while end > 0 && b[end - 1].is_ascii_alphabetic() {
            end -= 1;
        }
        return end;
    }
    entry
        .rfind(|c: char| c.is_ascii_whitespace() || c == ')')
        .map_or(0, |k| k + 1)
}

/// A `sizes` media condition: a media query with no media type (measured:
/// `screen 150px` is invalid, `not (…)` and `(…) and (…)` are not).
fn condition(c: &str) -> bool {
    let lower = c.to_ascii_lowercase();
    let bare = lower
        .strip_prefix("not")
        .map_or(lower.as_str(), str::trim_start);
    bare.starts_with('(') && media::matches(c)
}

#[cfg(test)]
mod tests;
