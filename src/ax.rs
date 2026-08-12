//! Accessibility-tree machinery: role resolution, accessible-name computation,
//! and the AX-tree builder for `--out ax`.

pub mod hidden;
pub mod name;
pub mod role;
pub mod tree;

pub use hidden::excluded;
pub use name::accessible_name;
pub use role::{level, role};
pub use tree::ax_tree;
