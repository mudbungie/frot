//! Selector AST, specificity, and matching.
//!
//! Supported: `*`, type, `.class`, `#id`, `[attr]`, `[attr=val]`, compound
//! sequences, the descendant (whitespace) and child (`>`) combinators, and the
//! `::before` / `::after` pseudo-elements (legacy single-colon spelling too).
//! Any other pseudo (`:hover`, `:nth-child(...)`, `::first-line`, …) parses
//! into a [`Simple::Unsupported`] marker so the whole compound fails to
//! match — a rule we cannot evaluate must never hide or reveal content.

use crate::dom::Element;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    Descendant,
    Child,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pseudo {
    Before,
    After,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Simple {
    Universal,
    Type(String),
    Class(String),
    Id(String),
    Attr { name: String, val: Option<String> },
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compound {
    pub simples: Vec<Simple>,
    pub pseudo: Option<Pseudo>,
}

/// A selector as compounds joined by combinators. `parts[i].0` is the
/// combinator linking `parts[i-1]` to `parts[i]`; `parts[0].0` is unused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    pub parts: Vec<(Combinator, Compound)>,
}

/// `(id, class+attr, type+pseudo-element)`, compared lexicographically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Specificity(pub u32, pub u32, pub u32);

impl Selector {
    /// The pseudo-element of the subject (rightmost) compound, if any.
    pub fn pseudo(&self) -> Option<Pseudo> {
        self.parts.last().and_then(|(_, c)| c.pseudo)
    }

    pub fn specificity(&self) -> Specificity {
        let mut s = Specificity::default();
        for (_, c) in &self.parts {
            for simple in &c.simples {
                match simple {
                    Simple::Id(_) => s.0 += 1,
                    Simple::Class(_) | Simple::Attr { .. } => s.1 += 1,
                    Simple::Type(_) => s.2 += 1,
                    Simple::Universal | Simple::Unsupported => {}
                }
            }
            if c.pseudo.is_some() {
                s.2 += 1;
            }
        }
        s
    }
}

fn class_list(el: &Element) -> Vec<&str> {
    el.attr("class")
        .map(|c| c.split_whitespace().collect())
        .unwrap_or_default()
}

fn compound_matches(c: &Compound, el: &Element) -> bool {
    c.simples.iter().all(|s| match s {
        Simple::Universal => true,
        Simple::Unsupported => false,
        Simple::Type(t) => el.name == *t,
        Simple::Class(cl) => class_list(el).contains(&cl.as_str()),
        Simple::Id(id) => el.attr("id") == Some(id.as_str()),
        Simple::Attr { name, val } => match (el.attr(name), val) {
            (Some(_), None) => true,
            (Some(v), Some(want)) => v == want,
            (None, _) => false,
        },
    })
}

/// Does `sel` match `el`, whose element ancestors are `ancestors`
/// (nearest first)? The subject compound's pseudo-element is ignored here;
/// the cascade routes its declarations to the right pseudo bucket.
pub fn matches(sel: &Selector, el: &Element, ancestors: &[&Element]) -> bool {
    matches_at(&sel.parts, sel.parts.len() - 1, el, ancestors)
}

fn matches_at(
    parts: &[(Combinator, Compound)],
    i: usize,
    el: &Element,
    ancestors: &[&Element],
) -> bool {
    if !compound_matches(&parts[i].1, el) {
        return false;
    }
    if i == 0 {
        return true;
    }
    match parts[i].0 {
        Combinator::Child => {
            !ancestors.is_empty() && matches_at(parts, i - 1, ancestors[0], &ancestors[1..])
        }
        Combinator::Descendant => (0..ancestors.len())
            .any(|k| matches_at(parts, i - 1, ancestors[k], &ancestors[k + 1..])),
    }
}

#[cfg(test)]
mod tests;
