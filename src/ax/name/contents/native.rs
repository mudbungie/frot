//! What an element says *instead of* its contents — the replaced-element and
//! embedded-control alternatives of accname rule 2D/2E, as one table.
//!
//! This is the half of [`super`] that is a lookup rather than a traversal: it
//! answers "what does this tag say for itself", and the recursion asks it once
//! per element. `<optgroup>`/`<option>` answer through [`super::super::label_attr`],
//! the single home of what an option says (`bl-4093`).

use crate::dom::{Document, Element, NodeId, NodeKind, WalkEvent};
use crate::tags;

/// Tags that say nothing in a name and separate nothing: source and data
/// ([`tags::is_source`]), a ruby annotation and its fallback parentheses, and
/// a `<title>` — which is its parent's alternative ([`svg_title`]) or, in
/// HTML, unrendered. Measured in Chrome 158 (`bl-3fc9`): `a<ruby>rb<rt>rt</rt>
/// </ruby>c` is "arbc" and `a<title>t</title>c` is "ac".
///
/// An `<object>`'s fallback and an image `<map>` say nothing either: `a<object>
/// obj</object>c` and `a<map><area alt=ar href=x></map>c` are "ac" (`bl-d94b`).
pub fn silent(name: &str) -> bool {
    tags::is_source(name) || matches!(name, "rp" | "rt" | "title" | "object" | "map")
}

/// Roles whose contents a descendant does not give: it says its own author
/// name and nothing else, and a `<fieldset>` its legend. Measured in Chrome
/// 158 (`bl-d94b`) on `a<span role=R>X</span>c`, "ac" for each — "a c" for the
/// two that are controls ([`CONTROLS`]) — and on `a<fieldset><legend>lg
/// </legend>fs</fieldset>c` ("a lg c"), `<figure>`, `<article>`, `<dialog
/// open>` and `<output>` ("a c").
const OPAQUE: &[&str] = &[
    "application",
    "article",
    "combobox",
    "dialog",
    "figure",
    "grid",
    "group",
    "img",
    "listbox",
    "menu",
    "menubar",
    "note",
    "progressbar",
    "radiogroup",
    "separator",
    "status",
    "tablist",
    "toolbar",
    "tree",
];

/// Whether a descendant with `id`'s role keeps its contents out of a name.
/// A `group` does only when the author wrote the role, or is a `<fieldset>`:
/// an open `<details>` ("a sm db c"), an unlabelled `<optgroup>` and a
/// described SVG `<g>` are groups that still give theirs.
pub fn opaque(doc: &Document, id: NodeId, el: &Element) -> bool {
    match crate::ax::role(doc, id) {
        Some("group") => el.name == "fieldset" || crate::ax::role::explicit(doc, id, el).is_some(),
        role => role.is_some_and(|r| OPAQUE.contains(&r)),
    }
}

/// What an ARIA range widget says in a name: `aria-valuetext`, else
/// `aria-valuenow`, else its default — the middle of a slider's or
/// scrollbar's range (`aria-valuemin` 0 and `aria-valuemax` 100 unless
/// given), zero for a spinbutton or meter. Measured in Chrome 158
/// (`bl-d94b`): `a<span role=slider>X</span>c` is "a 50 c", with
/// `aria-valuemin=10 aria-valuemax=20` "a 15 c", with `aria-valuenow=7`
/// "a 7 c" and `aria-valuetext=seven` "a seven c"; `role=spinbutton` "a 0 c".
pub fn range_value(doc: &Document, id: NodeId, el: &Element) -> Option<String> {
    let role = crate::ax::role(doc, id)?;
    let num = |attr: &str| el.attr(attr).and_then(|v| v.trim().parse::<f64>().ok());
    let middle =
        || (num("aria-valuemin").unwrap_or(0.0) + num("aria-valuemax").unwrap_or(100.0)) / 2.0;
    let default = match role {
        "slider" | "scrollbar" => middle(),
        "spinbutton" | "meter" => 0.0,
        _ => return None,
    };
    let text = el
        .attr("aria-valuetext")
        .filter(|t| !t.trim_ascii().is_empty());
    Some(text.map_or_else(
        || num("aria-valuenow").unwrap_or(default).to_string(),
        str::to_string,
    ))
}

/// Roles that are controls: a descendant holding one is spaced off from the
/// text either side of it, whatever its box. Measured in Chrome 158
/// (`bl-3fc9`) on `a<span role=R>X</span>c`: these give "a X c", while
/// `link`, `tooltip`, `cell`, `heading` and `term` give "aXc".
const CONTROLS: &[&str] = &[
    "button",
    "checkbox",
    "listbox",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "radio",
    "searchbox",
    "switch",
    "tab",
    "textbox",
    "tree",
];

/// Whether `el` is an atomic box whatever its computed `display`: a line
/// break, a control — by role, or a `<button>` under any role (`<button
/// role=link>` and `<button style="display:inline">` are still spaced off) —
/// or a `<canvas>` that has fallback content (an empty one separates nothing).
/// An `<output>` too: `a<output>out</output>c` is "a c" (`bl-d94b`).
pub fn atomic(doc: &Document, id: NodeId, el: &Element) -> bool {
    match el.name.as_str() {
        "br" | "wbr" | "button" | "output" => true,
        "canvas" => !doc.node(id).children.is_empty(),
        _ => crate::ax::role(doc, id).is_some_and(|r| CONTROLS.contains(&r)),
    }
}

/// The quote marks a `<q>` generates around its contents — the UA sheet's
/// `q::before { content: open-quote }` — which Chrome reads into a name
/// (`a<q>quote</q>c` is "a“quote”c", `bl-3fc9`). Double at the outer level and
/// single inside another `<q>`; these are the English marks, whatever `lang`
/// says.
pub fn quotes(doc: &Document, id: NodeId) -> Option<(&'static str, &'static str)> {
    let is_q = |n: NodeId| matches!(&doc.node(n).kind, NodeKind::Element(e) if e.name == "q");
    let depth = std::iter::successors(Some(id), |&n| doc.node(n).parent)
        .filter(|&n| is_q(n))
        .count();
    match (is_q(id), depth % 2) {
        (false, _) => None,
        (true, 1) => Some(("\u{201c}", "\u{201d}")),
        (true, _) => Some(("\u{2018}", "\u{2019}")),
    }
}

/// The replaced-element and embedded-control alternatives. `alt=""` is an
/// alternative — the empty one — which is exactly how a decorative image
/// contributes nothing without its `src` leaking in.
pub fn native_alternative(doc: &Document, id: NodeId, el: &Element) -> Option<String> {
    match el.name.as_str() {
        "img" => image_alternative(doc, id, el),
        "iframe" => Some(el.attr("title").unwrap_or("").to_string()),
        // MathML is not read into a name: `a<math><mi>x</mi></math>c` is "a c".
        "math" => Some(String::new()),
        "meter" | "progress" => Some(gauge(el)),
        "input" => Some(input_alternative(el)),
        "textarea" => Some(doc.text_content(id)),
        "select" => Some(selected_option_text(doc, id)),
        "option" | "optgroup" => super::super::label_attr(el),
        n if crate::ax::role::svg_titled(doc, id, n) => svg_title(doc, id),
        // An SVG description is never part of a name.
        "desc" => Some(String::new()),
        _ => None,
    }
}

/// An image is a word of its own even when it has none to say: `a<img>c` is
/// "a c" (Chrome 158, `bl-3fc9`). One the author made presentational —
/// `alt=""` with no other name, `role=presentation`, `role=none` — is not in
/// the tree at all, so it says nothing and separates nothing: `a<img alt="">c`
/// is "ac".
fn image_alternative(doc: &Document, id: NodeId, el: &Element) -> Option<String> {
    match crate::ax::role(doc, id) {
        Some("presentation" | "none") => None,
        _ => Some(el.attr("alt").unwrap_or("").to_string()),
    }
}

/// What a `<meter>` or `<progress>` says: its value, clamped to its range
/// (`min` 0 and `max` 1 unless given). A meter with no usable value says its
/// minimum; a progress bar with none is indeterminate and says nothing.
/// Measured in Chrome 158 (`bl-3fc9`): `<meter value=.5>` is "0.5",
/// `<meter value=1.50>` "1", `<meter>` "0", `<progress value=5 max=10>` "5",
/// `<progress>` "".
fn gauge(el: &Element) -> String {
    let num = |attr: &str| el.attr(attr).and_then(|v| v.trim().parse::<f64>().ok());
    let min = num("min").filter(|_| el.name == "meter").unwrap_or(0.0);
    let max = num("max").unwrap_or(1.0).max(min);
    match num("value") {
        None if el.name == "progress" => String::new(),
        value => value.unwrap_or(min).clamp(min, max).to_string(),
    }
}

/// An `<input>` is a leaf that speaks through its `value` — a button's
/// default label when it has none, a range's middle, a password's bullets —
/// and `type=image` through a non-empty `alt` first. Measured in Chrome 158
/// (`bl-d94b`): `a<input type=submit>c` is "a Submit c", `type=reset` "a Reset
/// c", `type=image` "a Submit c" (with `alt="" value=IV`, "a IV c"),
/// `type=range` "a 50 c" (`min=10 max=20`, "a 15 c"), `type=password
/// value=pw` "a •• c".
fn input_alternative(el: &Element) -> String {
    let kind = el.attr("type").unwrap_or("").to_ascii_lowercase();
    let value = el.attr("value");
    let num = |attr: &str, or: f64| {
        el.attr(attr)
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(or)
    };
    match kind.as_str() {
        "image" => el
            .attr("alt")
            .filter(|a| !a.is_empty())
            .or(value)
            .unwrap_or("Submit")
            .to_string(),
        "range" => value.map_or_else(
            || ((num("min", 0.0) + num("max", 100.0)) / 2.0).to_string(),
            str::to_string,
        ),
        "password" => "\u{2022}".repeat(value.unwrap_or("").chars().count()),
        _ => value.or(button_default(&kind)).unwrap_or("").to_string(),
    }
}

/// The label a `type=submit` or `type=reset` button shows with no `value`.
pub fn button_default(kind: &str) -> Option<&'static str> {
    match kind {
        "submit" => Some("Submit"),
        "reset" => Some("Reset"),
        _ => None,
    }
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

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;
