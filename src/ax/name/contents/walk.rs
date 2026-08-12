//! Where a traversal *is* — the one axis every positional accname rule reads.
//!
//! Three of the rules [`super`] applies hold at some nodes of a name
//! computation and not at others, and no two of them cut the traversal in the
//! same place. [`Via`] records how the current node was reached; each rule is
//! one predicate on it, so the recursion never asks where it is, only what it
//! may do.
//!
//! | reached | reads `title` §2I | reads a hidden node §2A | follows `aria-labelledby` §2B |
//! |---|---|---|---|
//! | `Subject` — the node being named: a `<label>`, a `<legend>` | yes | no | yes |
//! | `Contents` — a descendant, in the §2F recursion | **no** | no | yes |
//! | `Target` — the node a reference names directly | yes | **yes** | **no** |
//! | `UnderTarget` — a descendant of such a node | yes | no | **no** |
//!
//! Every cell is measured against Chrome 139 (`Accessibility.getFullAXTree`,
//! `--headless=new`); the `title` column is `bl-d8ff`, the other two `bl-0482`.
//! The `title` column is why `Subject` exists: a `<label title=TL></label>`
//! names its control "TL" while `<div role=button>A<span title=T></span>B</div>`
//! is "AB", so `title` is live at the node a computation is *about* and dead at
//! the descendants it merely reads — the same standing the `title` step of
//! [`super::super::accessible_name`] has at its own entry node. Inside a
//! reference it is live all the way down (`<span id=t><span title=T></span>
//! </span>` names its referrer "T"), which is the whole of the difference.

use crate::css::Styles;
use crate::dom::Document;

/// One traversal's fixed context: the tree, the cascade, and how the traversal
/// reached the node it is reading.
pub struct Walk<'a> {
    pub doc: &'a Document,
    pub styles: Option<&'a Styles>,
    via: Via,
}

/// How a node was reached. Four states because the three rules above need
/// four: no pair of them agrees on every row.
enum Via {
    /// The node whose name is being computed — the door `<label>` and
    /// `<legend>` enter by.
    Subject,
    /// A descendant, contributing its text alternative to someone else's name.
    Contents,
    /// The node a reference names *directly*.
    Target,
    /// Anything below such a node: still inside the reference, no longer the
    /// node it named.
    UnderTarget,
}

impl<'a> Walk<'a> {
    /// The descendants of a role that names from its contents.
    pub fn contents(doc: &'a Document, styles: Option<&'a Styles>) -> Self {
        Self {
            doc,
            styles,
            via: Via::Contents,
        }
    }

    /// The element being named itself.
    pub fn subject(doc: &'a Document, styles: Option<&'a Styles>) -> Self {
        Self {
            doc,
            styles,
            via: Via::Subject,
        }
    }

    /// The element an `aria-labelledby` points at.
    pub fn referenced(doc: &'a Document, styles: Option<&'a Styles>) -> Self {
        Self {
            doc,
            styles,
            via: Via::Target,
        }
    }

    /// The same traversal one step further in. Both things that single out a
    /// node — being the subject, being the target — are spent on that node;
    /// being *inside* a reference is not.
    pub fn below(&self) -> Self {
        let via = match self.via {
            Via::Subject | Via::Contents => Via::Contents,
            Via::Target | Via::UnderTarget => Via::UnderTarget,
        };
        Self {
            doc: self.doc,
            styles: self.styles,
            via,
        }
    }

    /// §2I: `title` speaks for a node that said nothing — but only where the
    /// node's own name is what is being computed, or where the whole traversal
    /// is a reference. A descendant read for its contribution is silent.
    pub fn reads_title(&self) -> bool {
        !matches!(self.via, Via::Contents)
    }

    /// §2A excludes a hidden node "unless directly referenced" — and only the
    /// referenced node itself is exempt; a hidden child of it is still dropped.
    pub fn reads_hidden(&self) -> bool {
        matches!(self.via, Via::Target)
    }

    /// §2B does not recurse: one reference hop is all there is, so neither a
    /// target nor anything under it follows a further `aria-labelledby`. A
    /// `<legend>`'s own reference *is* followed — a legend is not inside one.
    pub fn follows_references(&self) -> bool {
        matches!(self.via, Via::Subject | Via::Contents)
    }
}
