//! Reducing the declarations that reached an element to a [`ComputedStyle`].
//!
//! The cascade proper ([`super::cascade`]) decides *which* declarations apply;
//! this module decides what they compute to. A property's winner is the
//! applied declaration with the highest `(important, inline, specificity,
//! source-order)` key — the real CSS cascade order, compared in that sequence.

use super::content;
use super::selector::{Pseudo, Specificity};
use super::transform::TextTransform;
use super::{ComputedStyle, Display, FlexDirection, Flow, Visibility};
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
    pub(super) fn key(&self) -> Key {
        (self.important, self.inline, self.spec, self.order)
    }
}

/// Fold `applied` into the element's [`ComputedStyle`]; `parent` supplies the
/// inherited properties.
/// `shown` says a script put the element in the top layer as a popover
/// ([`crate::dom::Document::popover_open`]).
pub fn reduce(
    applied: &[Applied],
    el: &Element,
    parent: &ComputedStyle,
    js: bool,
    shown: bool,
) -> ComputedStyle {
    let transform = TextTransform::of(
        ranked(applied, None, "text-transform"),
        el,
        parent.text_transform,
    );
    ComputedStyle {
        display: display(applied, el, js, parent.display, shown),
        order: order_value(applied),
        flex_direction: flex_direction(applied),
        visibility: visibility(applied, parent.visibility),
        before: generated(applied, Pseudo::Before, el),
        after: generated(applied, Pseudo::After, el),
        before_display: pseudo_display(applied, Pseudo::Before),
        after_display: pseudo_display(applied, Pseudo::After),
        flow: flow(applied, None),
        unmarked: super::list::unmarked(applied, parent.unmarked),
        pseudo_out_of_flow: [Pseudo::Before, Pseudo::After]
            .map(|p| flow(applied, Some(p)) != Flow::In),
        text_transform: transform,
        pseudo_transform: [Pseudo::Before, Pseudo::After]
            .map(|p| transform.pseudo(ranked(applied, Some(p), "text-transform"))),
        text: None,
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
/// ([`out_of_flow`]) or a flex or grid item (its parent's `display`
/// [`Display::has_items`]): a matched author/inline rule ([`display_winner`];
/// `inherit` is the parent's), else the tag's UA-implicit display — but
/// `<noscript>` is UA-implicit `none` once `--js` ran (js.md §4: scripting
/// hides noscript), and `[hidden]`, a `<dialog>` without `open` and a
/// `[popover]` no script has shown (`bl-bc6d`; an open dialog with
/// `popover` stays) are UA-implicit `none` always ([`Element::hidden`]). All are author-overridable:
/// a matched author/inline `display` is the `Some` arm and wins, exactly as a
/// UA-origin declaration loses to the author origin.
///
/// [`Element::hidden_input`] and an `<audio>` without `controls` are the
/// exceptions that prove it: the UA sheet marks both `display: none
/// !important`, so they are decided *before* the author cascade is consulted
/// and nothing can render them (Chrome 158, `bl-9dec`).
fn display(applied: &[Applied], el: &Element, js: bool, parent: Display, shown: bool) -> Display {
    let controls = el.attr("controls").is_some();
    if el.hidden_input() || (el.name == "audio" && !controls) {
        return Display::None;
    }
    let (dialog, open) = (el.name == "dialog", el.attr("open").is_some());
    let popover = el.attr("popover").is_some();
    let closed = !(shown || dialog && open) && (dialog || popover);
    let display = match display_winner(applied, None, parent) {
        Some(d) => d,
        None if el.hidden() || closed || (js && el.name == "noscript") => Display::None,
        None => implicit_display(&el.name),
    };
    display.blockified(flow(applied, None) != Flow::In || parent.has_items())
}

/// The computed `display` of a pseudo-element's box: its own declaration,
/// else `inline`, blockified when the pseudo-element is out of flow.
fn pseudo_display(applied: &[Applied], pseudo: Pseudo) -> Display {
    display_winner(applied, Some(pseudo), Display::Inline)
        .unwrap_or_default()
        .blockified(flow(applied, Some(pseudo)) != Flow::In)
}

/// How the box stands in flow ([`Flow`]): absolutely positioned (`position:
/// absolute | fixed`), else floated, else in flow. Measured in Chrome 158
/// (`bl-346e`): either way out of flow computes `display: block` for a span
/// and puts it on a line of its own in `innerText`.
fn flow(applied: &[Applied], pseudo: Option<Pseudo>) -> Flow {
    let is = |name: &str, values: &[&str]| {
        winner(applied, pseudo, name)
            .is_some_and(|v| values.contains(&v.trim().to_ascii_lowercase().as_str()))
    };
    if is("position", &["absolute", "fixed"]) {
        Flow::Positioned
    } else if is("float", &["left", "right", "inline-start", "inline-end"]) {
        Flow::Float
    } else {
        Flow::In
    }
}

/// UA-implicit `display` when no rule sets it: `datalist`, `area` and `rp` →
/// none, `slot` → contents, `li` → list-item, any other block-level tag ([`is_block`]) → block, everything else
/// → inline.
///
/// `datalist`, `area` and `rp` are the tags the UA sheet hides outright
/// (`area, datalist, rp, … { display: none }`, HTML Rendering §15.3.1), and
/// `slot { display: contents }` boxes nothing of its own (Chrome 158,
/// `bl-9433`: `<p><area></p>` and `<p><slot></slot></p>` hold no line, and
/// `<ruby>base<rp>(</rp><rt>a</rt><rp>)</rp></ruby>` is `basea` in `innerText`). It lives here rather
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
        "datalist" | "area" | "rp" => Display::None,
        "slot" => Display::Contents,
        "li" => Display::ListItem,
        "table" => Display::Table,
        n if is_block(n) => Display::Block,
        _ => Display::Inline,
    }
}

/// Every value declared for `name`, the cascade's winner first.
fn ranked<'b>(
    applied: &'b [Applied],
    pseudo: Option<Pseudo>,
    name: &str,
) -> impl Iterator<Item = &'b str> {
    let mut hits: Vec<&Applied> = applied
        .iter()
        .filter(|a| a.pseudo == pseudo && a.name == name)
        .collect();
    hits.sort_by_key(|a| std::cmp::Reverse(a.key()));
    hits.into_iter().map(|a| a.value.as_str())
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
/// caller with the user agent's value, as it asks. `display: inherit` is the
/// `inherited` display — the parent's (Chrome 158, `bl-9dec`); a
/// pseudo-element's is not modelled and reads as `inline`.
fn display_winner(
    applied: &[Applied],
    pseudo: Option<Pseudo>,
    inherited: Display,
) -> Option<Display> {
    applied
        .iter()
        .filter(|a| a.pseudo == pseudo && a.name == "display")
        .filter_map(|a| {
            let inherit = a.value.trim().eq_ignore_ascii_case("inherit");
            let display = Display::parse(&a.value).or(inherit.then_some(inherited));
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
    if display_winner(applied, Some(pseudo), Display::Inline) == Some(Display::None) {
        return None;
    }
    let raw = winner(applied, Some(pseudo), "content")?;
    let lowered = raw.trim().to_ascii_lowercase();
    if lowered == "none" || lowered == "normal" {
        return None;
    }
    Some(content::string(raw, el))
}
