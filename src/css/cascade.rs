//! Gather author CSS from the document, match it against every element with
//! full ancestor context, and cascade it into a [`Styles`] table.
//!
//! Origin/precedence is a deliberate simplification of the real cascade,
//! sufficient for visibility and generated content: a declaration wins on
//! `(important, inline, specificity, source-order)`, compared in that order.
//! `visibility` is inherited; `display` and generated `content` are not.

use super::parse::{parse_decls, Stylesheet};
use super::selector::{Pseudo, Specificity};
use super::{ComputedStyle, Display, FlexDirection, Styles, Visibility};
use crate::dom::{Document, Element, NodeId, NodeKind};
use crate::tags::is_block;

struct Applied {
    pseudo: Option<Pseudo>,
    important: bool,
    inline: bool,
    spec: Specificity,
    order: usize,
    name: String,
    value: String,
}

type Key = (bool, bool, Specificity, usize);

impl Applied {
    fn key(&self) -> Key {
        (self.important, self.inline, self.spec, self.order)
    }
}

pub fn compute(doc: &Document) -> Styles {
    compute_with(doc, &[], false)
}

/// Like [`compute`], but `external` raw CSS texts (fetched `<link>`
/// stylesheets) cascade ahead of the document's own `<style>` rules. `js` (did
/// `--js` run) flips `<noscript>` to hidden (js.md §4).
pub fn compute_with(doc: &Document, external: &[String], js: bool) -> Styles {
    let mut sheets: Vec<Stylesheet> = external.iter().map(|s| Stylesheet::parse(s)).collect();
    sheets.extend(
        doc.find_by_tag("style")
            .into_iter()
            .map(|id| Stylesheet::parse(&doc.text_content(id))),
    );
    cascade(doc, &sheets, js)
}

/// UA-implicit `display` plus inline `style=` only — **no** author `<style>`
/// blocks and **no** external sheets. This is the styles source layout uses
/// **without** `--css` (`layout.md` §3): `--css` everywhere means "apply author
/// CSS", so bare layout must ignore `<style>` exactly as it ignores `<link>`
/// sheets. Shares the [`cascade`] walk with [`compute_with`]; the only
/// difference is the empty sheet list.
pub fn compute_bare(doc: &Document, js: bool) -> Styles {
    cascade(doc, &[], js)
}

/// Cascade `sheets` (already gathered) over every element with full ancestor
/// context. The single per-element `resolve` walk shared by [`compute_with`]
/// (author + external sheets) and [`compute_bare`] (empty sheets); inline
/// `style=` and UA-implicit display are applied by `resolve` regardless of the
/// sheet list.
fn cascade(doc: &Document, sheets: &[Stylesheet], js: bool) -> Styles {
    let mut nodes = vec![ComputedStyle::default(); doc.len()];
    let mut ancestors: Vec<&Element> = Vec::new();
    for &root in doc.roots() {
        walk(doc, root, &mut ancestors, sheets, Visibility::Visible, js, &mut nodes);
    }
    Styles::from_nodes(nodes)
}

fn walk<'a>(
    doc: &'a Document,
    id: NodeId,
    ancestors: &mut Vec<&'a Element>,
    sheets: &[Stylesheet],
    parent_vis: Visibility,
    js: bool,
    nodes: &mut [ComputedStyle],
) {
    let entry = doc.node(id);
    let NodeKind::Element(el) = &entry.kind else {
        return;
    };
    let cs = resolve(el, ancestors, sheets, parent_vis, js);
    let vis = cs.visibility;
    nodes[id as usize] = cs;
    ancestors.push(el);
    for &c in &entry.children {
        walk(doc, c, ancestors, sheets, vis, js, nodes);
    }
    ancestors.pop();
}

fn resolve(
    el: &Element,
    ancestors: &[&Element],
    sheets: &[Stylesheet],
    parent_vis: Visibility,
    js: bool,
) -> ComputedStyle {
    let near_first: Vec<&Element> = ancestors.iter().rev().copied().collect();
    let mut applied: Vec<Applied> = Vec::new();
    let mut order = 0usize;
    for sheet in sheets {
        for rule in &sheet.rules {
            for sel in &rule.selectors {
                if !super::selector::matches(sel, el, &near_first) {
                    continue;
                }
                let (pseudo, spec) = (sel.pseudo(), sel.specificity());
                for d in &rule.decls {
                    applied.push(Applied {
                        pseudo,
                        important: d.important,
                        inline: false,
                        spec,
                        order,
                        name: d.name.clone(),
                        value: d.value.clone(),
                    });
                }
            }
            order += 1;
        }
    }
    if let Some(style) = el.attr("style") {
        for d in parse_decls(style) {
            applied.push(Applied {
                pseudo: None,
                important: d.important,
                inline: true,
                spec: Specificity::default(),
                order,
                name: d.name,
                value: d.value,
            });
        }
    }
    ComputedStyle {
        display: display(&applied, el, js),
        order: order_value(&applied),
        flex_direction: flex_direction(&applied),
        visibility: visibility(&applied, parent_vis),
        before: generated(&applied, Pseudo::Before, el),
        after: generated(&applied, Pseudo::After, el),
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

/// The winning `display`: a matched author/inline rule ([`Display::parse`]),
/// else the tag's UA-implicit display — but `<noscript>` is UA-implicit `none`
/// once `--js` ran (js.md §4: scripting hides noscript), author-overridable.
fn display(applied: &[Applied], el: &Element, js: bool) -> Display {
    match winner(applied, None, "display") {
        Some(v) => Display::parse(v),
        None if js && el.name == "noscript" => Display::None,
        None => implicit_display(&el.name),
    }
}

/// UA-implicit `display` when no rule sets it: `li` → list-item, any other
/// block-level tag ([`is_block`]) → block, everything else → inline. Never
/// yields `none`.
fn implicit_display(name: &str) -> Display {
    if name == "li" {
        Display::ListItem
    } else if is_block(name) {
        Display::Block
    } else {
        Display::Inline
    }
}

fn winner<'b>(applied: &'b [Applied], pseudo: Option<Pseudo>, name: &str) -> Option<&'b str> {
    applied
        .iter()
        .filter(|a| a.pseudo == pseudo && a.name == name)
        .max_by_key(|a| a.key())
        .map(|a| a.value.as_str())
}

fn is_none_kw(v: &str) -> bool {
    v.trim().eq_ignore_ascii_case("none")
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
    if winner(applied, Some(pseudo), "display").is_some_and(is_none_kw) {
        return None;
    }
    let raw = winner(applied, Some(pseudo), "content")?;
    let lowered = raw.trim().to_ascii_lowercase();
    if lowered == "none" || lowered == "normal" {
        return None;
    }
    Some(content_string(raw, el))
}

fn content_string(raw: &str, el: &Element) -> String {
    let mut out = String::new();
    for tok in content_tokens(raw) {
        let bytes: Vec<char> = tok.chars().collect();
        if matches!(bytes.first(), Some('"') | Some('\'')) {
            out.push_str(&unquote(&bytes));
        } else if let Some(name) = tok.strip_prefix("attr(").and_then(|t| t.strip_suffix(')')) {
            out.push_str(el.attr(name.trim()).unwrap_or(""));
        }
    }
    out
}

fn unquote(b: &[char]) -> String {
    let q = b[0];
    let mut s = String::new();
    let mut i = 1;
    while i < b.len() && b[i] != q {
        if b[i] == '\\' && i + 1 < b.len() {
            i += 1;
        }
        s.push(b[i]);
        i += 1;
    }
    s
}

/// Split a `content` value into top-level tokens (whitespace separated,
/// quotes and parens kept intact).
fn content_tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut depth = 0u32;
    let mut quote: Option<char> = None;
    let flush = |buf: &mut String, out: &mut Vec<String>| {
        if !buf.is_empty() {
            out.push(std::mem::take(buf));
        }
    };
    for ch in s.chars() {
        if let Some(qc) = quote {
            buf.push(ch);
            if ch == qc {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => {
                quote = Some(ch);
                buf.push(ch);
            }
            '(' => {
                depth += 1;
                buf.push(ch);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                buf.push(ch);
            }
            c if c.is_whitespace() && depth == 0 => flush(&mut buf, &mut out),
            _ => buf.push(ch),
        }
    }
    flush(&mut buf, &mut out);
    out
}

#[cfg(test)]
mod tests;
