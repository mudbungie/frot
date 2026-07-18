//! CSS application (`--css`): a small, dependency-free engine that parses
//! author CSS, matches a selector subset, cascades it, and exposes a
//! per-node [`Styles`] side table.
//!
//! Scope is deliberately narrow — only what visibility and generated content
//! need: `display:none`, `visibility:hidden|collapse` (inherited), and
//! `::before`/`::after` `content`. `@media` preludes are evaluated against
//! the fixed 1280×720 viewport ([`media`] — the same evaluator JS
//! `matchMedia` delegates to). There is no layout and no
//! `cssparser`/`selectors` dependency; see `VISION.md`.

mod cascade;
pub mod media;
mod parse;
mod query;
mod selector;
mod selparse;

use crate::dom::{Document, NodeId, NodeKind};

pub use cascade::{compute, compute_bare, compute_with};
pub use query::{query_all, UnsupportedSelector};

/// Concatenated text of `id`'s subtree as the accessible-name algorithm sees
/// it under `--css`: `display:none` subtrees are dropped and `::before` /
/// `::after` generated content is included. With no rule affecting the
/// subtree this is exactly [`Document::text_content`].
pub fn rendered_subtree_text(doc: &Document, id: NodeId, styles: &Styles) -> String {
    let mut out = String::new();
    collect_text(doc, id, styles, &mut out);
    out
}

fn collect_text(doc: &Document, id: NodeId, styles: &Styles, out: &mut String) {
    let entry = doc.node(id);
    match &entry.kind {
        NodeKind::Element(_) => {
            if styles.display_none(id) {
                return;
            }
            if let Some(b) = styles.before(id) {
                out.push_str(b);
            }
            for &c in &entry.children {
                collect_text(doc, c, styles, out);
            }
            if let Some(a) = styles.after(id) {
                out.push_str(a);
            }
        }
        NodeKind::Text(t) => out.push_str(t),
        NodeKind::Comment(_) | NodeKind::Doctype => {}
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Visible,
    Hidden,
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

/// Computed `display` keyword: one of the seven values Phase 3 layout branches
/// on. The cascade coerces every other CSS `display` value (`grid`, `table*`,
/// `contents`, unknown) to [`Display::Block`] once, at compute time, so this
/// enum only ever holds one of these variants. The default is [`Display::Inline`]
/// (never `None`): non-element and untouched nodes must report as rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Display {
    None,
    Block,
    #[default]
    Inline,
    InlineBlock,
    ListItem,
    Flex,
    InlineFlex,
}

impl Display {
    /// Parse an author/inline `display` value into one of the seven layout
    /// keywords. Case-insensitive and trimmed. Every other value — `grid`,
    /// `inline-grid`, `table`, `table-*`, `contents`, unknown — coerces to
    /// [`Display::Block`] (the layout §2 coercion, applied once here).
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "none" => Display::None,
            "block" => Display::Block,
            "inline" => Display::Inline,
            "inline-block" => Display::InlineBlock,
            "list-item" => Display::ListItem,
            "flex" => Display::Flex,
            "inline-flex" => Display::InlineFlex,
            _ => Display::Block,
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
}

#[cfg(test)]
mod tests;
