//! What an element says *instead of* its contents — the replaced-element and
//! embedded-control alternatives of accname rule 2D/2E, as one table.
//!
//! This is the half of [`super`] that is a lookup rather than a traversal: it
//! answers "what does this tag say for itself", and the recursion asks it once
//! per element. `<optgroup>`/`<option>` answer through [`super::super::label_attr`],
//! the single home of what an option says (`bl-4093`).

use crate::dom::{Document, Element, NodeId, NodeKind, WalkEvent};

/// The replaced-element and embedded-control alternatives. `alt=""` is an
/// alternative — the empty one — which is exactly how a decorative image
/// contributes nothing without its `src` leaking in.
pub fn native_alternative(doc: &Document, id: NodeId, el: &Element) -> Option<String> {
    match el.name.as_str() {
        "img" | "area" => el.attr("alt").map(str::to_string),
        "input" => Some(input_alternative(el)),
        "textarea" => Some(doc.text_content(id)),
        "select" => Some(selected_option_text(doc, id)),
        "option" | "optgroup" => super::super::label_attr(el),
        n if crate::ax::role::svg_titled(n) => svg_title(doc, id),
        // An SVG description is never part of a name.
        "desc" => Some(String::new()),
        _ => None,
    }
}

/// An `<input>` is a leaf: `type=image` speaks through `alt`, everything else
/// (button labels and embedded controls alike) through its `value`.
fn input_alternative(el: &Element) -> String {
    let attr = match el.attr("type").unwrap_or("").to_ascii_lowercase().as_str() {
        "image" => "alt",
        _ => "value",
    };
    el.attr(attr).unwrap_or("").to_string()
}

/// An embedded `<select>` speaks its selected option — the first one when the
/// author marked none, since that is the one the UA selects. What the option
/// says is its own label, so `<option label=X>y</option>` speaks "X" (measured:
/// Chrome names `<button>Choose <select>…</select></button>` "Choose X").
fn selected_option_text(doc: &Document, id: NodeId) -> String {
    let (mut first, mut selected) = (None, None);
    doc.walk(Some(id), &mut |ev, e| {
        if let (WalkEvent::Enter(n), NodeKind::Element(el)) = (ev, &e.kind) {
            if el.name == "option" {
                let says = super::super::label_attr(el).unwrap_or_else(|| doc.text_content(n));
                if selected.is_none() && el.attr("selected").is_some() {
                    selected = Some(says.clone());
                }
                first.get_or_insert(says);
            }
        }
    });
    selected.or(first).unwrap_or_default()
}

/// SVG-AAM: an `<svg>` — and equally a `<g>`, a shape, a `<use>`
/// (`role::svg_titled`, `bl-07bd`) — is named by its `<title>` *child*: the
/// first one, and only a child. Measured in Chrome 156 (`bl-0a0a`), a `<title>`
/// deeper in (`<svg><g><title>`) names the `<g>`, not the graphic; a titled
/// part speaks that title and is not descended into (`<a><svg><g><title>deep
/// </title><text>tx</text></g></svg></a>` is "deep"). An empty `<title>` is
/// no title, unlike `alt=""`: Chrome names `<a><svg><title></title><text>t
/// </text></svg></a>` "t". The one home of this fact, for the graphic's own
/// name and for what it contributes to someone else's.
pub fn svg_title(doc: &Document, id: NodeId) -> Option<String> {
    let mut children = doc.node(id).children.iter().copied();
    let title = children
        .find(|&c| matches!(&doc.node(c).kind, NodeKind::Element(e) if e.name == "title"))?;
    Some(doc.text_content(title)).filter(|t| !t.trim_ascii().is_empty())
}
