//! Gather author CSS from the document, match it against every element where
//! it stands in the tree, and cascade it into a [`Styles`] table.
//!
//! Origin/precedence is a deliberate simplification of the real cascade,
//! sufficient for visibility and generated content: a declaration wins on
//! `(important, context, inline, specificity, source-order)`, compared in that
//! order — the context step is the shadow boundary's ([`super::trees`]).
//! The walk follows the flat tree (js.md §15), so a slotted element inherits
//! from its slot and a shadow tree from its host.
//! `visibility` and `text-transform` are inherited; `display` and generated
//! `content` are not. The walk also carries the line context capitalization
//! reads ([`super::transform`]): the character before each run of text.

use super::computed::{reduce, Applied};
use super::index::Index;
use super::parse::{parse_decls, Stylesheet};
use super::selector::{matches_in, Context, Specificity};
use super::trees::{self, Trees};
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
    let adopted = doc.adopted(Document::ROOT).iter();
    sheets.extend(adopted.map(|t| Stylesheet::parse(t)));
    let shadow = trees::gather(doc);
    cascade(doc, &Trees::build(&sheets, &shadow), js)
}

/// UA-implicit `display` plus inline `style=` only — **no** author `<style>`
/// blocks and **no** external sheets. This is the styles source layout uses
/// **without** `--css` (`layout.md` §3): `--css` everywhere means "apply author
/// CSS", so bare layout must ignore `<style>` exactly as it ignores `<link>`
/// sheets. Shares the [`cascade`] walk with [`compute_with`]; the only
/// difference is the empty sheet list.
pub fn compute_bare(doc: &Document, js: bool) -> Styles {
    cascade(doc, &Trees::build(&[], &[]), js)
}

/// Cascade `sheets` (already gathered) over every element. The single per-element `resolve` walk shared by [`compute_with`]
/// (author + external sheets) and [`compute_bare`] (empty sheets); inline
/// `style=` and UA-implicit display are applied by `resolve` regardless of the
/// sheet list.
fn cascade(doc: &Document, trees: &Trees, js: bool) -> Styles {
    let mut nodes = vec![ComputedStyle::default(); doc.len()];
    let top = ComputedStyle::default();
    let cx = Walk { doc, trees, js };
    for &root in doc.roots() {
        walk(cx, root, Document::ROOT, &top, &mut ' ', &mut nodes);
    }
    Styles::from_nodes(nodes)
}

/// What every step of the walk shares.
#[derive(Clone, Copy)]
struct Walk<'a> {
    doc: &'a Document,
    trees: &'a Trees<'a>,
    js: bool,
}

/// Resolve `id` and its subtree. `tree` is the root of `id`'s node tree (the
/// document, or a shadow root), whose rules it is matched against. `prev` is
/// the character before `id` in its line: a block, an atomic inline and a
/// replaced element each start a line context of their own, and a
/// `display:none` subtree, being in no line, leaves the context as it found it.
fn walk(
    cx: Walk,
    id: NodeId,
    tree: NodeId,
    parent: &ComputedStyle,
    prev: &mut char,
    nodes: &mut [ComputedStyle],
) {
    let doc = cx.doc;
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
    let mut cs = conceal(resolve(cx, id, el, tree, parent), concealed);
    let holds = doc.holds_generated(id, el);
    // A box that takes no children holds no generated ones either.
    if !holds {
        cs.before = None;
        cs.after = None;
    }
    let own_line = !holds || !(cs.display.inline() || cs.display == Display::Contents);
    let mut inner = if own_line { ' ' } else { *prev };
    let [before, after] = cs.pseudo_transform;
    cs.before = cs.before.map(|s| before.apply(&s, &mut inner).unwrap_or(s));
    for &c in doc.flat_children(id).iter() {
        // A flat child from another tree: the host's shadow root's, or a
        // slotted one back in the host's tree.
        let tree = if doc.node(c).parent == Some(id) {
            tree
        } else {
            doc.tree_root(c)
        };
        walk(cx, c, tree, &cs, &mut inner, nodes);
    }
    cs.after = cs.after.map(|s| after.apply(&s, &mut inner).unwrap_or(s));
    // After a block the next line starts fresh; after an atomic inline it
    // goes on from the inline's last character (`bl-f058`).
    if cs.display != Display::None {
        *prev = if holds && !cs.display.block_level() {
            inner
        } else {
            ' '
        };
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
/// [`ComputedStyle`]: its own tree's rules; as a host, its shadow tree's
/// `:host` rules; as a slotted element, its slot's tree's `::slotted()` ones.
/// Only an [`Index`]'s candidate rules are tested, and they arrive in
/// flat-scan order — see `index.rs` for why that is equivalent to testing
/// every rule of every sheet.
fn resolve(
    cx: Walk,
    id: NodeId,
    el: &Element,
    tree: NodeId,
    parent: &ComputedStyle,
) -> ComputedStyle {
    let doc = cx.doc;
    let mut applied: Vec<Applied> = Vec::new();
    let host = doc.shadow(tree).map(|s| s.host);
    let mine = Context {
        host,
        ..Context::of(doc)
    };
    rules(&mut applied, cx.trees.of(doc, tree), mine, id, el, false);
    if let Some(root) = doc.shadow_root(id) {
        let me = Context {
            host: Some(id),
            ..Context::of(doc)
        };
        rules(&mut applied, cx.trees.of(doc, root), me, id, el, true);
    }
    if let Some(slot) = doc.assigned_slot(id) {
        let root = doc.tree_root(slot);
        let host = doc.shadow(root).map(|s| s.host);
        let into = Context {
            host,
            slotted: Some(id),
            ..Context::of(doc)
        };
        let slot_el = doc.element(slot).unwrap_or(el);
        rules(
            &mut applied,
            cx.trees.of(doc, root),
            into,
            slot,
            slot_el,
            true,
        );
    }
    // Inline `style=` sits one slot past every author rule in source order.
    let order = cx.trees.rule_count();
    if let Some(style) = el.attr("style") {
        for d in parse_decls(style) {
            applied.push(Applied {
                pseudo: None,
                important: d.important,
                inline: true,
                inner: false,
                spec: Specificity::default(),
                order,
                name: d.name,
                value: d.value,
            });
        }
    }
    let hidden = el.hidden() && !super::lang::foreign(doc, id);
    reduce(&applied, el, parent, cx.js, doc.popover_open(id), hidden)
}

/// Push the declarations of `index`'s rules that match `at` (element `el`) in
/// `cx`; `inner` marks rules reaching out of a shadow tree.
fn rules(
    applied: &mut Vec<Applied>,
    index: Option<&Index>,
    cx: Context,
    at: NodeId,
    el: &Element,
    inner: bool,
) {
    for cand in index.map(|i| i.candidates(el)).unwrap_or_default() {
        if !matches_in(cand.selector, cx, at) {
            continue;
        }
        for d in cand.decls {
            applied.push(Applied {
                pseudo: cand.pseudo,
                important: d.important,
                inline: false,
                inner,
                spec: cand.spec,
                order: cand.order,
                name: d.name.clone(),
                value: d.value.clone(),
            });
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod details_tests;

#[cfg(test)]
mod ua_tests;

#[cfg(test)]
mod media_tests;

#[cfg(test)]
mod shadow_tests;
