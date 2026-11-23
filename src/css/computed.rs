//! Reducing the declarations that reached an element to a [`ComputedStyle`].
//!
//! The cascade proper ([`super::cascade`]) decides *which* declarations apply;
//! this module decides what they compute to. A property's winner is the
//! applied declaration with the highest `(important, inline, specificity,
//! source-order)` key — the real CSS cascade order, compared in that sequence.

use super::content;
use super::selector::{Pseudo, Specificity};
use super::{ComputedStyle, Display, FlexDirection, Visibility};
use crate::dom::Element;
use crate::tags::is_block;

pub struct Applied {
    pub pseudo: Option<Pseudo>,
    pub important: bool,
    pub inline: bool,
    pub spec: Specificity,
    pub order: usize,
    pub name: String,
    pub value: String,
}

type Key = (bool, bool, Specificity, usize);

impl Applied {
    fn key(&self) -> Key {
        (self.important, self.inline, self.spec, self.order)
    }
}

/// Fold `applied` into the element's [`ComputedStyle`].
pub fn reduce(
    applied: &[Applied],
    el: &Element,
    parent_vis: Visibility,
    js: bool,
) -> ComputedStyle {
    ComputedStyle {
        display: display(applied, el, js),
        order: order_value(applied),
        flex_direction: flex_direction(applied),
        visibility: visibility(applied, parent_vis),
        before: generated(applied, Pseudo::Before, el),
        after: generated(applied, Pseudo::After, el),
        before_display: pseudo_display(applied, Pseudo::Before),
        after_display: pseudo_display(applied, Pseudo::After),
        out_of_flow: out_of_flow(applied, None),
        pseudo_out_of_flow: [Pseudo::Before, Pseudo::After].map(|p| out_of_flow(applied, Some(p))),
    }
}

/// The winning `order`: the winning declaration parsed as an `i32`; a
/// non-integer or absent value is `0` (the initial value). Only flex layout
/// reads it (`layout.md` §5).
fn order_value(applied: &[Applied]) -> i32 {
    winner(applied, None, "order")
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0)
}

/// The winning `flex-direction`, parsed (and coerced) by [`FlexDirection::parse`];
/// an absent value is [`FlexDirection::Row`]. Only flex layout reads it.
fn flex_direction(applied: &[Applied]) -> FlexDirection {
    winner(applied, None, "flex-direction")
        .map(FlexDirection::parse)
        .unwrap_or_default()
}

/// The winning `display`, blockified when the box is out of flow
/// ([`out_of_flow`]): a matched author/inline rule ([`display_winner`]),
/// else the tag's UA-implicit display — but `<noscript>` is UA-implicit `none`
/// once `--js` ran (js.md §4: scripting hides noscript), and `[hidden]` is
/// UA-implicit `none` always ([`Element::hidden`]). Both are author-overridable:
/// a matched author/inline `display` is the `Some` arm and wins, exactly as a
/// UA-origin declaration loses to the author origin.
///
/// [`Element::hidden_input`] is the exception that proves it: the UA sheet
/// marks `input[type=hidden]` `!important`, so it is decided *before* the
/// author cascade is consulted and nothing can render it.
fn display(applied: &[Applied], el: &Element, js: bool) -> Display {
    if el.hidden_input() {
        return Display::None;
    }
    let display = match display_winner(applied, None) {
        Some(d) => d,
        None if el.hidden() || (js && el.name == "noscript") => Display::None,
        None => implicit_display(&el.name),
    };
    display.blockified(out_of_flow(applied, None))
}

/// The computed `display` of a pseudo-element's box: its own declaration,
/// else `inline`, blockified when the pseudo-element is out of flow.
fn pseudo_display(applied: &[Applied], pseudo: Pseudo) -> Display {
    display_winner(applied, Some(pseudo))
        .unwrap_or_default()
        .blockified(out_of_flow(applied, Some(pseudo)))
}

/// Whether the box is out of flow: absolutely positioned (`position:
/// absolute | fixed`) or floated. `relative` and `sticky` stay in flow.
/// Measured in Chrome 158 (`bl-346e`): each computes `display: block` for a
/// span and puts it on a line of its own in `innerText`.
fn out_of_flow(applied: &[Applied], pseudo: Option<Pseudo>) -> bool {
    let is = |name: &str, values: &[&str]| {
        winner(applied, pseudo, name)
            .is_some_and(|v| values.contains(&v.trim().to_ascii_lowercase().as_str()))
    };
    is("position", &["absolute", "fixed"])
        || is("float", &["left", "right", "inline-start", "inline-end"])
}

/// UA-implicit `display` when no rule sets it: `datalist` → none, `li` →
/// list-item, any other block-level tag ([`is_block`]) → block, everything else
/// → inline.
///
/// `datalist` is the one tag the UA sheet hides outright
/// (`datalist { display: none }`, HTML Rendering §15.5.1). It lives here rather
/// than beside `[hidden]` above because it is keyed on the *tag*, like every
/// other entry in this map — and it is an ordinary UA declaration, so an author
/// rule beats it and the `Some` arm above never reaches this function.
/// Measured, Chrome 139 (`bl-66ed`): a bare `<datalist>`'s options compute
/// `display: block`, get zero client rects, contribute nothing to `innerText`,
/// and are `ignored` in the AX tree; add `datalist { display: block }` and all
/// four reverse. Being a declaration, it needs the cascade — without `--css`
/// the suggestions stay in `text` and `ax` (`layout.md` §2.3).
fn implicit_display(name: &str) -> Display {
    match name {
        "datalist" => Display::None,
        "li" => Display::ListItem,
        n if is_block(n) => Display::Block,
        _ => Display::Inline,
    }
}

fn winner<'b>(applied: &'b [Applied], pseudo: Option<Pseudo>, name: &str) -> Option<&'b str> {
    applied
        .iter()
        .filter(|a| a.pseudo == pseudo && a.name == name)
        .max_by_key(|a| a.key())
        .map(|a| a.value.as_str())
}

/// The winning `display` among the declarations Chrome would keep: one
/// whose value is not a `display` value at all ([`Display::parse`]) was
/// dropped when the sheet was parsed, so the valid declaration it would have
/// beaten wins instead — `display: inline-block; display: bogus` is an
/// inline-block (Chrome 158, `bl-e3a0`).
///
/// `display: revert` is kept and computes to nothing here, which leaves the
/// caller with the user agent's value, as it asks.
fn display_winner(applied: &[Applied], pseudo: Option<Pseudo>) -> Option<Display> {
    applied
        .iter()
        .filter(|a| a.pseudo == pseudo && a.name == "display")
        .filter_map(|a| {
            let display = Display::parse(&a.value);
            (display.is_some() || Display::reverts(&a.value)).then_some((a.key(), display))
        })
        .max_by_key(|(key, _)| *key)
        .and_then(|(_, display)| display)
}

fn visibility(applied: &[Applied], parent_vis: Visibility) -> Visibility {
    match winner(applied, None, "visibility") {
        Some(v) => {
            let v = v.trim().to_ascii_lowercase();
            if v == "hidden" || v == "collapse" {
                Visibility::Hidden
            } else {
                Visibility::Visible
            }
        }
        None => parent_vis,
    }
}

fn generated(applied: &[Applied], pseudo: Pseudo, el: &Element) -> Option<String> {
    if display_winner(applied, Some(pseudo)) == Some(Display::None) {
        return None;
    }
    let raw = winner(applied, Some(pseudo), "content")?;
    let lowered = raw.trim().to_ascii_lowercase();
    if lowered == "none" || lowered == "normal" {
        return None;
    }
    Some(content::string(raw, el))
}
