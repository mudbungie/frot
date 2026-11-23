//! Pseudo-class and pseudo-element parsing: what follows a `:` in a compound.
//!
//! Supported: the `::before` / `::after` pseudo-elements (legacy single-colon
//! spelling too), the child-indexed pseudo-classes (`:nth-child()` and its
//! family, [`super::nth`]), the state pseudo-classes ([`super::state`], and `NEVER`
//! for those known to be false), and the logical pseudo-classes `:is(…)`,
//! `:where(…)` and `:not(…)`, each over a comma-separated list of complex
//! selectors. Every other pseudo parses into [`Simple::Unsupported`], which
//! never matches — a pseudo-class written with two colons (`::first-child`)
//! among them.
//!
//! A logical pseudo-class is itself [`Simple::Unsupported`] when the engine
//! could not answer it truthfully:
//!
//! - an argument that does not parse, or that carries a pseudo-element (the
//!   grammar forbids one there) — the list is not parsed forgivingly;
//! - an unterminated argument list;
//! - for `:not()` only, an argument holding an unsupported simple at any
//!   depth. "Never matches" is a safe reading of `:target`, whose answer the
//!   engine does not know, but its negation would be "always matches" —
//!   `:not(:target)` must fail closed, not open. (`:not(:hover)` is another
//!   matter: `:hover` is known to be false, so its negation matches.)
//!   Inside `:is()` / `:where()` an unsupported argument simply never matches
//!   and the other arguments still can.

use super::nth::Nth;
use super::selector::{Pseudo, Selector, Simple};
use super::selparse::{ident, parse_selector, split_top};
use super::state::State;

pub(super) enum PseudoTok {
    Element(Pseudo),
    Class(Simple),
}

/// Parse the pseudo whose first `:` is at `start`; returns it and the index
/// just past it.
pub(super) fn parse_pseudo(b: &[char], start: usize) -> (PseudoTok, usize) {
    let mut i = start + 1;
    let double = i < b.len() && b[i] == ':';
    if double {
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
                return (PseudoTok::Class(logical(&name, &args, double)), n);
            }
        }
        return (PseudoTok::Class(Simple::Unsupported), n);
    }
    let tok = match (name.as_str(), double) {
        ("before", _) => PseudoTok::Element(Pseudo::Before),
        ("after", _) => PseudoTok::Element(Pseudo::After),
        (name, false) if NEVER.contains(&name) => PseudoTok::Class(Simple::Never),
        (name, false) => {
            let state = || State::named(name).map_or(Simple::Unsupported, Simple::State);
            PseudoTok::Class(Nth::named(name).map_or_else(state, Simple::Nth))
        }
        _ => PseudoTok::Class(Simple::Unsupported),
    };
    (tok, n)
}

/// Pseudo-classes that are false for every element of a page nobody is
/// using: no pointer, no history, no popover, dialog or full-screen request,
/// no autofill, no user edit. Measured in headless Chrome 158
/// (`tests/fixtures/fidelity/state.html`).
const NEVER: [&str; 11] = [
    "hover",
    "active",
    "visited",
    "popover-open",
    "modal",
    "fullscreen",
    "picture-in-picture",
    "autofill",
    "-webkit-autofill",
    "user-valid",
    "user-invalid",
];

/// The functional pseudo-class `:name(args)`, or [`Simple::Unsupported`]
/// (module doc).
fn logical(name: &str, args: &str, double: bool) -> Simple {
    match (name, double) {
        ("is", false) => list(args).map_or(Simple::Unsupported, Simple::Is),
        ("where", false) => list(args).map_or(Simple::Unsupported, Simple::Where),
        ("not", false) => list(args)
            .filter(|list| list.iter().all(Selector::evaluable))
            .map_or(Simple::Unsupported, Simple::Not),
        (name, false) => Nth::function(name, args).map_or(Simple::Unsupported, Simple::Nth),
        _ => Simple::Unsupported,
    }
}

/// A comma-separated list of complex selectors, parsed unforgivingly: `None`
/// if any one does not parse or carries a pseudo-element.
pub(super) fn list(args: &str) -> Option<Vec<Selector>> {
    let parts = split_top(args, ',');
    let parsed = parts
        .iter()
        .map(|a| parse_selector(a.trim()).filter(|s| !s.pseudo_element()));
    parsed.collect()
}

#[cfg(test)]
mod tests;
