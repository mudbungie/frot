//! Key-selector index: bucket rules by their rightmost simple selector so a
//! given element only tests rules that could plausibly match it.
//!
//! The standard engine technique. Without it the cascade is
//! `O(elements × total selectors)` — a 30k-rule sheet over a 1k-element DOM
//! costs ~0.7s of pure matching (`docs/design/css.md`, Finding 1b).
//!
//! **This is a cost fix, never a semantics change.** Two invariants make the
//! indexed walk observationally identical to the exhaustive one:
//!
//! - *Completeness.* A selector is bucketed by a simple in its subject
//!   (rightmost) compound, and every simple in a compound is required for that
//!   compound to match. So if a selector matches an element, the element's own
//!   id / class / tag lookups necessarily reach it. Anything with no usable key
//!   — `*`, attribute-only, and [`Simple::Unsupported`] — lands in the
//!   always-tested `others` bucket rather than being dropped, preserving
//!   `selector.rs`'s fail-closed contract.
//! - *Order.* Every (rule, selector) pair carries the sequence number it had in
//!   the flat scan, and [`Index::candidates`] re-sorts by it, so declarations
//!   reach the cascade in exactly the order the exhaustive scan produced them —
//!   which is what `winner`'s last-max tie-break depends on.

use std::collections::HashMap;

use super::parse::{Decl, Stylesheet};
use super::selector::{Compound, Pseudo, Selector, Simple, Specificity};
use crate::dom::Element;

/// One (rule, selector) pair with its cascade coordinates resolved once at
/// build time instead of per element.
pub struct Candidate<'a> {
    /// Position in the flat sheet → rule → selector scan; restores its order.
    seq: usize,
    /// The rule's index across all sheets — the cascade's source-order key.
    pub order: usize,
    pub pseudo: Option<Pseudo>,
    pub spec: Specificity,
    pub selector: &'a Selector,
    pub decls: &'a [Decl],
}

enum Key<'a> {
    Id(&'a str),
    Class(&'a str),
    Tag(&'a str),
    None,
}

/// Rules bucketed by the key simple of their subject compound.
#[derive(Default)]
pub struct Index<'a> {
    ids: HashMap<&'a str, Vec<Candidate<'a>>>,
    classes: HashMap<&'a str, Vec<Candidate<'a>>>,
    tags: HashMap<&'a str, Vec<Candidate<'a>>>,
    /// Selectors with no usable key: universal, attribute-only, unsupported.
    others: Vec<Candidate<'a>>,
    /// Total rules across all sheets — the source-order slot inline `style=`
    /// occupies, one past every author rule.
    pub rule_count: usize,
}

impl<'a> Index<'a> {
    pub fn build(sheets: &'a [Stylesheet]) -> Index<'a> {
        let mut idx = Index::default();
        let mut seq = 0usize;
        for sheet in sheets {
            for rule in &sheet.rules {
                for sel in &rule.selectors {
                    let c = Candidate {
                        seq,
                        order: idx.rule_count,
                        pseudo: sel.pseudo(),
                        spec: sel.specificity(),
                        selector: sel,
                        decls: &rule.decls,
                    };
                    seq += 1;
                    idx.push(c);
                }
                idx.rule_count += 1;
            }
        }
        idx
    }

    fn push(&mut self, c: Candidate<'a>) {
        // `parts` is never empty: the selector parser only emits a selector
        // once it has read a compound (`selector::matches` relies on this too).
        match key(&c.selector.parts[c.selector.parts.len() - 1].1) {
            Key::Id(v) => self.ids.entry(v).or_default().push(c),
            Key::Class(v) => self.classes.entry(v).or_default().push(c),
            Key::Tag(v) => self.tags.entry(v).or_default().push(c),
            Key::None => self.others.push(c),
        }
    }

    /// Every rule `el` could match, in flat-scan order. A superset of the
    /// matching rules — callers still run [`super::selector::matches`].
    pub fn candidates(&self, el: &Element) -> Vec<&Candidate<'a>> {
        let mut out: Vec<&Candidate<'a>> = self.others.iter().collect();
        let id = el.attr("id").unwrap_or_default();
        out.extend(self.ids.get(id).into_iter().flatten());
        for cl in el.attr("class").unwrap_or_default().split_whitespace() {
            out.extend(self.classes.get(cl).into_iter().flatten());
        }
        out.extend(self.tags.get(el.name.as_str()).into_iter().flatten());
        out.sort_by_key(|c| c.seq);
        out
    }
}

/// The bucket key of a subject compound: an id if it has one, else a class,
/// else a type. Nothing else narrows the candidate set — `*` matches every
/// element, an attribute name is not indexed, and an unsupported simple must
/// stay in the always-tested bucket so it keeps failing closed rather than
/// disappearing.
fn key(compound: &Compound) -> Key<'_> {
    let mut class = None;
    let mut tag = None;
    for s in &compound.simples {
        match s {
            Simple::Id(v) => return Key::Id(v),
            Simple::Class(v) => class = Some(v.as_str()),
            Simple::Type(v) => tag = Some(v.as_str()),
            _ => {}
        }
    }
    match (class, tag) {
        (Some(c), _) => Key::Class(c),
        (None, Some(t)) => Key::Tag(t),
        (None, None) => Key::None,
    }
}

#[cfg(test)]
mod tests;
