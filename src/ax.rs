//! Accessibility-tree machinery: role resolution, accessible-name computation,
//! and the AX-tree builder for `--out ax`.

pub mod name;
pub mod role;

pub use name::accessible_name;
pub use role::{level, role};
