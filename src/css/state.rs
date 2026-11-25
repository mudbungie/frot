//! State pseudo-classes whose answer is a function of the DOM alone.
//!
//! Each rule below is what Chrome 158 answered for a page nobody touched
//! (`tests/fixtures/fidelity/state.html` holds the cases). They read
//! attributes, and the state a script keeps in the arena (`dom::live`):
//! checkedness and selectedness live in [`super::checked`].

use super::checked::{checked, option_off, owner_select};
use super::focus::{focus_within, focused};
use super::selector::ancestors;
use super::{control, validity};
use crate::dom::{Document, Element, NodeId, NodeKind};

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
    /// `:valid`/`:invalid`, `:in-range`/`:out-of-range` ([`super::validity`]).
    Valid,
    Invalid,
    InRange,
    OutOfRange,
    /// `:paused`: every `<video>` and `<audio>` — nothing plays unattended.
    Paused,
    /// `:muted`: a `<video>` or `<audio>` with `muted`.
    Muted,
    /// `:popover-open` / `:modal`: shown by a script ([`crate::dom::live`]).
    PopoverOpen,
    Modal,
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
            "valid" => State::Valid,
            "invalid" => State::Invalid,
            "in-range" => State::InRange,
            "out-of-range" => State::OutOfRange,
            "paused" => State::Paused,
            "muted" => State::Muted,
            "popover-open" => State::PopoverOpen,
            "modal" => State::Modal,
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
            State::Focus => focused(doc, id),
            State::FocusWithin => focus_within(doc, id),
            State::Target => doc.target() == Some(id),
            State::Checked => checked(doc, id, el),
            State::Disabled => disabled(doc, id, el),
            State::Enabled => {
                let can = matches!(
                    name,
                    "button" | "input" | "select" | "textarea" | "fieldset" | "optgroup" | "option"
                ) || doc.face(id);
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
            State::Valid => validity::valid(doc, id, el) == Some(true),
            State::Invalid => validity::valid(doc, id, el) == Some(false),
            State::InRange => validity::in_range(doc, id, el) == Some(true),
            State::OutOfRange => validity::in_range(doc, id, el) == Some(false),
            State::Paused => matches!(name, "video" | "audio"),
            State::Muted => matches!(name, "video" | "audio") && has(el, "muted"),
            State::PopoverOpen => has(el, "popover") && doc.popover_open(id),
            State::Modal => doc.modal(id),
        }
    }
}

pub(super) fn has(el: &Element, attr: &str) -> bool {
    el.attr(attr).is_some()
}

pub(super) fn input_type(el: &Element) -> String {
    el.attr("type").unwrap_or_default().to_ascii_lowercase()
}

pub(super) fn required(el: &Element) -> bool {
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
        _ => return editable(doc, id, el),
    };
    text && !has(el, "readonly") && !disabled(doc, id, el)
}

/// Whether `el` is editable: the nearest `contenteditable` on it or above it
/// says so.
pub fn editable(doc: &Document, id: NodeId, el: &Element) -> bool {
    let mut chain = std::iter::once((id, el)).chain(ancestors(doc, id));
    chain.find_map(|(_, e)| editing(e)).unwrap_or(false)
}

/// What `el`'s own `contenteditable` says, if it says anything.
pub(super) fn editing(el: &Element) -> Option<bool> {
    match el.attr("contenteditable")?.to_ascii_lowercase().as_str() {
        "" | "true" | "plaintext-only" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// `:disabled`. A button, input, select, textarea, fieldset or
/// form-associated custom element (Firefox 153, js.md §15.6): its own
/// `disabled`, or a disabled `<fieldset>` above it — unless it sits inside
/// that fieldset's first `<legend>` child. An option or optgroup: its own
/// `disabled`, a disabled optgroup above it, or a disabled owner select.
pub(super) fn disabled(doc: &Document, id: NodeId, el: &Element) -> bool {
    match el.name.as_str() {
        "option" | "optgroup" => {
            let owner = owner_select(doc, id);
            option_off(doc, id, el) || owner.is_some_and(|(s, e)| disabled(doc, s, e))
        }
        "button" | "input" | "select" | "textarea" | "fieldset" => listed(doc, id, el),
        _ => doc.face(id) && listed(doc, id, el),
    }
}

/// A listed control's own `disabled`, or a disabled `<fieldset>` above it
/// whose first `<legend>` does not hold it.
fn listed(doc: &Document, id: NodeId, el: &Element) -> bool {
    let mut below = id;
    let fieldset = |(a, e): (NodeId, &Element)| {
        let from = std::mem::replace(&mut below, a);
        e.name == "fieldset" && has(e, "disabled") && first_legend(doc, a) != Some(from)
    };
    has(el, "disabled") || ancestors(doc, id).any(fieldset)
}

fn first_legend(doc: &Document, fieldset: NodeId) -> Option<NodeId> {
    let mut kids = doc.node(fieldset).children.iter().copied();
    kids.find(|&c| matches!(&doc.node(c).kind, NodeKind::Element(e) if e.name == "legend"))
}

#[cfg(test)]
mod tests;
