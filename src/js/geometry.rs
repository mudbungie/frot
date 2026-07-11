//! Geometry reads — the third layout trigger (js.md §8).
//!
//! Page scripts ask for boxes (`getBoundingClientRect`/`offset*`) and computed
//! style through two narrow syscalls that route into a per-generation cache: on
//! the first query for a generation this computes `Styles` + `Layout` for the
//! **current** document generation (authored CSS under `--css`, `compute_bare`
//! without it — the rule `bboxes` established, `layout.md` §3), reuses them
//! while the generation is unchanged, and recomputes on the next mutation (which
//! bumps [`Document::generation`], js.md §2). The cache lives in the [`Session`]
//! state (a shared `Geometry`), never a global.
//!
//! Both syscalls are total over any [`NodeId`]: a box-less node (a
//! `display:none`/non-rendered element, a non-element node) yields the all-zero
//! rect — spec-legal, matching a browser's `getBoundingClientRect` on a
//! `display:none` element — and an unknown computed property yields `""`.
//!
//! [`Session`]: super::Session

use std::cell::RefCell;
use std::rc::Rc;

use crate::css::{self, Display, FlexDirection, Styles, Visibility};
use crate::dom::{Document, NodeId};
use crate::layout::{self, Layout, Rect};

/// How the geometry cache computes `Styles` for a generation (js.md §8). Under
/// `--js` this is the sole styling policy the JS phase carries; the pipeline
/// picks the arm from `--css`.
pub enum StyleSource {
    /// UA-implicit display + inline `style=` only — the no-`--css` rule
    /// ([`css::compute_bare`]).
    Bare,
    /// Author cascade over the given external `<link>` stylesheet texts plus the
    /// document's own `<style>` blocks ([`css::compute_with`]) — the `--css` rule.
    Authored(Vec<String>),
}

/// Shared handle on the per-generation cache, cloned into the geometry syscall
/// closures for the JS phase's mutable window. Interior mutability, not a mirror.
pub type SharedGeometry = Rc<RefCell<Geometry>>;

/// The per-generation `Styles`+`Layout` memo (js.md §8). Holds the styling
/// policy and, once queried, the tables for one generation of the document.
pub struct Geometry {
    source: StyleSource,
    cache: Option<Cached>,
}

struct Cached {
    generation: u64,
    styles: Styles,
    layout: Layout,
}

impl Geometry {
    /// A cold cache with the given styling policy.
    pub fn new(source: StyleSource) -> Self {
        Geometry { source, cache: None }
    }

    /// The tables for `doc`'s current generation, recomputing on a miss (the
    /// first query, or the first query after a mutation bumped the generation)
    /// and reusing them while the generation is unchanged.
    fn current(&mut self, doc: &Document) -> &Cached {
        let generation = doc.generation();
        let fresh = matches!(&self.cache, Some(c) if c.generation == generation);
        if !fresh {
            // Geometry only exists inside a JS session, so `--js` is definitionally
            // active — `<noscript>` is hidden (js.md §4).
            let styles = match &self.source {
                StyleSource::Bare => css::compute_bare(doc, true),
                StyleSource::Authored(ext) => css::compute_with(doc, ext, true),
            };
            let layout = layout::compute(doc, &styles, layout::VIEWPORT_WIDTH);
            self.cache = Some(Cached { generation, styles, layout });
        }
        self.cache.as_ref().expect("cache set when stale, retained when fresh")
    }

    /// The border-box rect for `id` in viewport coordinates as `[x, y, w, h]`;
    /// the all-zero rect for any box-less node (see the module docs). The
    /// `__frot_rect` syscall shape (the prelude builds the `DOMRect`/`offset*`).
    pub(super) fn rect(&mut self, doc: &Document, id: NodeId) -> Vec<i32> {
        let r = self.current(doc).layout.rect(id).unwrap_or(Rect::ZERO);
        vec![r.x, r.y, r.w, r.h]
    }

    /// A computed value for the js.md §8 subset — `display`, `visibility`,
    /// `order`, `flex-direction`, exactly what the cascade computes — or `""`
    /// for every other property (no full computed-style set is faked).
    pub(super) fn computed(&mut self, doc: &Document, id: NodeId, prop: &str) -> String {
        let cs = self.current(doc).styles.get(id);
        match prop {
            "display" => display_keyword(cs.display).to_string(),
            "visibility" => visibility_keyword(cs.visibility).to_string(),
            "order" => cs.order.to_string(),
            "flex-direction" => flex_keyword(cs.flex_direction).to_string(),
            _ => String::new(),
        }
    }
}

fn display_keyword(d: Display) -> &'static str {
    match d {
        Display::None => "none",
        Display::Block => "block",
        Display::Inline => "inline",
        Display::InlineBlock => "inline-block",
        Display::ListItem => "list-item",
        Display::Flex => "flex",
        Display::InlineFlex => "inline-flex",
    }
}

fn visibility_keyword(v: Visibility) -> &'static str {
    match v {
        Visibility::Visible => "visible",
        Visibility::Hidden => "hidden",
    }
}

fn flex_keyword(f: FlexDirection) -> &'static str {
    match f {
        FlexDirection::Row => "row",
        FlexDirection::RowReverse => "row-reverse",
        FlexDirection::Column => "column",
        FlexDirection::ColumnReverse => "column-reverse",
    }
}

#[cfg(test)]
mod tests;
