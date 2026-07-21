//! Gather author CSS from the document, match it against every element with
//! full ancestor context, and cascade it into a [`Styles`] table.
//!
//! Origin/precedence is a deliberate simplification of the real cascade,
//! sufficient for visibility and generated content: a declaration wins on
//! `(important, inline, specificity, source-order)`, compared in that order.
//! `visibility` is inherited; `display` and generated `content` are not.

use super::computed::{reduce, Applied};
use super::index::Index;
use super::parse::{parse_decls, Stylesheet};
use super::selector::Specificity;
use super::{ComputedStyle, Styles, Visibility};
use crate::dom::{Document, Element, NodeId, NodeKind};

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
    let index = Index::build(sheets);
    let mut nodes = vec![ComputedStyle::default(); doc.len()];
    let mut ancestors: Vec<&Element> = Vec::new();
    for &root in doc.roots() {
        walk(
            doc,
            root,
            &mut ancestors,
            &index,
            Visibility::Visible,
            js,
            &mut nodes,
        );
    }
    Styles::from_nodes(nodes)
}

fn walk<'a>(
    doc: &'a Document,
    id: NodeId,
    ancestors: &mut Vec<&'a Element>,
    index: &Index,
    parent_vis: Visibility,
    js: bool,
    nodes: &mut [ComputedStyle],
) {
    let entry = doc.node(id);
    let NodeKind::Element(el) = &entry.kind else {
        return;
    };
    let cs = resolve(el, ancestors, index, parent_vis, js);
    let vis = cs.visibility;
    nodes[id as usize] = cs;
    ancestors.push(el);
    for &c in &entry.children {
        walk(doc, c, ancestors, index, vis, js, nodes);
    }
    ancestors.pop();
}

/// Collect the declarations that reach `el` and reduce them to a
/// [`ComputedStyle`]. Only the [`Index`]'s candidate rules are tested, and they
/// arrive in flat-scan order — see `index.rs` for why that is equivalent to
/// testing every rule of every sheet.
fn resolve(
    el: &Element,
    ancestors: &[&Element],
    index: &Index,
    parent_vis: Visibility,
    js: bool,
) -> ComputedStyle {
    let near_first: Vec<&Element> = ancestors.iter().rev().copied().collect();
    let mut applied: Vec<Applied> = Vec::new();
    for cand in index.candidates(el) {
        if !super::selector::matches(cand.selector, el, &near_first) {
            continue;
        }
        for d in cand.decls {
            applied.push(Applied {
                pseudo: cand.pseudo,
                important: d.important,
                inline: false,
                spec: cand.spec,
                order: cand.order,
                name: d.name.clone(),
                value: d.value.clone(),
            });
        }
    }
    // Inline `style=` sits one slot past every author rule in source order.
    let order = index.rule_count;
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
    reduce(&applied, el, parent_vis, js)
}

#[cfg(test)]
mod tests;
