//! `:has()`: an element (the anchor) matches when some element stands where
//! one of the relative selectors in the argument reaches from it — `:has(b)`
//! a descendant `b`, `:has(> b)` a child, `:has(+ b)` the next element
//! sibling, `:has(~ b)` a later one.
//!
//! The relative selector's leftmost compound is anchored, so `:has(p b)`
//! wants the `p` inside the anchor too, and `:has(+ a b)` a `b` inside the
//! next sibling `a`. Measured in Chrome 158 (`bl-2a85`): the argument list is
//! not forgiving — one argument that does not parse, an empty one, a
//! pseudo-element or a nested `:has()` voids the whole selector
//! (`querySelector` throws); `:is(:has(b))` and `:not(:has(b))` work. It
//! weighs as its most specific argument, as `:is()` does.

use super::selector::{anchored, later_siblings, Combinator, Context, Selector, Simple};
use super::selparse::{parse_selector, split_top};
use crate::dom::{Document, NodeId, WalkEvent};

/// The argument list of `:has(…)`, or `None` when it is invalid (module
/// doc). Each selector's leading combinator rides in `parts[0].0`.
pub(super) fn parse(args: &str) -> Option<Vec<Selector>> {
    let list = split_top(args, ',');
    list.iter().map(|a| relative(a.trim())).collect()
}

/// One relative selector. A leading `>`, `+` or `~` is what the parser
/// already records as the first compound's combinator; none is a descendant.
fn relative(arg: &str) -> Option<Selector> {
    parse_selector(arg).filter(|s| !s.pseudo_element() && !nests(s))
}

/// Whether `sel` holds a `:has()` at any depth.
fn nests(sel: &Selector) -> bool {
    let simples = sel.parts.iter().flat_map(|(_, c)| &c.simples);
    simples.into_iter().any(|s| match s {
        Simple::Has(_) => true,
        Simple::Is(list) | Simple::Where(list) | Simple::Not(list) => list.iter().any(nests),
        Simple::Nth(nth) => nth.of().iter().any(nests),
        _ => false,
    })
}

/// Whether the anchor `id` has an element that one of `list` reaches.
pub(super) fn holds(list: &[Selector], cx: Context, id: NodeId) -> bool {
    list.iter().any(|rel| {
        let reach = reach(cx.doc, id, rel.parts[0].0);
        reach.into_iter().any(|n| anchored(rel, cx, n, id))
    })
}

/// Every node a relative selector led by `comb` could land on from `id`: its
/// descendants, or its later siblings (just the next for `+`) and theirs.
fn reach(doc: &Document, id: NodeId, comb: Combinator) -> Vec<NodeId> {
    let starts: Vec<NodeId> = match comb {
        Combinator::Child | Combinator::Descendant => doc.node(id).children.clone(),
        Combinator::NextSibling => later_siblings(doc, id).take(1).collect(),
        Combinator::SubsequentSibling => later_siblings(doc, id).collect(),
    };
    let mut out = Vec::new();
    for s in starts {
        doc.walk(Some(s), &mut |ev, _| {
            if let WalkEvent::Enter(n) = ev {
                out.push(n);
            }
        });
    }
    out
}

#[cfg(test)]
mod tests;
