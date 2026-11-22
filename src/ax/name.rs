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
//!    - `<table>` — its first `<caption>` child, read as a descendant
//!      ([`contents::descendant_name`]); with no caption, its `summary`
//!      attribute ([`summary`])
//!    - `<svg>` and every SVG part (`<g>`, a shape, `<use>`) — its `<title>`
//!      child ([`contents::svg_title`])
//!    - `<optgroup>` and `<option>` — the `label` content attribute
//!      ([`label_attr`]); an `<option>` without one falls back to its contents
//!    - everything else — the element's descendant text *alternatives* (the
//!      §2F recursion, [`contents`]), but *only* when the element's role is
//!      name-from-contents (see [`NAME_FROM_CONTENTS`]); containers (list,
//!      paragraph, nav, a plain table's row, …) yield no native name
//! 4. `title` — last, only where the node's own name is at stake, and never
//!    for a `term` ([`title_names`]). `title`
//!    is positional (`contents/walk.rs`): a `<label>`/`<legend>` and every node
//!    inside an `aria-labelledby` reference read it too, a descendant read for
//!    its contribution does not.
//!
//! Returns the trimmed result, or `None` if nothing produced text. With
//! `Some(styles)` (`--css`), the text-content sources drop `display:none`
//! subtrees and include `::before`/`::after` generated content.

mod contents;
mod related;

use crate::css::Styles;
use crate::dom::{Document, NodeId, NodeKind, WalkEvent};
use related::{control_label_text, fieldset_legend_text, table_caption_text};

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
    trimmed_attr(el, "title").filter(|_| title_names(doc, id))
}

/// Whether `title` is a name source for the node at all. It is not for a
/// `term`: measured in Chrome 158 (`bl-0794`), `<dt title=T></dt>`,
/// `<dfn title=T></dfn>` and `<span role=term title=T></span>` are all
/// `term ""`, while an empty heading, link, button, tab, grid row or cell
/// with a `title` is named by it. `aria-label` still names a term.
fn title_names(doc: &Document, id: NodeId) -> bool {
    crate::ax::role(doc, id) != Some("term")
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
        "table" => table_caption_text(doc, id, styles).or_else(|| summary(el)),
        n if super::role::svg_titled(n) => contents::svg_title(doc, id),
        "optgroup" => label_attr(el),
        "option" => label_attr(el).or_else(|| Some(contents_text(doc, id, styles))),
        _ if names_from_contents(doc, id) => Some(contents_text(doc, id, styles)),
        _ => None,
    }
}

/// A `<table>`'s `summary` attribute: its name when it has no `<caption>`
/// child. Measured in Chrome 158 (`bl-2b09`): `aria-label` and a caption
/// outrank it, it outranks `title`, and a caption that is present but empty
/// still shuts it out (`<table summary=S><caption></caption>` is named "").
/// It is the `<table>` element's, whatever its role — `role=grid` reads it,
/// a `<div role=table summary=S>` does not.
fn summary(el: &crate::dom::Element) -> Option<String> {
    el.attr("summary").map(str::to_string)
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
/// subtree-text fallback (`row` too, conditionally: [`names_from_contents`]);
/// container roles (`table`, `rowgroup`, `list`,
/// `listitem`, `paragraph`, `navigation`, `region`, `group`, …) are absent,
/// so they yield no native name instead of echoing their whole subtree.
///
/// `term` is the one entry ARIA does not list (it is name-from-author there):
/// Chrome 156 names a `<dt>`, a `<dfn>` and a `role=term` from their contents
/// (`bl-8ad7`), and the browser is the oracle. `definition` stays out — Chrome
/// names a `<dd>` nothing. The five `doc-*` rows are the DPUB-ARIA roles
/// Chrome 158 names from contents (`bl-fc83`): the three link kinds, the
/// back-link, and the subtitle; the other 36 are sections and name nothing.
const NAME_FROM_CONTENTS: &[&str] = &[
    "button",
    "cell",
    "checkbox",
    "columnheader",
    "doc-backlink",
    "doc-biblioref",
    "doc-glossref",
    "doc-noteref",
    "doc-subtitle",
    "gridcell",
    "heading",
    "link",
    "menuitem",
    "menuitemcheckbox",
    "menuitemradio",
    "option",
    "radio",
    "rowheader",
    "switch",
    "tab",
    "term",
    "tooltip",
    "treeitem",
];

/// Whether the node's effective role is in [`NAME_FROM_CONTENTS`] — and, for
/// a `row`, whether its table is a `grid`/`treegrid`. ARIA lists `row` as
/// name-from-contents without condition, but a browser computes that name only
/// where a row can be the thing in focus: measured in Chrome 156 (`bl-3072`),
/// a row of a `table` is `""` — `<tr>` or `role=row`, focusable or not — while
/// the same row in a grid is named from its cells.
fn names_from_contents(doc: &Document, id: NodeId) -> bool {
    match crate::ax::role(doc, id) {
        Some("row") => crate::ax::role::in_grid(doc, id),
        Some(r) => NAME_FROM_CONTENTS.contains(&r),
        None => false,
    }
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

#[cfg(test)]
mod structure_tests;

#[cfg(test)]
mod svg_tests;

#[cfg(test)]
mod title_tests;
