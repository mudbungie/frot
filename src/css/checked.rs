//! Checkedness and selectedness (`:checked`, the `checked` and `selected`
//! properties): the attribute a page was parsed with, until a script sets the
//! property — HTML's dirty flag, kept in the arena (`dom::live`, `bl-ff74`).
//!
//! Measured in Chrome 158 (`tests/fixtures/fidelity/checkedjs.html`):
//! `input.checked = true` leaves no `checked` attribute and `defaultChecked`
//! and `:default` keep answering from the attribute; checking a radio
//! unchecks the rest of its group; selecting an option in a drop-down or a
//! list box deselects the others, and deselecting a drop-down's only
//! selected option falls back to the first one that can be picked;
//! `input.indeterminate` is a property only — no attribute — that
//! `:indeterminate` reads on a checkbox, and a checked checkbox made
//! indeterminate is not `:checked` (its `checked` property still says true).

use super::selector::ancestors;
use super::state::{has, input_type};
use crate::dom::{Document, Element, NodeId, NodeKind, WalkEvent};

/// What the `checked` (or an option's `selected`) property reads: `:checked`
/// for a radio or option, else the dirty value or the attribute.
pub fn picked(doc: &Document, id: NodeId, el: &Element) -> bool {
    match (el.name.as_str(), input_type(el).as_str()) {
        ("input", "radio") | ("option", _) => checked(doc, id, el),
        _ => doc.picked(id).unwrap_or(has(el, "checked")),
    }
}

/// The `checked` / `selected` setter: `id` takes `on`, and when it is turned
/// on so are the rest of its radio group or its single select turned off.
pub fn pick(doc: &mut Document, id: NodeId, on: bool) {
    let mut off = Vec::new();
    if let Some(el) = doc.element(id).filter(|_| on) {
        let single = owner_select(doc, id).filter(|(_, s)| !has(s, "multiple"));
        let (radio, mine) = (input_type(el) == "radio", group(doc, id, el));
        let (option, select) = (el.name == "option", single.map(|s| s.0));
        doc.walk(select, &mut |ev, entry| {
            if let (WalkEvent::Enter(n), NodeKind::Element(e)) = (ev, &entry.kind) {
                let rival = if option {
                    select.is_some() && e.name == "option"
                } else {
                    radio && rival_radio(doc, n, e, mine)
                };
                if rival && n != id {
                    off.push(n);
                }
            }
        });
    }
    for n in off {
        doc.set_picked(n, false);
    }
    doc.set_picked(id, on);
}

fn rival_radio(
    doc: &Document,
    n: NodeId,
    e: &Element,
    mine: Option<(&str, Option<NodeId>)>,
) -> bool {
    e.name == "input" && input_type(e) == "radio" && mine.is_some() && group(doc, n, e) == mine
}

pub(super) fn owner_select(doc: &Document, id: NodeId) -> Option<(NodeId, &Element)> {
    ancestors(doc, id).find(|(_, e)| e.name == "select")
}

/// An option's own `disabled`, or a disabled optgroup between it and its
/// select. This — not `:disabled` — is what keeps a select from picking it.
pub(super) fn option_off(doc: &Document, id: NodeId, el: &Element) -> bool {
    let mut groups = ancestors(doc, id).take_while(|(_, e)| e.name != "select");
    has(el, "disabled") || groups.any(|(_, e)| e.name == "optgroup" && has(e, "disabled"))
}

/// `:checked`: a checkbox with `checked` that is not `indeterminate` (Chrome
/// paints it as neither); a radio with `checked` that no later
/// radio of its group overrides; a selected option — each until a script
/// sets it ([`pick`]).
pub(super) fn checked(doc: &Document, id: NodeId, el: &Element) -> bool {
    match (el.name.as_str(), input_type(el).as_str()) {
        ("input", "checkbox") => picked(doc, id, el) && !doc.indeterminate(id),
        ("input", "radio") => doc
            .picked(id)
            .unwrap_or_else(|| has(el, "checked") && last_of_group(doc, id, el)),
        ("option", _) => selected(doc, id, el),
        _ => false,
    }
}

/// A radio's group: its name and its form owner ([`Document::form_owner`]),
/// which for a radio no form owns is the tree. Unnamed radios are each
/// alone. A radio with `form=f` and one inside `<form id=f>` are one group,
/// and a `form` attribute that names no form puts a radio with the unowned
/// ones (Chrome 158, `bl-e8fa`).
pub(super) fn group<'a>(
    doc: &Document,
    id: NodeId,
    el: &'a Element,
) -> Option<(&'a str, Option<NodeId>)> {
    let name = el.attr("name").filter(|n| !n.is_empty())?;
    Some((name, doc.form_owner(id)))
}

/// No radio after `id` in tree order has `checked` and `id`'s group.
fn last_of_group(doc: &Document, id: NodeId, el: &Element) -> bool {
    let Some(mine) = group(doc, id, el) else {
        return true;
    };
    let (mut passed, mut last) = (false, true);
    doc.walk(None, &mut |ev, entry| {
        if let (WalkEvent::Enter(n), NodeKind::Element(e)) = (ev, &entry.kind) {
            let rival = input_type(e) == "radio" && has(e, "checked");
            last &= !(passed && rival && e.name == "input" && group(doc, n, e) == Some(mine));
            passed |= n == id;
        }
    });
    last
}

/// An option's selectedness. Outside a select, or in a `multiple` one, its
/// own (dirty, else `selected`). Otherwise the select picks one: the last option with
/// `selected`, else — unless it is a list box (`size` above 1) — the first
/// that is not [`option_off`].
pub(super) fn selected(doc: &Document, id: NodeId, el: &Element) -> bool {
    let Some((select, sel)) = owner_select(doc, id).filter(|(_, s)| !has(s, "multiple")) else {
        return doc.picked(id).unwrap_or(has(el, "selected"));
    };
    // (id, has `selected`, cannot be the default pick), in tree order.
    let mut options = Vec::new();
    doc.walk(Some(select), &mut |ev, entry| {
        if let (WalkEvent::Enter(n), NodeKind::Element(e)) = (ev, &entry.kind) {
            if e.name == "option" {
                let on = doc.picked(n).unwrap_or(has(e, "selected"));
                options.push((n, on, option_off(doc, n, e)));
            }
        }
    });
    let marked = options.iter().rev().find(|o| o.1);
    let list_box = sel.attr("size").and_then(|s| s.trim().parse().ok()) > Some(1u32);
    let default = options.iter().find(|o| !list_box && !o.2);
    marked.or(default).is_some_and(|o| o.0 == id)
}

#[cfg(test)]
mod tests {
    use super::{pick, picked};
    use crate::css::query_all;
    use crate::dom::Document;

    /// Run `writes` (`(id, on)`) through the setter, then answer `selector`
    /// with the ids it matches.
    fn after(html: &str, writes: &[(&str, bool)], selector: &str) -> String {
        let mut doc = Document::parse(html);
        for &(id, on) in writes {
            let n = doc.element_by_id(id).unwrap();
            pick(&mut doc, n, on);
        }
        let hits = query_all(&doc, Document::ROOT, selector).unwrap();
        let id = |&n: &u32| doc.element(n).and_then(|e| e.attr("id")).unwrap_or("-");
        hits.iter().map(id).collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn a_script_write_is_state_and_keeps_groups_exclusive() {
        let radios = "<input type=radio name=g id=a checked><input type=radio name=g id=b>\
                      <input type=radio name=h id=c checked><input type=radio id=d>";
        assert_eq!(after(radios, &[("b", true)], ":checked"), "b c");
        assert_eq!(after(radios, &[("b", true)], "[checked]"), "a c");
        assert_eq!(
            after(radios, &[("c", false), ("d", true)], ":checked"),
            "a d"
        );
        let boxes = "<input type=checkbox id=a><input type=checkbox id=b checked>";
        assert_eq!(after(boxes, &[("a", true), ("b", false)], ":checked"), "a");
        let select = "<select><option id=a selected>a<option id=b>b<option id=c>c</select>";
        assert_eq!(after(select, &[("c", true)], ":checked"), "c");
        assert_eq!(after(select, &[("a", false)], ":checked"), "a");
        let multiple = "<select multiple><option id=a selected>a<option id=b>b</select>";
        assert_eq!(after(multiple, &[("b", true)], ":checked"), "a b");
        let lone = "<option id=a>a</option><input id=t checked>";
        assert_eq!(after(lone, &[("a", true), ("t", false)], ":checked"), "a");
    }

    #[test]
    fn the_property_reads_checkedness_and_indeterminate_hides_checked() {
        let mut doc = Document::parse("<input type=checkbox id=c checked><input id=t checked>");
        let (c, t) = (
            doc.element_by_id("c").unwrap(),
            doc.element_by_id("t").unwrap(),
        );
        doc.set_indeterminate(c, true);
        let el = |n| doc.element(n).unwrap();
        assert!(picked(&doc, c, el(c)) && picked(&doc, t, el(t)));
        let hits = query_all(&doc, Document::ROOT, ":checked, :indeterminate").unwrap();
        assert_eq!(hits, [c]);
        assert!(query_all(&doc, Document::ROOT, ":checked")
            .unwrap()
            .is_empty());
    }
}
