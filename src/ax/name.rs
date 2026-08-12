//! Accessible-name computation: a Phase-1 subset of the WAI accname algorithm.
//!
//! Priority order:
//! 1. `aria-label` (if non-empty)
//! 2. `aria-labelledby` — join the referenced elements' *text alternatives*
//!    ([`contents::referenced_name`]) with a space
//! 3. Native labelling:
//!    - `<img>` and `<area>` — `alt`
//!    - `<input type=button|submit|reset>` — `value`
//!    - `<input type=image>` — `alt`
//!    - other form controls (`<input>`, `<textarea>`, `<select>`) — the matching
//!      `<label for=…>` or the wrapping `<label>` text content
//!    - `<optgroup>` and `<option>` — the `label` content attribute
//!      ([`label_attr`]); an `<option>` without one falls back to its contents
//!    - everything else — the element's descendant text *alternatives* (the
//!      §2F recursion, [`contents`]), but *only* when the element's role is
//!      name-from-contents (see [`NAME_FROM_CONTENTS`]); containers (table,
//!      row, list, paragraph, nav, …) yield no native name
//! 4. `title`
//!
//! Returns the trimmed result, or `None` if nothing produced text. With
//! `Some(styles)` (`--css`), the text-content sources drop `display:none`
//! subtrees and include `::before`/`::after` generated content.

mod contents;

use crate::css::Styles;
use crate::dom::{Document, NodeId, NodeKind, WalkEvent};

/// The name a role that names from its contents takes from its subtree — the
/// §2F recursion over descendants only ([`contents`]), never the element's own
/// alternative, which the steps above `native_name` have already had.
fn contents_text(doc: &Document, id: NodeId, styles: Option<&Styles>) -> String {
    contents::contents_name(doc, id, styles, &[])
}

pub fn accessible_name(doc: &Document, id: NodeId, styles: Option<&Styles>) -> Option<String> {
    let entry = doc.node(id);
    let NodeKind::Element(el) = &entry.kind else {
        return None;
    };

    if let Some(label) = trimmed_attr(el, "aria-label") {
        return Some(label);
    }
    if let Some(label) = labelledby_text(doc, el, styles) {
        return Some(label);
    }
    if let Some(label) = native_name(doc, id, el, styles) {
        let t = collapse_whitespace(&label);
        if !t.is_empty() {
            return Some(t);
        }
    }
    if let Some(label) = trimmed_attr(el, "title") {
        return Some(label);
    }
    None
}

fn trimmed_attr(el: &crate::dom::Element, name: &str) -> Option<String> {
    el.attr(name)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn labelledby_text(
    doc: &Document,
    el: &crate::dom::Element,
    styles: Option<&Styles>,
) -> Option<String> {
    let raw = el.attr("aria-labelledby")?;
    let parts: Vec<String> = raw
        .split_whitespace()
        .filter_map(|target| find_by_id(doc, target))
        .map(|id| collapse_whitespace(&contents::referenced_name(doc, id, styles)))
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn native_name(
    doc: &Document,
    id: NodeId,
    el: &crate::dom::Element,
    styles: Option<&Styles>,
) -> Option<String> {
    match el.name.as_str() {
        "img" | "area" => el.attr("alt").map(str::to_string),
        "input" => input_name(doc, id, el, styles),
        "textarea" | "select" => control_label_text(doc, id, el, styles),
        "fieldset" => fieldset_legend_text(doc, id, styles),
        "optgroup" => label_attr(el),
        "option" => label_attr(el).or_else(|| Some(contents_text(doc, id, styles))),
        _ if names_from_contents(doc, id) => Some(contents_text(doc, id, styles)),
        _ => None,
    }
}

/// The `label` content attribute of `<optgroup>`/`<option>` — an HTML-AAM name
/// from author, in the same family as `alt` on `<img>`, and the one home of
/// this fact for both the name of such an element and what it contributes to
/// someone else's name ([`contents`]).
///
/// Unlike `alt=""`, an empty `label` is **not** the empty alternative: HTML
/// defines an option's label as its `label` attribute "if there is one and its
/// value is not the empty string", otherwise the element's text. Measured in
/// Chrome 139 for both tags — `<option label="">t</option>` is named "t", and
/// `<optgroup label="">` is nameless rather than named from its options.
fn label_attr(el: &crate::dom::Element) -> Option<String> {
    el.attr("label")
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// The *author-supplied* name — `aria-label`, `aria-labelledby`, `title` — with
/// no native step. This is the sub-algorithm the HTML-AAM's conditional role
/// mappings ask about ("if the element has an accessible name"): they apply to
/// elements with no native name source, so for them the two agree, and asking
/// only this way keeps role resolution free of any dependency on the role
/// resolution of descendants.
pub fn author_name(
    doc: &Document,
    el: &crate::dom::Element,
    styles: Option<&Styles>,
) -> Option<String> {
    trimmed_attr(el, "aria-label")
        .or_else(|| labelledby_text(doc, el, styles))
        .or_else(|| trimmed_attr(el, "title"))
}

/// Roles whose accessible name is computed from descendant text — the
/// "Name From: contents" set of WAI-ARIA 1.2 (roles listed with
/// `nameFrom: contents` in §5.4 Definition of Roles). Only these take the
/// subtree-text fallback; container roles (`table`, `rowgroup`, `list`,
/// `listitem`, `paragraph`, `navigation`, `region`, `group`, …) are absent,
/// so they yield no native name instead of echoing their whole subtree.
const NAME_FROM_CONTENTS: &[&str] = &[
    "button",
    "cell",
    "checkbox",
    "columnheader",
    "gridcell",
    "heading",
    "link",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "option",
    "radio",
    "row",
    "rowheader",
    "switch",
    "tab",
    "tooltip",
    "treeitem",
];

/// Whether the node's effective role is in [`NAME_FROM_CONTENTS`].
fn names_from_contents(doc: &Document, id: NodeId) -> bool {
    matches!(crate::ax::role(doc, id), Some(r) if NAME_FROM_CONTENTS.contains(&r))
}

fn input_name(
    doc: &Document,
    id: NodeId,
    el: &crate::dom::Element,
    styles: Option<&Styles>,
) -> Option<String> {
    let t = el
        .attr("type")
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_else(|| "text".to_string());
    match t.as_str() {
        "button" | "submit" | "reset" => el.attr("value").map(str::to_string),
        "image" => el.attr("alt").map(str::to_string),
        _ => control_label_text(doc, id, el, styles),
    }
}

fn control_label_text(
    doc: &Document,
    control_id: NodeId,
    el: &crate::dom::Element,
    styles: Option<&Styles>,
) -> Option<String> {
    let target = el.attr("id");
    let mut for_match: Option<NodeId> = None;
    let mut label_stack: Vec<NodeId> = Vec::new();
    let mut wrapping_label: Option<NodeId> = None;
    doc.walk(None, &mut |ev, e| match ev {
        WalkEvent::Enter(id) => {
            if let NodeKind::Element(elx) = &e.kind {
                if elx.name == "label" {
                    label_stack.push(id);
                    if let Some(t) = target {
                        if for_match.is_none() && elx.attr("for") == Some(t) {
                            for_match = Some(id);
                        }
                    }
                } else if id == control_id && wrapping_label.is_none() {
                    wrapping_label = label_stack.first().copied();
                }
            }
        }
        WalkEvent::Exit(id) => {
            if label_stack.last() == Some(&id) {
                label_stack.pop();
            }
        }
    });
    for_match
        .or(wrapping_label)
        .map(|id| contents::element_name(doc, id, styles, &[control_id]))
}

fn fieldset_legend_text(
    doc: &Document,
    fieldset_id: NodeId,
    styles: Option<&Styles>,
) -> Option<String> {
    let mut found: Option<NodeId> = None;
    doc.walk(Some(fieldset_id), &mut |ev, e| {
        if found.is_some() {
            return;
        }
        if let (WalkEvent::Enter(id), NodeKind::Element(el)) = (ev, &e.kind) {
            if id != fieldset_id && el.name == "legend" {
                found = Some(id);
            }
        }
    });
    found.map(|id| contents::element_name(doc, id, styles, &[]))
}

fn find_by_id(doc: &Document, target: &str) -> Option<NodeId> {
    let mut found = None;
    doc.walk(None, &mut |ev, e| {
        if found.is_some() {
            return;
        }
        if let (WalkEvent::Enter(id), NodeKind::Element(el)) = (ev, &e.kind) {
            if el.attr("id") == Some(target) {
                found = Some(id);
            }
        }
    });
    found
}

fn collapse_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod contents_tests;

#[cfg(test)]
mod option_tests;

#[cfg(test)]
mod reference_tests;
