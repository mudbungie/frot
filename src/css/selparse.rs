//! Selector-list parsing into the [`super::selector`] AST.

use super::escape::Quoting;
use super::pseudo::{parse_pseudo, PseudoTok};
use super::selector::{Combinator, Compound, Selector, Simple};

/// Split a selector list on top-level commas (commas inside `[]`, `()`, or
/// quotes stay), parse each, and drop any that fail to parse.
pub fn parse_selector_list(s: &str) -> Vec<Selector> {
    split_top(s, ',')
        .into_iter()
        .filter_map(|part| parse_selector(part.trim()))
        .collect()
}

/// Parse a selector list for `querySelector` (js.md §3). Unlike
/// [`parse_selector_list`], which silently drops unparseable selectors, this
/// returns `None` if **any** selector fails to parse or is unqueryable — one
/// carrying an unsupported simple at any depth (`:hover`, `:nth-child(…)`,
/// `:is(:hover)`) or a pseudo-element (`::before` matches no real element).
/// The caller turns `None` into a thrown, counted error rather than a silent
/// empty match.
pub fn parse_query(list: &str) -> Option<Vec<Selector>> {
    // `split_top` always yields at least one part; an unparseable one returns
    // via `?`, so a normal exit means every part parsed and `out` is non-empty.
    let mut out = Vec::new();
    for part in split_top(list, ',') {
        let sel = parse_selector(part.trim())?;
        if sel.pseudo_element() || !sel.evaluable() {
            return None;
        }
        out.push(sel);
    }
    Some(out)
}

/// Split on `sep` at bracket/paren depth 0 and outside quotes — string
/// literals are opaque, escapes included ([`super::escape::Quoting`]).
pub fn split_top(s: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut depth = 0u32;
    let mut quoting = Quoting::default();
    for ch in s.chars() {
        if quoting.feed(ch) {
            buf.push(ch);
            continue;
        }
        match ch {
            '[' | '(' => {
                depth += 1;
                buf.push(ch);
            }
            ']' | ')' => {
                depth = depth.saturating_sub(1);
                buf.push(ch);
            }
            c if c == sep && depth == 0 => out.push(std::mem::take(&mut buf)),
            _ => buf.push(ch),
        }
    }
    out.push(buf);
    out
}

enum Tok {
    Compound(String),
    Combinator(Combinator),
}

pub(super) fn parse_selector(s: &str) -> Option<Selector> {
    let mut parts: Vec<(Combinator, Compound)> = Vec::new();
    let mut comb = Combinator::Descendant;
    for tok in tokenize_combinators(s) {
        match tok {
            Tok::Combinator(c) => comb = c,
            Tok::Compound(text) => {
                parts.push((comb, parse_compound(&text)?));
                comb = Combinator::Descendant;
            }
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(Selector { parts })
}

/// Break a complex selector into compound chunks and `>` / `+` / `~` markers.
/// Whitespace runs become compound boundaries; whitespace around a marker is
/// absorbed.
fn tokenize_combinators(s: &str) -> Vec<Tok> {
    let mut toks = Vec::new();
    let mut buf = String::new();
    let mut depth = 0u32;
    let mut quote: Option<char> = None;
    let flush = |buf: &mut String, toks: &mut Vec<Tok>| {
        if !buf.is_empty() {
            toks.push(Tok::Compound(std::mem::take(buf)));
        }
    };
    for ch in s.chars() {
        if let Some(q) = quote {
            buf.push(ch);
            if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => {
                quote = Some(ch);
                buf.push(ch);
            }
            '[' | '(' => {
                depth += 1;
                buf.push(ch);
            }
            ']' | ')' => {
                depth = depth.saturating_sub(1);
                buf.push(ch);
            }
            '>' | '+' | '~' if depth == 0 => {
                flush(&mut buf, &mut toks);
                toks.push(Tok::Combinator(match ch {
                    '>' => Combinator::Child,
                    '+' => Combinator::NextSibling,
                    _ => Combinator::SubsequentSibling,
                }));
            }
            c if c.is_whitespace() && depth == 0 => flush(&mut buf, &mut toks),
            _ => buf.push(ch),
        }
    }
    flush(&mut buf, &mut toks);
    toks
}

fn parse_compound(s: &str) -> Option<Compound> {
    let mut simples = Vec::new();
    let mut pseudo = None;
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            '*' => {
                simples.push(Simple::Universal);
                i += 1;
            }
            '.' => {
                let (name, n) = ident(&b, i + 1);
                if name.is_empty() {
                    return None;
                }
                simples.push(Simple::Class(name));
                i = n;
            }
            '#' => {
                let (name, n) = ident(&b, i + 1);
                if name.is_empty() {
                    return None;
                }
                simples.push(Simple::Id(name));
                i = n;
            }
            '[' => {
                let (simple, n) = parse_attr(&b, i + 1)?;
                simples.push(simple);
                i = n;
            }
            ':' => {
                let (p, n) = parse_pseudo(&b, i);
                match p {
                    PseudoTok::Element(pe) => pseudo = Some(pe),
                    PseudoTok::Class(simple) => simples.push(simple),
                }
                i = n;
            }
            _ => {
                let (name, n) = ident(&b, i);
                if name.is_empty() {
                    return None;
                }
                simples.push(Simple::Type(name.to_ascii_lowercase()));
                i = n;
            }
        }
    }
    // `tokenize_combinators` never yields an empty chunk, and every branch
    // above either contributes a simple/pseudo or returns `None`, so a
    // compound reaching here is non-empty.
    Some(Compound { simples, pseudo })
}

pub(super) fn ident(b: &[char], start: usize) -> (String, usize) {
    let mut i = start;
    while i < b.len() && (b[i].is_alphanumeric() || b[i] == '-' || b[i] == '_') {
        i += 1;
    }
    (b[start..i].iter().collect(), i)
}

fn parse_attr(b: &[char], start: usize) -> Option<(Simple, usize)> {
    let (name, mut i) = ident(b, start);
    if name.is_empty() {
        return None;
    }
    i = skip_ws(b, i);
    if i < b.len() && b[i] == ']' {
        return Some((attr(name, None), i + 1));
    }
    if i >= b.len() || b[i] != '=' {
        return None;
    }
    i = skip_ws(b, i + 1);
    let val = if i < b.len() && (b[i] == '"' || b[i] == '\'') {
        let q = b[i];
        let s: String = b[i + 1..].iter().take_while(|&&c| c != q).collect();
        i += s.chars().count() + 2;
        s
    } else {
        let (v, n) = ident(b, i);
        i = n;
        v
    };
    i = skip_ws(b, i);
    if i >= b.len() || b[i] != ']' {
        return None;
    }
    Some((attr(name, Some(val)), i + 1))
}

fn attr(name: String, val: Option<String>) -> Simple {
    Simple::Attr {
        name: name.to_ascii_lowercase(),
        val,
    }
}

fn skip_ws(b: &[char], mut i: usize) -> usize {
    while i < b.len() && b[i].is_whitespace() {
        i += 1;
    }
    i
}

#[cfg(test)]
mod tests;
