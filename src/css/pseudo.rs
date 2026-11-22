//! Pseudo-class and pseudo-element parsing: what follows a `:` in a compound.
//!
//! Supported: the `::before` / `::after` pseudo-elements (legacy single-colon
//! spelling too) and the logical pseudo-classes `:is(…)`, `:where(…)` and
//! `:not(…)`, each over a comma-separated list of complex selectors. Every
//! other pseudo parses into [`Simple::Unsupported`], which never matches.
//!
//! A logical pseudo-class is itself [`Simple::Unsupported`] when the engine
//! could not answer it truthfully:
//!
//! - an argument that does not parse, or that carries a pseudo-element (the
//!   grammar forbids one there) — the list is not parsed forgivingly;
//! - an unterminated argument list;
//! - for `:not()` only, an argument holding an unsupported simple at any
//!   depth. "Never matches" is a safe reading of `:hover`, but its negation
//!   would be "always matches" — `:not(:hover)` must fail closed, not open.
//!   Inside `:is()` / `:where()` an unsupported argument simply never matches
//!   and the other arguments still can.

use super::selector::{Pseudo, Selector, Simple};
use super::selparse::{ident, parse_selector, split_top};

pub(super) enum PseudoTok {
    Element(Pseudo),
    Class(Simple),
}

/// Parse the pseudo whose first `:` is at `start`; returns it and the index
/// just past it.
pub(super) fn parse_pseudo(b: &[char], start: usize) -> (PseudoTok, usize) {
    let mut i = start + 1;
    if i < b.len() && b[i] == ':' {
        i += 1;
    }
    let (name, mut n) = ident(b, i);
    let name = name.to_ascii_lowercase();
    if n < b.len() && b[n] == '(' {
        // Consume a balanced functional argument, e.g. `:nth-child(2n+1)`.
        let open = n;
        let mut depth = 0u32;
        while n < b.len() {
            match b[n] {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            n += 1;
            if depth == 0 {
                let args: String = b[open + 1..n - 1].iter().collect();
                return (PseudoTok::Class(logical(&name, &args)), n);
            }
        }
        return (PseudoTok::Class(Simple::Unsupported), n);
    }
    let tok = match name.as_str() {
        "before" => PseudoTok::Element(Pseudo::Before),
        "after" => PseudoTok::Element(Pseudo::After),
        _ => PseudoTok::Class(Simple::Unsupported),
    };
    (tok, n)
}

/// The functional pseudo-class `:name(args)`, or [`Simple::Unsupported`]
/// (module doc).
fn logical(name: &str, args: &str) -> Simple {
    let list: Option<Vec<Selector>> = split_top(args, ',')
        .iter()
        .map(|a| parse_selector(a.trim()).filter(|s| !s.pseudo_element()))
        .collect();
    match (name, list) {
        ("is", Some(list)) => Simple::Is(list),
        ("where", Some(list)) => Simple::Where(list),
        ("not", Some(list)) if list.iter().all(Selector::evaluable) => Simple::Not(list),
        _ => Simple::Unsupported,
    }
}

#[cfg(test)]
mod tests;
