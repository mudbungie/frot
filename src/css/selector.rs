//! Selector AST, specificity, and matching.
//!
//! Supported: `*`, type, `.class`, `#id`, `[attr]`, `[attr=val]`, compound
//! sequences, the descendant (whitespace), child (`>`), next-sibling (`+`)
//! and subsequent-sibling (`~`) combinators, the logical pseudo-classes
//! `:is()` / `:where()` / `:not()` over complex selector lists
//! ([`super::pseudo`]), `:nth-child()` and its family ([`Nth`]), the state
//! pseudo-classes ([`State`]; `:hover` and its kind are [`Simple::Never`]),
//! and the `::before` / `::after` pseudo-elements (legacy single-colon
//! spelling too). Any other pseudo (`:has(...)`, `:nth-col()`,
//! `::first-line`, …) parses into a [`Simple::Unsupported`] marker so the
//! whole compound fails to match — a rule we cannot evaluate must never apply.

use super::nth::Nth;
use super::state::State;
use crate::dom::{Document, Element, NodeId, NodeKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    Descendant,
    Child,
    /// `+`: the element sibling immediately before.
    NextSibling,
    /// `~`: any element sibling before.
    SubsequentSibling,
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
    Attr {
        name: String,
        val: Option<String>,
    },
    /// `:is(…)`: matches when any argument does; as specific as its most
    /// specific argument.
    Is(Vec<Selector>),
    /// `:where(…)`: `:is(…)` with zero specificity.
    Where(Vec<Selector>),
    /// `:not(…)`: matches when no argument does; as specific as its most
    /// specific argument. Every argument is [`Selector::evaluable`].
    Not(Vec<Selector>),
    /// A position among siblings: `:nth-child()` and its family ([`Nth`]).
    Nth(Nth),
    /// A state computed from the DOM ([`State`]).
    State(State),
    /// A pseudo-class known to be false in an unattended load (`:hover`):
    /// unlike [`Simple::Unsupported`] its negation is known too.
    Never,
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

    /// Whether any compound carries a pseudo-element. (An argument of a
    /// logical pseudo-class never does — [`super::pseudo`] refuses it.)
    pub fn pseudo_element(&self) -> bool {
        self.parts.iter().any(|(_, c)| c.pseudo.is_some())
    }

    /// Whether the engine can answer this selector: no
    /// [`Simple::Unsupported`] marker at any depth.
    pub fn evaluable(&self) -> bool {
        let simples = self.parts.iter().flat_map(|(_, c)| &c.simples);
        simples.into_iter().all(|s| match s {
            Simple::Unsupported => false,
            Simple::Is(list) | Simple::Where(list) | Simple::Not(list) => {
                list.iter().all(Selector::evaluable)
            }
            Simple::Nth(nth) => nth.of().iter().all(Selector::evaluable),
            _ => true,
        })
    }

    pub fn specificity(&self) -> Specificity {
        let mut s = Specificity::default();
        for (_, c) in &self.parts {
            for simple in &c.simples {
                match simple {
                    Simple::Id(_) => s.0 += 1,
                    Simple::Class(_) | Simple::Attr { .. } => s.1 += 1,
                    Simple::State(_) | Simple::Never => s.1 += 1,
                    Simple::Nth(nth) => {
                        let most = nth.of().iter().map(Selector::specificity).max();
                        let Specificity(a, b, c) = most.unwrap_or_default();
                        s = Specificity(s.0 + a, s.1 + b + 1, s.2 + c);
                    }
                    Simple::Type(_) => s.2 += 1,
                    Simple::Is(list) | Simple::Not(list) => {
                        let most = list.iter().map(Selector::specificity).max();
                        let Specificity(a, b, c) = most.unwrap_or_default();
                        s = Specificity(s.0 + a, s.1 + b, s.2 + c);
                    }
                    Simple::Universal | Simple::Where(_) | Simple::Unsupported => {}
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

fn compound_matches(c: &Compound, doc: &Document, id: NodeId, el: &Element) -> bool {
    let any = |list: &[Selector]| list.iter().any(|s| matches(s, doc, id));
    c.simples.iter().all(|s| match s {
        Simple::Is(list) | Simple::Where(list) => any(list),
        Simple::Not(list) => !any(list),
        Simple::Nth(nth) => nth.holds(doc, id, el),
        Simple::State(state) => state.holds(doc, id, el),
        Simple::Universal => true,
        Simple::Unsupported | Simple::Never => false,
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

fn element(doc: &Document, id: NodeId) -> Option<&Element> {
    match &doc.node(id).kind {
        NodeKind::Element(el) => Some(el),
        _ => None,
    }
}

/// `id`'s element ancestors, nearest first.
pub(super) fn ancestors(doc: &Document, id: NodeId) -> impl Iterator<Item = (NodeId, &Element)> {
    std::iter::successors(doc.node(id).parent, |&p| doc.node(p).parent)
        .filter_map(|p| element(doc, p).map(|el| (p, el)))
}

/// The nodes on either side of `id` among its parent's children, in document
/// order. A top-level node's parent is the document node; a node with no
/// parent — one a script created and has not inserted — has no siblings.
fn sides(doc: &Document, id: NodeId) -> (&[NodeId], &[NodeId]) {
    let parent = doc.node(id).parent;
    let family = parent.map_or(&[][..], |p| &doc.node(p).children[..]);
    let mut halves = family.split(|&s| s == id);
    (
        halves.next().unwrap_or_default(),
        halves.next().unwrap_or_default(),
    )
}

/// The element siblings before `id`, nearest first. Text and comments between
/// siblings do not separate them, as in CSS.
pub(super) fn earlier_siblings(doc: &Document, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
    let before = sides(doc, id).0.iter().rev().copied();
    before.filter(|&s| element(doc, s).is_some())
}

/// The element siblings after `id`, nearest first.
pub(super) fn later_siblings(doc: &Document, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
    let after = sides(doc, id).1.iter().copied();
    after.filter(|&s| element(doc, s).is_some())
}

/// Does `sel` match the node `id` where it stands in `doc`? Only an element
/// can match. The subject compound's pseudo-element is ignored here; the
/// cascade routes its declarations to the right pseudo bucket.
pub fn matches(sel: &Selector, doc: &Document, id: NodeId) -> bool {
    matches_at(&sel.parts, sel.parts.len() - 1, doc, id)
}

fn matches_at(parts: &[(Combinator, Compound)], i: usize, doc: &Document, id: NodeId) -> bool {
    let Some(el) = element(doc, id) else {
        return false;
    };
    if !compound_matches(&parts[i].1, doc, id, el) {
        return false;
    }
    if i == 0 {
        return true;
    }
    let rest = |n: NodeId| matches_at(parts, i - 1, doc, n);
    let mut above = ancestors(doc, id).map(|(a, _)| a);
    match parts[i].0 {
        Combinator::Child => above.take(1).any(rest),
        Combinator::Descendant => above.any(rest),
        Combinator::NextSibling => earlier_siblings(doc, id).take(1).any(rest),
        Combinator::SubsequentSibling => earlier_siblings(doc, id).any(rest),
    }
}

#[cfg(test)]
mod tests;
