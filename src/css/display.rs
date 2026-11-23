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
    /// Chrome 158 (`bl-e3a0`).
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value.trim().to_ascii_lowercase().as_str() {
            "none" => Display::None,
            "block" | "flow" | "flow-root" | "grid" | "table" | "table-caption"
            | "table-column" | "table-column-group" | "-webkit-box" | "block flow"
            | "block flow-root" | "block grid" | "block table" | "block ruby" | "block math" => {
                Display::Block
            }
            "inline" | "ruby" | "ruby-text" | "math" | "inline flow" | "inline ruby"
            | "inline math" | "inline list-item" | "initial" | "unset" => Display::Inline,
            "inline-block" | "inline-grid" | "inline-table" | "-webkit-inline-box"
            | "inline flow-root" | "inline grid" | "inline table" => Display::InlineBlock,
            "list-item" | "block list-item" | "flow-root list-item" => Display::ListItem,
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
        matches!(self, Display::Block | Display::ListItem | Display::Flex)
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
            Display::InlineFlex if out_of_flow => Display::Flex,
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

#[cfg(test)]
mod tests;
