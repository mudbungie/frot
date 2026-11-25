//! What an element's `ElementInternals` tell the page's views (js.md §15.6):
//! whether its definition is form-associated, its custom states (`:state()`),
//! whether its validity flags say invalid (`:invalid`), and the ARIA defaults
//! its internals set, which the accessibility tree reads where the author set
//! no attribute. The `ElementInternals` object and its validity flags are the
//! prelude's (`internals.js`); these are the facts the cascade and `ax` ask of
//! the arena, written through on every change.

use crate::dom::{Document, Element, NodeId};

#[derive(Debug, Clone, Default)]
pub struct Internals {
    /// The element's definition is form-associated.
    pub face: bool,
    /// Its custom states, in the order added.
    pub states: Vec<String>,
    /// Some validity flag is set (`setValidity`).
    pub invalid: bool,
    /// The ARIA defaults set through the internals, as attribute names.
    pub aria: Vec<(String, String)>,
}

impl Document {
    /// The internals state of `id`, if anything set one.
    pub fn internals(&self, id: NodeId) -> Option<&Internals> {
        let hit = self.live.internals.iter().find(|(n, _)| *n == id);
        hit.map(|(_, i)| i)
    }

    /// Change the internals state of `id`.
    pub fn set_internals(&mut self, id: NodeId, change: impl FnOnce(&mut Internals)) {
        let list = &mut self.live.internals;
        let at = match list.iter().position(|(n, _)| *n == id) {
            Some(at) => at,
            None => {
                list.push((id, Internals::default()));
                list.len() - 1
            }
        };
        change(&mut list[at].1);
        self.generation += 1;
    }

    /// Whether `id` is a form-associated custom element.
    pub fn face(&self, id: NodeId) -> bool {
        self.internals(id).is_some_and(|i| i.face)
    }

    /// Whether `id`'s custom states hold `state` (`:state()`, case-sensitive).
    pub fn custom_state(&self, id: NodeId, state: &str) -> bool {
        self.internals(id)
            .is_some_and(|i| i.states.iter().any(|s| s == state))
    }

    /// The ARIA attribute `name` of the element `el` at `id`: the author's
    /// attribute, else the default its internals set (Chrome 158,
    /// `tests/fixtures/fidelity/internals.html`).
    pub fn aria<'a>(&'a self, id: NodeId, el: &'a Element, name: &str) -> Option<&'a str> {
        el.attr(name).or_else(|| self.aria_default(id, name))
    }

    /// The ARIA attribute `name`'s default the internals of `id` set.
    pub fn aria_default(&self, id: NodeId, name: &str) -> Option<&str> {
        let aria = self.internals(id).map_or(&[][..], |i| &i.aria[..]);
        aria.iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

#[cfg(test)]
mod tests {
    use crate::dom::Document;

    #[test]
    fn internals_default_until_set_and_attributes_outrank_them() {
        let mut doc = Document::parse("<x-a role=link></x-a><x-b></x-b>");
        let (a, b) = (doc.find_by_tag("x-a")[0], doc.find_by_tag("x-b")[0]);
        assert!(!doc.face(a) && doc.internals(a).is_none());
        doc.set_internals(b, |i| i.states.push("on".into()));
        assert!(doc.custom_state(b, "on") && !doc.custom_state(b, "On"));
        assert!(!doc.custom_state(a, "on"));
        doc.set_internals(a, |i| i.aria.push(("role".into(), "button".into())));
        doc.set_internals(a, |i| i.face = true);
        doc.set_internals(b, |i| i.aria.push(("role".into(), "img".into())));
        assert!(doc.face(a) && !doc.face(b));
        let el = |n| doc.element(n).unwrap();
        assert_eq!(doc.aria(a, el(a), "role"), Some("link"));
        assert_eq!(doc.aria(b, el(b), "role"), Some("img"));
        assert_eq!(doc.aria(b, el(b), "aria-label"), None);
        assert_eq!(doc.aria(Document::ROOT, el(b), "aria-label"), None);
    }
}
