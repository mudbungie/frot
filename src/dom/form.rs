//! Form ownership (HTML, "form owner") — the one home of the
//! question "which form is this control in?", asked by `--out forms` and by
//! the radio-group rule behind `:checked`.
//!
//! A connected control that carries a `form` attribute is owned by the first
//! element in the tree with that id *if that element is a form*, and by no
//! form at all otherwise — an empty value, an id nothing has, or an id a
//! `<div>` has first all leave it unowned, even inside another form. A
//! control without the attribute, or a detached one (HTML, "reset the form
//! owner"), is owned by its nearest `<form>` ancestor. Measured in
//! Chrome 158 `form.elements` (`bl-e8fa`); ids compare case-sensitively.

use super::{Document, Element, NodeId, NodeKind, WalkEvent};

impl Document {
    /// The first element in tree order whose `id` is `target`.
    pub fn element_by_id(&self, target: &str) -> Option<NodeId> {
        self.first(|el| el.attr("id") == Some(target))
    }

    /// The form that owns the element `id` (module docs), if any.
    pub fn form_owner(&self, id: NodeId) -> Option<NodeId> {
        let is_form = |&n: &NodeId| self.element(n).is_some_and(|e| e.name == "form");
        match self
            .element(id)?
            .attr("form")
            .filter(|_| self.connected(id))
        {
            Some("") => None,
            Some(target) => self.element_by_id(target).filter(is_form),
            None => {
                std::iter::successors(self.node(id).parent, |&p| self.node(p).parent).find(is_form)
            }
        }
    }

    /// The `<label>`s whose labeled control is `control`, in tree order,
    /// from the control's own tree (HTML, `labels`; Firefox 153 answers a
    /// wrapping label and a `for` label alike, js.md §15.6).
    pub fn labels(&self, control: NodeId) -> Vec<NodeId> {
        let elements = self.elements_in(self.tree_root(control));
        let labels = elements.iter().filter(|n| n.1 == "label");
        labels
            .filter(|&&(label, _)| self.labeled_control(label, &elements) == Some(control))
            .map(|n| n.0)
            .collect()
    }

    /// A label's control: the element its `for` names (first in the tree with
    /// that id), else its first labelable descendant — either only if labelable.
    fn labeled_control(&self, label: NodeId, tree: &[(NodeId, String)]) -> Option<NodeId> {
        let target = self.element(label).and_then(|el| el.attr("for"));
        let hit = match target {
            Some(t) => tree
                .iter()
                .find(|n| self.element(n.0).and_then(|e| e.attr("id")) == Some(t)),
            None => tree
                .iter()
                .find(|n| self.inside(n.0, label) && self.labelable(n.0)),
        };
        hit.map(|n| n.0).filter(|&n| self.labelable(n))
    }

    /// HTML's labelable elements, form-associated custom elements among them.
    fn labelable(&self, id: NodeId) -> bool {
        self.element(id).is_some_and(|el| match el.name.as_str() {
            "button" | "meter" | "output" | "progress" | "select" | "textarea" => true,
            "input" => !el
                .attr("type")
                .is_some_and(|t| t.eq_ignore_ascii_case("hidden")),
            _ => self.face(id),
        })
    }

    /// Every element of the tree at `root`, with its name, in tree order.
    fn elements_in(&self, root: NodeId) -> Vec<(NodeId, String)> {
        let mut out = Vec::new();
        self.walk(Some(root), &mut |ev, entry| {
            if let (WalkEvent::Enter(n), NodeKind::Element(el)) = (ev, &entry.kind) {
                out.push((n, el.name.clone()));
            }
        });
        out
    }

    /// The element at `id`, if it is one.
    pub fn element(&self, id: NodeId) -> Option<&Element> {
        match &self.node(id).kind {
            NodeKind::Element(el) => Some(el),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The owner of the element with id `control`, as its index among the
    /// document's forms.
    fn owner(html: &str, control: &str) -> Option<usize> {
        let doc = Document::parse(html);
        let id = doc.element_by_id(control).expect("the control");
        let forms = doc.find_by_tag("form");
        doc.form_owner(id)
            .map(|form| forms.iter().position(|&f| f == form).expect("a form"))
    }

    #[test]
    fn the_nearest_form_above_owns_a_control_without_the_attribute() {
        let html = "<form><p><input id=i></p></form><input id=o>";
        assert_eq!(owner(html, "i"), Some(0));
        assert_eq!(owner(html, "o"), None);
    }

    #[test]
    fn the_form_attribute_names_the_owner_wherever_the_control_is() {
        let html = "<input id=before form=f><form><input id=in form=f></form>\
                    <form id=f></form><form id=f></form>";
        assert_eq!(owner(html, "before"), Some(1));
        assert_eq!(owner(html, "in"), Some(1));
    }

    #[test]
    fn a_form_attribute_that_names_no_form_leaves_no_owner() {
        let html = "<form id=f><input id=e form=''><input id=m form=nope>\
                    <input id=u form=F><input id=d form=x></form><div id=x></div><form id=x></form>";
        for control in ["e", "m", "u", "d"] {
            assert_eq!(owner(html, control), None, "{control}");
        }
    }

    #[test]
    fn only_an_element_has_an_owner_or_an_id() {
        let doc = Document::parse("<form><p id=p>text</p></form>");
        let p = doc.element_by_id("p").unwrap();
        let text = doc.node(p).children[0];
        assert_eq!(doc.form_owner(text), None);
        assert_eq!(doc.element_by_id("nope"), None);
    }

    #[test]
    fn a_detached_control_ignores_its_form_attribute() {
        let mut doc = Document::parse("<form id=f></form><form id=g><input id=i form=f></form>");
        let i = doc.element_by_id("i").unwrap();
        let g = doc.element_by_id("g").unwrap();
        assert_eq!(doc.form_owner(i), doc.element_by_id("f"));
        doc.detach(g);
        assert_eq!(doc.form_owner(i), Some(g));
    }

    #[test]
    fn labels_are_the_labels_whose_control_is_it() {
        let mut doc = Document::parse(
            "<label id=a for=i>a</label><label id=b><input type=hidden><input id=i></label>\
             <label for=h id=c></label><input type=hidden id=h><label id=d><span>none</span></label>\
             <label id=e><x-f id=x></x-f></label><label id=m><meter id=me></meter></label>",
        );
        let by = |doc: &Document, s: &str| doc.element_by_id(s).unwrap();
        let (i, h, x, me) = (by(&doc, "i"), by(&doc, "h"), by(&doc, "x"), by(&doc, "me"));
        assert_eq!(doc.labels(i), [by(&doc, "a"), by(&doc, "b")]);
        assert!(doc.labels(h).is_empty() && doc.labels(x).is_empty());
        assert_eq!(doc.labels(me), [by(&doc, "m")]);
        doc.set_internals(x, |n| n.face = true);
        assert_eq!(doc.labels(x), [by(&doc, "e")]);
    }
}
