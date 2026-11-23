//! Form-control and element states a page shows before anyone touches it:
//! `:default`, `:indeterminate`, `:placeholder-shown` and `:defined`.
//! Measured in Chrome 158 (`bl-2a85`); like the rest of [`super::state`] they
//! read attributes, never script-set properties.
//!
//! - **`:default`** — a checkbox or radio with `checked`, an option with
//!   `selected` (not one a select picks by default), and a form's default
//!   button: the first submit button in tree order that the form owns, even
//!   a disabled one. A submit button is a `<button>` whose `type` is not
//!   `button` or `reset`, or an `<input type=submit|image>`.
//! - **`:indeterminate`** — a `<progress>` without `value`; a radio whose group
//!   has no radio with `checked` (a radio with no name is a group of one).
//! - **`:placeholder-shown`** — an `<input>` that takes a placeholder (every
//!   type but `hidden`, the date and time types, `range`, `color`,
//!   `checkbox`, `radio`, `file` and the buttons) or a `<textarea>`, with a
//!   `placeholder` (even an empty one) and an empty value. The value is the
//!   `value` attribute as the type sanitizes it: line breaks removed,
//!   surrounding white space too for `email` and `url`, and a `number` that
//!   is not a valid number is empty. A textarea's value is its text.
//! - **`:defined`** — every element but an HTML one whose name is a valid
//!   custom element name (a lower-case letter first, a `-`, not one of eight
//!   reserved names), or that carries `is` (no element is registered:
//!   frot has no `customElements`). SVG and MathML elements are defined.

use super::lang::foreign;
use super::state::{group, has, input_type};
use crate::dom::{Document, Element, NodeId, NodeKind, WalkEvent};

/// Names the custom element grammar matches but HTML reserves.
const RESERVED: [&str; 8] = [
    "annotation-xml",
    "color-profile",
    "font-face",
    "font-face-src",
    "font-face-uri",
    "font-face-format",
    "font-face-name",
    "missing-glyph",
];

/// `<input>` types that show no placeholder.
const NO_PLACEHOLDER: [&str; 15] = [
    "hidden",
    "date",
    "month",
    "week",
    "time",
    "datetime-local",
    "range",
    "color",
    "checkbox",
    "radio",
    "file",
    "submit",
    "image",
    "reset",
    "button",
];

pub(super) fn defined(doc: &Document, id: NodeId, el: &Element) -> bool {
    foreign(doc, id) || !(has(el, "is") || custom(&el.name))
}

/// A valid custom element name: a lower-case ASCII letter first, a `-`
/// somewhere, not reserved. (The parser lower-cases tag names, and Chrome
/// takes any other character: `<x-y!>` is a custom element.)
fn custom(name: &str) -> bool {
    let first = name.chars().next().is_some_and(|c| c.is_ascii_lowercase());
    first && name.contains('-') && !RESERVED.contains(&name)
}

pub(super) fn default(doc: &Document, id: NodeId, el: &Element) -> bool {
    match (el.name.as_str(), input_type(el).as_str()) {
        ("input", "checkbox" | "radio") => has(el, "checked"),
        ("option", _) => has(el, "selected"),
        _ if submit(el) => doc.form_owner(id).is_some_and(|f| {
            let mut first = None;
            elements(doc, &mut |n, e| {
                if first.is_none() && submit(e) && doc.form_owner(n) == Some(f) {
                    first = Some(n);
                }
            });
            first == Some(id)
        }),
        _ => false,
    }
}

fn submit(el: &Element) -> bool {
    match (el.name.as_str(), input_type(el).as_str()) {
        ("button", kind) => !matches!(kind, "button" | "reset"),
        ("input", kind) => matches!(kind, "submit" | "image"),
        _ => false,
    }
}

pub(super) fn indeterminate(doc: &Document, id: NodeId, el: &Element) -> bool {
    match (el.name.as_str(), input_type(el).as_str()) {
        ("progress", _) => !has(el, "value"),
        ("input", "radio") => {
            let Some(mine) = group(doc, id, el) else {
                return !has(el, "checked");
            };
            let mut checked = false;
            elements(doc, &mut |n, e| {
                let radio = e.name == "input" && input_type(e) == "radio";
                checked |= radio && has(e, "checked") && group(doc, n, e) == Some(mine);
            });
            !checked
        }
        _ => false,
    }
}

pub(super) fn placeholder_shown(doc: &Document, id: NodeId, el: &Element) -> bool {
    let kind = input_type(el);
    let value = match el.name.as_str() {
        "textarea" => doc.text_content(id),
        "input" if !NO_PLACEHOLDER.contains(&kind.as_str()) => {
            sanitized(&kind, el.attr("value").unwrap_or_default())
        }
        _ => return false,
    };
    has(el, "placeholder") && value.is_empty()
}

/// An `<input>`'s value as its type sanitizes it (module doc).
fn sanitized(kind: &str, raw: &str) -> String {
    let flat: String = raw.chars().filter(|&c| c != '\n' && c != '\r').collect();
    match kind {
        "email" | "url" => flat
            .trim_matches(|c: char| c.is_ascii_whitespace())
            .to_owned(),
        "number" if !number(raw) => String::new(),
        _ => flat,
    }
}

/// HTML's valid floating-point number: `-`? digits with an optional
/// fraction (or a bare fraction), then an optional exponent.
fn number(s: &str) -> bool {
    let digits = |s: &str| s.len() - s.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let s = s.strip_prefix('-').unwrap_or(s);
    let (whole, s) = s.split_at(digits(s));
    let (mantissa, s) = match s.strip_prefix('.') {
        Some(rest) => (digits(rest) > 0, &rest[digits(rest)..]),
        None => (!whole.is_empty(), s),
    };
    let exponent = match s.strip_prefix(['e', 'E']) {
        Some(rest) => {
            let rest = rest.strip_prefix(['+', '-']).unwrap_or(rest);
            !rest.is_empty() && digits(rest) == rest.len()
        }
        None => s.is_empty(),
    };
    mantissa && exponent
}

/// Call `f` on every element in tree order.
fn elements(doc: &Document, f: &mut dyn FnMut(NodeId, &Element)) {
    doc.walk(None, &mut |ev, entry| {
        if let (WalkEvent::Enter(n), NodeKind::Element(e)) = (ev, &entry.kind) {
            f(n, e);
        }
    });
}

#[cfg(test)]
mod tests;
