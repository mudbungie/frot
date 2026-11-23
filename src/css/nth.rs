//! The child-indexed pseudo-classes: `:nth-child()`, `:nth-last-child()`,
//! `:nth-of-type()`, `:nth-last-of-type()` and the names that are fixed cases
//! of them — `:first-child`, `:last-child`, `:only-child` and the three
//! `-of-type` forms.
//!
//! One shape covers all ten: an element's 1-based position among the
//! siblings that count, taken from the start, the end or both, tested
//! against `An+B`. Text and comments are never siblings. The grammar is what
//! Chrome 158 accepts (`tests/fixtures/fidelity/structural.html`).

use super::pseudo::list;
use super::selector::{earlier_siblings, later_siblings, matches_in, Context, Selector};
use crate::dom::{Element, NodeId, NodeKind};

/// Which siblings are counted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Among {
    /// Every element sibling.
    Children,
    /// Those with the element's own tag name (`-of-type`).
    Type,
    /// Those matching a selector list (`:nth-child(… of S)`); the element
    /// itself must match it too.
    Of(Vec<Selector>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nth {
    /// The position must be `a·n + b` for some `n ≥ 0`.
    pub a: i32,
    pub b: i32,
    /// Tested against the position counted from the start.
    pub start: bool,
    /// Tested against the position counted from the end.
    pub end: bool,
    pub among: Among,
}

impl Nth {
    /// The pseudo-class `:name` (no arguments), if it is one of these.
    pub fn named(name: &str) -> Option<Nth> {
        let (start, end, among) = match name {
            "first-child" => (true, false, Among::Children),
            "last-child" => (false, true, Among::Children),
            "only-child" => (true, true, Among::Children),
            "first-of-type" => (true, false, Among::Type),
            "last-of-type" => (false, true, Among::Type),
            "only-of-type" => (true, true, Among::Type),
            _ => return None,
        };
        let (a, b) = (0, 1);
        Some(Nth {
            a,
            b,
            start,
            end,
            among,
        })
    }

    /// The pseudo-class `:name(args)`, if it is one of these and `args` is
    /// `An+B`, with ` of S` allowed on the two `-child` forms.
    pub fn function(name: &str, args: &str) -> Option<Nth> {
        let (end, typed) = match name {
            "nth-child" => (false, false),
            "nth-last-child" => (true, false),
            "nth-of-type" => (false, true),
            "nth-last-of-type" => (true, true),
            _ => return None,
        };
        let (formula, of) = split_of(args);
        let among = match (of, typed) {
            (None, false) => Among::Children,
            (None, true) => Among::Type,
            (Some(s), false) => Among::Of(list(s)?),
            (Some(_), true) => return None,
        };
        let (a, b) = an_plus_b(&formula.trim().to_ascii_lowercase())?;
        Some(Nth {
            a,
            b,
            start: !end,
            end,
            among,
        })
    }

    /// The `of S` list; empty without one.
    pub fn of(&self) -> &[Selector] {
        match &self.among {
            Among::Of(list) => list,
            _ => &[],
        }
    }

    /// Whether the element `el` at `id` stands at a matching position.
    pub fn holds(&self, cx: Context, id: NodeId, el: &Element) -> bool {
        let doc = cx.doc;
        let counts = |s: &NodeId| match &self.among {
            Among::Children => true,
            Among::Type => {
                matches!(&doc.node(*s).kind, NodeKind::Element(e) if e.name == el.name)
            }
            Among::Of(list) => list.iter().any(|sel| matches_in(sel, cx, *s)),
        };
        let at = |position: usize| {
            let over = position as i64 - i64::from(self.b);
            match i64::from(self.a) {
                0 => over == 0,
                a => over % a == 0 && over / a >= 0,
            }
        };
        counts(&id)
            && (!self.start || at(1 + earlier_siblings(doc, id).filter(counts).count()))
            && (!self.end || at(1 + later_siblings(doc, id).filter(counts).count()))
    }
}

/// Split `An+B of S` at the `of` keyword: lower-case and set off by
/// whitespace on both sides (Chrome rejects `2 OF li` and `2of li`).
fn split_of(args: &str) -> (&str, Option<&str>) {
    let space = |c: Option<char>| c.is_some_and(char::is_whitespace);
    let mut hits = args.match_indices("of");
    let hit = hits.find(|&(i, _)| {
        space(args[..i].chars().next_back()) && space(args[i + 2..].chars().next())
    });
    match hit {
        Some((i, _)) => (&args[..i], Some(&args[i + 2..])),
        None => (args, None),
    }
}

/// The `An+B` microsyntax over trimmed, lower-cased text: `odd`, `even`, an
/// integer, or `An` with an optional signed `B` — whitespace allowed only
/// around that sign, and none inside `An`.
fn an_plus_b(s: &str) -> Option<(i32, i32)> {
    match s {
        "odd" => return Some((2, 1)),
        "even" => return Some((2, 0)),
        _ => {}
    }
    let Some((step, rest)) = s.split_once('n') else {
        return Some((0, s.parse().ok()?));
    };
    let a = match step {
        "" | "+" => 1,
        "-" => -1,
        digits => digits.parse().ok()?,
    };
    let rest = rest.trim_start();
    if rest.is_empty() {
        return Some((a, 0));
    }
    let sign = match rest.chars().next() {
        Some('+') => 1,
        Some('-') => -1,
        _ => return None,
    };
    let digits = rest[1..].trim_start();
    if !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some((a, sign * digits.parse::<i32>().ok()?))
}

#[cfg(test)]
mod tests;
