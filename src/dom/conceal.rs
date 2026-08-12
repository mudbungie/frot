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
//!   recipe-independent subtree skips in `views::text` and `ax::tree`).
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
        self.tag_withholds(parent, keeps)
            || (self.closed_details(parent) && self.first_summary(parent) != Some(id))
    }

    /// Whether `id` is an element that withholds its children by `keeps` —
    /// asked of the *element*, not of a child, which is what the AX tree's own
    /// recipe-independent cut needs (it has no cascade to consult without
    /// `--css`). `<source>`/`<track>` are covered like any other child: they
    /// configure the box, they are never rendered beside it.
    pub fn withholds_children_from_ax(&self, id: NodeId) -> bool {
        self.tag_withholds(id, tags::exposes_children)
    }

    fn tag_withholds(&self, id: NodeId, keeps: fn(&str) -> bool) -> bool {
        matches!(&self.node(id).kind, NodeKind::Element(el) if !keeps(&el.name))
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
