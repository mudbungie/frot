//! The computed `display` keyword (`layout.md` §2): what the cascade keeps of
//! a `display` value, and the questions the views ask of it.

use super::Styles;
use crate::dom::NodeId;

/// Computed `display`: the boxes frot tells apart. The cascade maps every
/// value Chrome accepts onto one of these ([`Display::parse`]), so the enum
/// only ever holds one of these variants. The default is [`Display::Inline`]
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
    /// A table box (`table`, `block table`): laid out as a block, and no
    /// block container — it holds no line of its own (`bl-08c6`).
    Table,
    /// `inline-table`: an inline-block to every reader but `ax`, where it is
    /// a table as [`Display::Table`] is.
    InlineTable,
    /// A grid container: laid out as a block, but its children are grid
    /// items, which CSS blockifies.
    Grid,
    InlineGrid,
    /// `table-column` / `table-column-group`: a box that starts and ends a
    /// line and paints none of its content (Chrome 158, `bl-9dec`).
    TableColumn,
    /// No box of its own: the children stand in the parent's flow.
    Contents,
    /// A table cell: laid out as a block, and in `text` set off only from
    /// the cell before it.
    TableCell,
    /// A table row or row group: the same, from the row before it.
    TableRow,
}

impl Display {
    /// Parse an author/inline `display` value, case-insensitive and trimmed.
    /// A value Chrome does not accept is `None`: the declaration is invalid
    /// and the cascade passes over it, as if it had not been written. A
    /// value frot has no algorithm for takes the nearest box it has — a grid
    /// or a table is a block, their inline twins an inline-block, a ruby an
    /// inline (the layout §2 coercion, applied once here). Measured in
    /// Chrome 158 (`bl-e3a0`). The keywords of a multi-keyword value come in
    /// any order (`flex inline`, `list-item block`; `bl-9dec`).
    pub fn parse(value: &str) -> Option<Self> {
        Some(match canonical(value)?.as_str() {
            "none" => Display::None,
            "block" | "flow" | "flow-root" | "table-caption" | "-webkit-box" | "block flow"
            | "block flow-root" | "block ruby" | "block math" => Display::Block,
            "table" | "block table" => Display::Table,
            "grid" | "block grid" => Display::Grid,
            "inline-grid" | "inline grid" => Display::InlineGrid,
            "table-column" | "table-column-group" => Display::TableColumn,
            "inline"
            | "ruby"
            | "ruby-text"
            | "math"
            | "inline flow"
            | "inline ruby"
            | "inline math"
            | "inline list-item"
            | "inline flow-root list-item"
            | "initial"
            | "unset" => Display::Inline,
            "inline-block" | "-webkit-inline-box" | "inline flow-root" => Display::InlineBlock,
            "inline-table" | "inline table" => Display::InlineTable,
            "list-item" | "block list-item" | "block flow-root list-item" => Display::ListItem,
            "flex" | "-webkit-flex" | "block flex" => Display::Flex,
            "inline-flex" | "-webkit-inline-flex" | "inline flex" => Display::InlineFlex,
            "contents" => Display::Contents,
            "table-cell" => Display::TableCell,
            "table-row" | "table-row-group" | "table-header-group" | "table-footer-group" => {
                Display::TableRow
            }
            _ => return None,
        })
    }

    /// Whether `value` is `revert`: a valid declaration that asks for the
    /// user agent's own `display`, which is what losing every author
    /// declaration leaves.
    pub fn reverts(value: &str) -> bool {
        value.trim().eq_ignore_ascii_case("revert")
    }

    /// Whether the box is block-level: it starts and ends a line. The
    /// inline-level boxes are `inline`, `inline-block` and `inline-flex`; a
    /// table part is neither, and `contents` is no box.
    pub fn block_level(self) -> bool {
        matches!(
            self,
            Display::Block
                | Display::Table
                | Display::ListItem
                | Display::Flex
                | Display::Grid
                | Display::TableColumn
        )
    }

    /// Whether the box's children are flex or grid items, which CSS
    /// blockifies as it does a box taken out of flow (Chrome 158: a span in a
    /// flex or a grid computes `block`, `bl-9dec`).
    pub fn has_items(self) -> bool {
        matches!(
            self,
            Display::Flex | Display::InlineFlex | Display::Grid | Display::InlineGrid
        )
    }

    /// Whether layout stacks the box as a block: the block-level boxes, and
    /// the table parts, which have no algorithm of their own (§2).
    pub fn stacks(self) -> bool {
        self.block_level() || self.table_part()
    }

    /// Whether the box is a table cell or row.
    pub fn table_part(self) -> bool {
        matches!(self, Display::TableCell | Display::TableRow)
    }

    /// The computed `display` of a box taken out of flow — absolutely
    /// positioned or floated — which CSS blockifies (CSS Display §2.7):
    /// `inline`, `inline-block` and the table parts compute to `block`,
    /// `inline-flex` to `flex`. In flow, and for every other value —
    /// `contents` has no box to take out of flow — it is `self`.
    pub fn blockified(self, out_of_flow: bool) -> Self {
        match self {
            Display::Inline | Display::InlineBlock | Display::TableCell | Display::TableRow
                if out_of_flow =>
            {
                Display::Block
            }
            Display::InlineTable if out_of_flow => Display::Table,
            Display::InlineFlex if out_of_flow => Display::Flex,
            Display::InlineGrid if out_of_flow => Display::Grid,
            other => other,
        }
    }

    /// The box an element generates, for a reader that may have no cascade:
    /// the computed `display` where there is one, and where there is not the
    /// tag's own — block for a [`crate::tags::is_block`] tag, inline for any
    /// other.
    pub fn of(styles: Option<&Styles>, id: NodeId, tag: &str) -> Self {
        match styles {
            Some(s) => s.display(id),
            None if crate::tags::is_block(tag) => Display::Block,
            None => Display::Inline,
        }
    }
}

/// `value` lower-cased and trimmed, its keywords in the order the table in
/// [`Display::parse`] spells them: the outer (`block`, `inline`), then the
/// inner (`flow`, `flow-root`, `flex`, …), then `list-item`; a list item's
/// `flow` dropped and its missing outer taken as `block`. `None` for a
/// keyword given twice. A single keyword passes through.
fn canonical(value: &str) -> Option<String> {
    let value = value.trim().to_ascii_lowercase();
    let words: Vec<&str> = value.split_ascii_whitespace().collect();
    if words.len() < 2 {
        return Some(value);
    }
    let rank = |w: &&str| match *w {
        "block" | "inline" => 0,
        "list-item" => 2,
        _ => 1,
    };
    let mut sorted = words.clone();
    sorted.sort_by_key(rank);
    sorted.dedup_by_key(|w| rank(w));
    if sorted.len() != words.len() {
        return None;
    }
    if sorted.last() == Some(&"list-item") {
        sorted.retain(|w| *w != "flow");
        if rank(&sorted[0]) != 0 {
            sorted.insert(0, "block");
        }
    }
    Some(sorted.join(" "))
}

#[cfg(test)]
mod tests;
