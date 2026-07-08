//! View modules: produce one output payload each from a [`crate::dom::Document`].
//!
//! Each view stays independent and side-effect-free.

pub mod bboxes;
pub mod dom;
pub mod forms;
pub mod links;
pub mod meta;
pub mod text;
