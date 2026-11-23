//! Gather author CSS from the document, match it against every element where
//! it stands in the tree, and cascade it into a [`Styles`] table.
//!
//! Origin/precedence is a deliberate simplification of the real cascade,
//! sufficient for visibility and generated content: a declaration wins on
//! `(important, inline, specificity, source-order)`, compared in that order.
//! `visibility` and `text-transform` are inherited; `display` and generated
//! `content` are not. The walk also carries the line context capitalization
//! reads ([`super::transform`]): the character before each run of text.

use super::computed::{reduce, Applied};
use super::index::Index;
use super::parse::{parse_decls, Stylesheet};
use super::selector::Specificity;
use super::{ComputedStyle, Display, Styles};
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

/// Cascade `sheets` (already gathered) over every element. The single per-element `resolve` walk shared by [`compute_with`]
/// (author + external sheets) and [`compute_bare`] (empty sheets); inline
/// `style=` and UA-implicit display are applied by `resolve` regardless of the
/// sheet list.
fn cascade(doc: &Document, sheets: &[Stylesheet], js: bool) -> Styles {
    let index = Index::build(sheets);
    let mut nodes = vec![ComputedStyle::default(); doc.len()];
    let top = ComputedStyle::default();
    for &root in doc.roots() {
        walk(doc, root, &index, &top, js, &mut ' ', &mut nodes);
    }
    Styles::from_nodes(nodes)
}

/// Resolve `id` and its subtree. `prev` is the character before `id` in its
/// line: a block, an atomic inline and a replaced element each start a line
/// context of their own, and a `display:none` subtree, being in no line,
/// leaves the context as it found it.
fn walk(
    doc: &Document,
    id: NodeId,
    index: &Index,
    parent: &ComputedStyle,
    js: bool,
    prev: &mut char,
    nodes: &mut [ComputedStyle],
) {
    let entry = doc.node(id);
    // A table column paints none of its content (`bl-9dec`).
    let concealed = doc.concealed(id) || parent.display == Display::TableColumn;
    let NodeKind::Element(el) = &entry.kind else {
        let mut cs = conceal(ComputedStyle::default(), concealed);
        if let NodeKind::Text(t) = &entry.kind {
            cs.text = parent.text_transform.apply(t, prev);
        }
        nodes[id as usize] = cs;
        return;
    };
    let mut cs = conceal(resolve(doc, id, el, index, parent, js), concealed);
    let holds = doc.holds_generated(id, el);
    // A box that takes no children holds no generated ones either.
    if !holds {
        cs.before = None;
        cs.after = None;
    }
    let own_line = !holds || !matches!(cs.display, Display::Inline | Display::Contents);
    let mut inner = if own_line { ' ' } else { *prev };
    let [before, after] = cs.pseudo_transform;
    cs.before = cs.before.map(|s| before.apply(&s, &mut inner).unwrap_or(s));
    for &c in &entry.children {
        walk(doc, c, index, &cs, js, &mut inner, nodes);
    }
    cs.after = cs.after.map(|s| after.apply(&s, &mut inner).unwrap_or(s));
    if cs.display != Display::None {
        *prev = if own_line { ' ' } else { inner };
    }
    nodes[id as usize] = cs;
}

/// Fold the UA's **structural** concealment ([`Document::concealed`]) into a
/// computed style. It lands here rather than inside [`reduce`]'s UA-implicit
/// `display` step because, unlike `[hidden]` and `<noscript>`, it is not
/// author-overridable: the box the UA skips belongs to the parent, so no
/// declaration on this node can put it back. Non-element nodes get it too —
/// the raw text of a closed `<details>` is concealed exactly as its elements
/// are.
fn conceal(mut cs: ComputedStyle, concealed: bool) -> ComputedStyle {
    if concealed {
        cs.display = Display::None;
    }
    cs
}

/// Collect the declarations that reach `el` and reduce them to a
/// [`ComputedStyle`]. Only the [`Index`]'s candidate rules are tested, and they
/// arrive in flat-scan order — see `index.rs` for why that is equivalent to
/// testing every rule of every sheet.
fn resolve(
    doc: &Document,
    id: NodeId,
    el: &Element,
    index: &Index,
    parent: &ComputedStyle,
    js: bool,
) -> ComputedStyle {
    let mut applied: Vec<Applied> = Vec::new();
    for cand in index.candidates(el) {
        if !super::selector::matches(cand.selector, doc, id) {
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
    reduce(&applied, el, parent, js, doc.popover_open(id))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod details_tests;

#[cfg(test)]
mod media_tests;
