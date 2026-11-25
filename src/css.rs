//! CSS application (`--css`): a small, dependency-free engine that parses
//! author CSS, matches a selector subset, cascades it, and exposes a
//! per-node [`Styles`] side table.
//!
//! Scope is deliberately narrow — only what visibility and generated content
//! need: `display:none`, `visibility:hidden|collapse` (inherited),
//! `::before`/`::after` `content`, and `text-transform` (inherited; the case
//! text is painted in, [`transform`]). `@media` preludes are evaluated against
//! the fixed 1280×720 viewport ([`media`] — the same evaluator JS
//! `matchMedia` delegates to). There is no layout and no
//! `cssparser`/`selectors` dependency; see `VISION.md`.

mod attrsel;
mod cascade;
mod checked;
mod computed;
mod content;
mod control;
mod display;
mod escape;
mod focus;
mod has;
mod import;
mod index;
mod lang;
mod list;
pub mod media;
mod nth;
mod paint;
mod parse;
mod pseudo;
mod query;
mod selector;
mod selparse;
mod shadow;
mod state;
mod transform;
mod trees;
mod validity;

use crate::dom::NodeId;

pub use cascade::{compute, compute_bare, compute_with};
pub use checked::{pick, picked};
pub use control::{custom as custom_name, takes_placeholder};
pub use display::Display;
pub use focus::{focusable, focused_element};
pub use import::imports;
pub use paint::Paint;
pub use query::{matches_any, query_all, UnsupportedSelector};
pub use state::editable;
pub use transform::{Case, TextTransform};

// This module publishes a table, not a traversal. Every consumer that needs
// "the text of this subtree as rendered" walks the arena itself and asks
// `Styles` per node — `views::text` for `--out text`, `ax::name::contents` for
// the accname §2F recursion. There is deliberately no subtree-text helper
// here: two of them drifted once already (`bl-3d2e`).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Visible,
    Hidden,
}

/// How a box stands in its parent's flow. Absolutely positioned (`position:
/// absolute | fixed`) wins over a float; `relative` and `sticky` stay in
/// flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Flow {
    #[default]
    In,
    Float,
    Positioned,
}

/// Computed `flex-direction` — the flex container's main axis. Only consumed by
/// flex layout (reading order and crude main-axis placement, `layout.md` §5/§6).
/// The default is [`FlexDirection::Row`]; every unrecognised value coerces to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

impl FlexDirection {
    /// Parse an author/inline `flex-direction` value. Case-insensitive and
    /// trimmed; any unknown/absent value coerces to [`FlexDirection::Row`].
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "row" => FlexDirection::Row,
            "row-reverse" => FlexDirection::RowReverse,
            "column" => FlexDirection::Column,
            "column-reverse" => FlexDirection::ColumnReverse,
            _ => FlexDirection::Row,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComputedStyle {
    /// Computed `display` keyword — the winning author/inline rule, else the
    /// element's UA-implicit display by tag. `display:none` is the
    /// [`Display::None`] variant (queried via [`Styles::display_none`]).
    pub display: Display,
    /// Computed `order` (flex reorder key); default `0`. Only flex layout reads
    /// it (`layout.md` §5: sort in-flow children by `(order, source index)`).
    pub order: i32,
    /// Computed `flex-direction`; default [`FlexDirection::Row`]. Only flex
    /// layout reads it (main axis + `*-reverse` reading-order flip).
    pub flex_direction: FlexDirection,
    /// Computed (inherited) visibility.
    pub visibility: Visibility,
    /// `::before` generated content, if a generated box is produced.
    pub before: Option<String>,
    /// `::after` generated content, if a generated box is produced.
    pub after: Option<String>,
    /// Computed `display` of the `::before` box (`inline` unless a rule on
    /// the pseudo-element says otherwise or takes it out of flow).
    pub before_display: Display,
    /// Computed `display` of the `::after` box.
    pub after_display: Display,
    /// Whether the box is in flow, floated or absolutely positioned. A box
    /// out of flow is blockified ([`Display::blockified`]) and yet does not
    /// make its parent's other children block-level.
    pub flow: Flow,
    /// Computed (inherited) `list-style-type` is `none`: a list-item box
    /// with no marker (`list.rs`).
    pub unmarked: bool,
    /// The same fact for the `::before` and the `::after` box, in that order.
    pub pseudo_out_of_flow: [bool; 2],
    /// Computed (inherited) `text-transform` ([`transform`]). `before` and
    /// `after` are already transformed by theirs, `pseudo_transform`.
    pub text_transform: TextTransform,
    pub pseudo_transform: [TextTransform; 2],
    /// What the data-table test reads of a cell or row ([`Paint`]).
    pub paint: Paint,
    /// A text node's characters as painted, when a transform changed them.
    pub text: Option<String>,
}

/// Per-node computed styles, indexed by [`NodeId`]. Non-element nodes and
/// elements no rule touched carry [`ComputedStyle::default`] (rendered,
/// visible, no generated content).
#[derive(Debug, Clone)]
pub struct Styles {
    nodes: Vec<ComputedStyle>,
}

impl Styles {
    fn from_nodes(nodes: Vec<ComputedStyle>) -> Self {
        Self { nodes }
    }

    pub fn get(&self, id: NodeId) -> &ComputedStyle {
        &self.nodes[id as usize]
    }

    /// Whether `id`'s subtree is removed from rendered output — the derived
    /// query `display == Display::None` (true iff author `display:none`).
    pub fn display_none(&self, id: NodeId) -> bool {
        self.get(id).display == Display::None
    }

    /// Computed `display` keyword for `id` (see [`ComputedStyle::display`]).
    pub fn display(&self, id: NodeId) -> Display {
        self.get(id).display
    }

    /// Computed `order` for `id` (flex reorder key; see
    /// [`ComputedStyle::order`]).
    pub fn order(&self, id: NodeId) -> i32 {
        self.get(id).order
    }

    /// Computed `flex-direction` for `id` (see
    /// [`ComputedStyle::flex_direction`]).
    pub fn flex_direction(&self, id: NodeId) -> FlexDirection {
        self.get(id).flex_direction
    }

    pub fn visibility(&self, id: NodeId) -> Visibility {
        self.get(id).visibility
    }

    pub fn before(&self, id: NodeId) -> Option<&str> {
        self.get(id).before.as_deref()
    }

    pub fn after(&self, id: NodeId) -> Option<&str> {
        self.get(id).after.as_deref()
    }

    /// Computed `display` of `id`'s `::before` box.
    pub fn before_display(&self, id: NodeId) -> Display {
        self.get(id).before_display
    }

    /// Computed `display` of `id`'s `::after` box.
    pub fn after_display(&self, id: NodeId) -> Display {
        self.get(id).after_display
    }

    /// The boxes `id` generates, `::before` then `::after`, each as its
    /// computed `display` and whether it is in flow.
    pub fn generated_boxes(&self, id: NodeId) -> impl Iterator<Item = (Display, bool)> {
        let cs = self.get(id);
        let [before, after] = cs.pseudo_out_of_flow;
        let before = cs.before.as_ref().map(|_| (cs.before_display, !before));
        let after = cs.after.as_ref().map(|_| (cs.after_display, !after));
        before.into_iter().chain(after)
    }

    /// Text node `id`'s characters as painted: `raw` unless a
    /// `text-transform` changed them.
    pub fn text<'a>(&'a self, id: NodeId, raw: &'a str) -> &'a str {
        self.get(id).text.as_deref().unwrap_or(raw)
    }

    /// Border sides, background and `empty-cells` of `id` ([`Paint`]).
    pub fn paint(&self, id: NodeId) -> &Paint {
        &self.get(id).paint
    }

    /// Whether `id`'s box is out of flow ([`ComputedStyle::flow`]).
    pub fn out_of_flow(&self, id: NodeId) -> bool {
        self.get(id).flow != Flow::In
    }

    /// Whether `id`'s box is absolutely positioned ([`Flow::Positioned`]).
    pub fn positioned(&self, id: NodeId) -> bool {
        self.get(id).flow == Flow::Positioned
    }

    /// Whether `id`'s box, as a list item, has a marker
    /// ([`ComputedStyle::unmarked`]).
    pub fn marked(&self, id: NodeId) -> bool {
        !self.get(id).unmarked
    }
}

#[cfg(test)]
mod tests;
