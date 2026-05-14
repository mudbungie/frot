//! Accessibility-tree machinery: role resolution, accessible-name computation,
//! and the AX-tree builder for `--out ax`.
//!
//! Phase 1 only ships [`role`] and [`level`]; the rest lands in later tasks.

pub mod role;

pub use role::{level, role};
