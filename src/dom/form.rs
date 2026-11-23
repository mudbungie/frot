//! Form ownership (HTML, "form owner") — the one home of the
//! question "which form is this control in?", asked by `--out forms` and by
//! the radio-group rule behind `:checked`.
//!
//! A control that carries a `form` attribute is owned by the first element in
//! the tree with that id *if that element is a form*, and by no form at all
//! otherwise — an empty value, an id nothing has, or an id a `<div>` has
//! first all leave it unowned, even inside another form. A control without
//! the attribute is owned by its nearest `<form>` ancestor. Measured in
//! Chrome 158 `form.elements` (`bl-e8fa`); ids compare case-sensitively.

use super::{Document, NodeId, NodeKind, WalkEvent};

impl Document {
    /// The first element in tree order whose `id` is `target`.
    pub fn element_by_id(&self, target: &str) -> Option<NodeId> {
        let mut found = None;
        self.walk(None, &mut |ev, e| {
            if let (WalkEvent::Enter(id), NodeKind::Element(el)) = (ev, &e.kind) {
                if found.is_none() && el.attr("id") == Some(target) {
                    found = Some(id);
                }
            }
        });
        found
    }

    /// The form that owns the element `id` (module docs), if any.
    pub fn form_owner(&self, id: NodeId) -> Option<NodeId> {
        let is_form = |&n: &NodeId| self.tag(n) == Some("form");
        let NodeKind::Element(el) = &self.node(id).kind else {
            return None;
        };
        match el.attr("form") {
            Some("") => None,
            Some(target) => self.element_by_id(target).filter(is_form),
            None => {
                std::iter::successors(self.node(id).parent, |&p| self.node(p).parent).find(is_form)
            }
        }
    }

    fn tag(&self, id: NodeId) -> Option<&str> {
        match &self.node(id).kind {
            NodeKind::Element(el) => Some(el.name.as_str()),
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
}
