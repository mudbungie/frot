//! State pseudo-classes whose answer is a function of the DOM alone.
//!
//! Each rule below is what Chrome 158 answered for a page nobody touched
//! (`tests/fixtures/fidelity/state.html` holds the cases). They read
//! attributes, never script-set properties: `input.checked = true` leaves no
//! mark in the tree, so it is not seen here (`bl-bc6d`).

use super::control;
use super::focus::{focus_within, focused};
use super::selector::ancestors;
use crate::dom::{Document, Element, NodeId, NodeKind, WalkEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// `:root`: the document's root element.
    Root,
    /// `:empty`: no element child and no text, whitespace included; comments
    /// do not count.
    Empty,
    /// `:link` / `:any-link`: an `<a>` or `<area>` with `href`. No link is
    /// visited in an unattended load, so the two are one.
    AnyLink,
    /// `:focus` / `:focus-visible` ([`super::focus`]).
    Focus,
    FocusWithin,
    /// `:target`: the document's target element ([`Document::target`]).
    Target,
    Checked,
    Disabled,
    Enabled,
    Required,
    Optional,
    ReadOnly,
    ReadWrite,
    /// `:open`: a `<details>` or `<dialog>` with `open`.
    Open,
    /// `:defined` ([`control::defined`]).
    Defined,
    /// `:default`, `:indeterminate`, `:placeholder-shown` ([`super::control`]).
    Default,
    Indeterminate,
    PlaceholderShown,
    /// `:paused`: every `<video>` and `<audio>` — nothing plays unattended.
    Paused,
    /// `:muted`: a `<video>` or `<audio>` with `muted`.
    Muted,
}

/// `<input>` types `required` does not apply to.
const NEVER_REQUIRED: [&str; 7] = [
    "hidden", "range", "color", "submit", "image", "reset", "button",
];
/// `<input>` types that hold no editable text besides those.
const NEVER_TEXT: [&str; 3] = ["checkbox", "radio", "file"];

impl State {
    /// The state pseudo-class called `name` (lower-cased), if it is one.
    pub fn named(name: &str) -> Option<State> {
        Some(match name {
            "root" => State::Root,
            "empty" => State::Empty,
            "link" | "any-link" | "-webkit-any-link" => State::AnyLink,
            "focus" | "focus-visible" => State::Focus,
            "focus-within" => State::FocusWithin,
            "target" => State::Target,
            "checked" => State::Checked,
            "disabled" => State::Disabled,
            "enabled" => State::Enabled,
            "required" => State::Required,
            "optional" => State::Optional,
            "read-only" => State::ReadOnly,
            "read-write" => State::ReadWrite,
            "open" => State::Open,
            "defined" => State::Defined,
            "default" => State::Default,
            "indeterminate" => State::Indeterminate,
            "placeholder-shown" => State::PlaceholderShown,
            "paused" => State::Paused,
            "muted" => State::Muted,
            _ => return None,
        })
    }

    /// Whether the element `el` at `id` is in this state.
    pub fn holds(self, doc: &Document, id: NodeId, el: &Element) -> bool {
        let name = el.name.as_str();
        match self {
            State::Root => doc.node(id).parent == Some(Document::ROOT),
            State::Empty => doc
                .node(id)
                .children
                .iter()
                .all(|&c| match &doc.node(c).kind {
                    NodeKind::Text(t) => t.is_empty(),
                    kind => matches!(kind, NodeKind::Comment(_)),
                }),
            State::AnyLink => {
                let href = el.attr("href").or(el.attr("xlink:href"));
                matches!(name, "a" | "area") && href.is_some()
            }
            State::Focus => focused(doc, id, el),
            State::FocusWithin => focus_within(doc, id),
            State::Target => doc.target() == Some(id),
            State::Checked => checked(doc, id, el),
            State::Disabled => disabled(doc, id, el),
            State::Enabled => {
                let can = matches!(
                    name,
                    "button" | "input" | "select" | "textarea" | "fieldset" | "optgroup" | "option"
                );
                can && !disabled(doc, id, el)
            }
            State::Required => required(el),
            State::Optional => {
                matches!(name, "input" | "select" | "textarea" | "button") && !required(el)
            }
            State::ReadOnly => !read_write(doc, id, el),
            State::ReadWrite => read_write(doc, id, el),
            State::Open => matches!(name, "details" | "dialog") && has(el, "open"),
            State::Defined => control::defined(doc, id, el),
            State::Default => control::default(doc, id, el),
            State::Indeterminate => control::indeterminate(doc, id, el),
            State::PlaceholderShown => control::placeholder_shown(doc, id, el),
            State::Paused => matches!(name, "video" | "audio"),
            State::Muted => matches!(name, "video" | "audio") && has(el, "muted"),
        }
    }
}

pub(super) fn has(el: &Element, attr: &str) -> bool {
    el.attr(attr).is_some()
}

pub(super) fn input_type(el: &Element) -> String {
    el.attr("type").unwrap_or_default().to_ascii_lowercase()
}

fn required(el: &Element) -> bool {
    let applies = match el.name.as_str() {
        "input" => !NEVER_REQUIRED.contains(&input_type(el).as_str()),
        name => matches!(name, "select" | "textarea"),
    };
    applies && has(el, "required")
}

/// `:read-write`: a text-holding `<input>` or a `<textarea>` that is neither
/// `readonly` nor disabled; any other element when it is editable — the
/// nearest `contenteditable` on it or above it says so.
fn read_write(doc: &Document, id: NodeId, el: &Element) -> bool {
    let text = match el.name.as_str() {
        "input" => {
            let kind = input_type(el);
            !NEVER_REQUIRED.contains(&kind.as_str()) && !NEVER_TEXT.contains(&kind.as_str())
        }
        "textarea" => true,
        _ => {
            let mut chain = std::iter::once((id, el)).chain(ancestors(doc, id));
            return chain.find_map(|(_, e)| editing(e)).unwrap_or(false);
        }
    };
    text && !has(el, "readonly") && !disabled(doc, id, el)
}

/// What `el`'s own `contenteditable` says, if it says anything.
pub(super) fn editing(el: &Element) -> Option<bool> {
    match el.attr("contenteditable")?.to_ascii_lowercase().as_str() {
        "" | "true" | "plaintext-only" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// `:disabled`. A button, input, select, textarea or fieldset: its own
/// `disabled`, or a disabled `<fieldset>` above it — unless it sits inside
/// that fieldset's first `<legend>` child. An option or optgroup: its own
/// `disabled`, a disabled optgroup above it, or a disabled owner select.
pub(super) fn disabled(doc: &Document, id: NodeId, el: &Element) -> bool {
    match el.name.as_str() {
        "option" | "optgroup" => {
            let owner = owner_select(doc, id);
            option_off(doc, id, el) || owner.is_some_and(|(s, e)| disabled(doc, s, e))
        }
        "button" | "input" | "select" | "textarea" | "fieldset" => {
            let mut below = id;
            let fieldset = |(a, e): (NodeId, &Element)| {
                let from = std::mem::replace(&mut below, a);
                e.name == "fieldset" && has(e, "disabled") && first_legend(doc, a) != Some(from)
            };
            has(el, "disabled") || ancestors(doc, id).any(fieldset)
        }
        _ => false,
    }
}

fn first_legend(doc: &Document, fieldset: NodeId) -> Option<NodeId> {
    let mut kids = doc.node(fieldset).children.iter().copied();
    kids.find(|&c| matches!(&doc.node(c).kind, NodeKind::Element(e) if e.name == "legend"))
}

fn owner_select(doc: &Document, id: NodeId) -> Option<(NodeId, &Element)> {
    ancestors(doc, id).find(|(_, e)| e.name == "select")
}

/// An option's own `disabled`, or a disabled optgroup between it and its
/// select. This — not `:disabled` — is what keeps a select from picking it.
fn option_off(doc: &Document, id: NodeId, el: &Element) -> bool {
    let mut groups = ancestors(doc, id).take_while(|(_, e)| e.name != "select");
    has(el, "disabled") || groups.any(|(_, e)| e.name == "optgroup" && has(e, "disabled"))
}

/// `:checked`: a checkbox with `checked`; a radio with `checked` that no later
/// radio of its group overrides; a selected option.
fn checked(doc: &Document, id: NodeId, el: &Element) -> bool {
    match (el.name.as_str(), input_type(el).as_str()) {
        ("input", "checkbox") => has(el, "checked"),
        ("input", "radio") => has(el, "checked") && last_of_group(doc, id, el),
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
/// own `selected`. Otherwise the select picks one: the last option with
/// `selected`, else — unless it is a list box (`size` above 1) — the first
/// that is not [`option_off`].
fn selected(doc: &Document, id: NodeId, el: &Element) -> bool {
    let Some((select, sel)) = owner_select(doc, id).filter(|(_, s)| !has(s, "multiple")) else {
        return has(el, "selected");
    };
    // (id, has `selected`, cannot be the default pick), in tree order.
    let mut options = Vec::new();
    doc.walk(Some(select), &mut |ev, entry| {
        if let (WalkEvent::Enter(n), NodeKind::Element(e)) = (ev, &entry.kind) {
            if e.name == "option" {
                options.push((n, has(e, "selected"), option_off(doc, n, e)));
            }
        }
    });
    let marked = options.iter().rev().find(|o| o.1);
    let list_box = sel.attr("size").and_then(|s| s.trim().parse().ok()) > Some(1u32);
    let default = options.iter().find(|o| !list_box && !o.2);
    marked.or(default).is_some_and(|o| o.0 == id)
}

#[cfg(test)]
mod tests;
