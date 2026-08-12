//! Structural concealment: the UA rules that withhold a node for what its
//! **parent** is, rather than for anything the node itself carries
//! ([`Element::hidden`] is the attribute-local half of the same question).
//! Answered for text nodes as much as elements — the box the UA skips is the
//! parent's, so everything inside it goes, markup or not.
//!
//! Two rules, both of them "the parent's box swallows the child":
//!
//! - A **`<details>` without `open`** renders only its first `<summary>`
//!   element child — the disclosure control. Every other child is disclosure
//!   content, which HTML Rendering ("The `details` and `summary` elements")
//!   puts in a `::details-content` box that is `content-visibility: hidden`
//!   while closed. A closed `<details>` with no `<summary>` conceals every
//!   child — the UA supplies its own disclosure control, which is not in the
//!   document (`bl-74a6`).
//! - A **fallback-content element** paints *none* of its children
//!   ([`crate::tags`], which owns that fact and is also read by the
//!   recipe-independent subtree skip in `views::text`).
//!
//! Neither is author-overridable: the skipped box is not the child's, and frot
//! has no anonymous boxes to give it one.
//!
//! ## Why two queries and not one (`bl-e79a`)
//!
//! "Not painted" and "not in the accessibility tree" looked like one fact until
//! `<canvas>` was measured. HTML makes canvas fallback content the element's
//! accessible sub-tree, and Chrome 139 agrees: `<canvas>x</canvas>` gives `x`
//! zero client rects and no `innerText`, yet exposes a live `StaticText "x"`.
//! So the two questions have different answers for the same node, and each gets
//! its own name here:
//!
//! - [`Document::unpainted`] — generates no box. Read by `layout` (and so by
//!   `bboxes`) and by the needs walk.
//! - [`Document::concealed`] — the strict subset that is *also* absent from the
//!   accessibility tree. Read by the cascade, which folds it into
//!   `display: none`; that is why canvas fallback must not be in it, since
//!   `ax::tree` prunes at `display:none` and would lose the sub-tree HTML says
//!   a screen-reader user gets.
//!
//! The `<canvas>` exception is confined to the tag table: everything else about
//! canvas fallback is ordinary. Authored `display:none` inside it still hides
//! it from AX, in Chrome and here alike, because that arrives through the
//! cascade rather than through these rules.
//!
//! ## Accessors, not remembered checks (`bl-0aaf`, `bl-d470`)
//!
//! The same defect surfaced three times in one day, once per traversal that
//! walked `node.children` directly and re-derived its own idea of what was
//! excluded: the AX tree walk asked `concealed` and the accname §2F recursion
//! did not (`bl-0aaf`), and `views::text` asked only the *tag* half and not the
//! closed-`<details>` half (`bl-d470`). Every one of them was invisible under
//! `--css`, because the cascade spelled the same fact a second time as
//! `display: none` and the walks did check that.
//!
//! So the predicates above are no longer what a traversal asks. Each has an
//! **accessor** built from the same `keeps`, and a walk descends through it:
//!
//! | traversal | accessor |
//! |---|---|
//! | `views::text` | [`Document::painted_children`] |
//! | `ax::tree`, `ax::name::contents` | [`Document::ax_children`] |
//!
//! The agreement is then structural rather than remembered: a node the page
//! does not paint is unreachable by the text walk, and a node absent from the
//! AX tree is unreachable by the name walk, *because there is no other way in*
//! — in every recipe, with or without a cascade. The recipe decides how the
//! fact was computed, never which content it covers.
//!
//! The predicates stay for the callers that are **not** child traversals but
//! compound gates gluing this fact to a cascade fact: `layout::renders`
//! (`unpainted` + `display:none`), `needs::not_rendered` (`unpainted` +
//! `display:none` or the `hidden` attribute), and `css::cascade::conceal`
//! (`concealed`, which is how the fact reaches `--css` consumers at all).

use super::{Document, NodeId, NodeKind};
use crate::tags;

impl Document {
    /// Whether `id` generates no box because of what its parent is — the wider
    /// of the two structural queries (module docs). A `<video>`'s prose, a
    /// `<canvas>`'s fallback and a closed `<details>`'s disclosure body are all
    /// unpainted; only the canvas one is still exposed to assistive technology.
    pub fn unpainted(&self, id: NodeId) -> bool {
        self.swallowed(id, tags::renders_children)
    }

    /// Whether `id` is unpainted **and** absent from the accessibility tree —
    /// the narrower query, and the one the cascade folds into `display: none`.
    pub fn concealed(&self, id: NodeId) -> bool {
        self.swallowed(id, tags::exposes_children)
    }

    /// Whether `id`'s parent's box swallows it, with `keeps` deciding the
    /// fallback-content half: [`tags::renders_children`] asks about paint,
    /// [`tags::exposes_children`] about the accessibility tree. The closed
    /// `<details>` half is document *state* rather than a tag fact, so it holds
    /// for both.
    fn swallowed(&self, id: NodeId, keeps: fn(&str) -> bool) -> bool {
        let Some(parent) = self.node(id).parent else {
            return false;
        };
        matches!(&self.node(parent).kind, NodeKind::Element(el) if !keeps(&el.name))
            || (self.closed_details(parent) && self.first_summary(parent) != Some(id))
    }

    /// `id`'s children as the page paints them — the accessor form of
    /// [`Document::unpainted`], and the only way into a node's children from
    /// `views::text` (module docs, "Accessors, not remembered checks").
    pub fn painted_children(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.kept_children(id, tags::renders_children)
    }

    /// `id`'s children as the accessibility tree sees them — the accessor form
    /// of [`Document::concealed`], and the only way into a node's children from
    /// `ax::tree` and `ax::name::contents`. `<source>`/`<track>` need no
    /// mention: they are children of a fallback-content element like any other,
    /// and go with the rest.
    pub fn ax_children(&self, id: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.kept_children(id, tags::exposes_children)
    }

    /// The children `keeps` does not swallow — one shape for both accessors,
    /// taking the same `keeps` that separates [`Document::unpainted`] from
    /// [`Document::concealed`], so the pair can never answer a *third* question.
    fn kept_children(
        &self,
        id: NodeId,
        keeps: fn(&str) -> bool,
    ) -> impl Iterator<Item = NodeId> + '_ {
        self.node(id)
            .children
            .iter()
            .copied()
            .filter(move |&c| !self.swallowed(c, keeps))
    }

    /// Whether `id` is a `<details>` element carrying no `open` attribute.
    /// `open` is a boolean attribute, so presence is the fact.
    fn closed_details(&self, id: NodeId) -> bool {
        matches!(&self.node(id).kind,
            NodeKind::Element(el) if el.name == "details" && el.attr("open").is_none())
    }

    /// The first `<summary>` element child of `id`, if any — the one child a
    /// closed `<details>` renders (`details > summary:first-of-type`); a later
    /// `<summary>` is disclosure content like any other child.
    fn first_summary(&self, id: NodeId) -> Option<NodeId> {
        self.node(id)
            .children
            .iter()
            .copied()
            .find(|&c| matches!(&self.node(c).kind, NodeKind::Element(el) if el.name == "summary"))
    }
}
