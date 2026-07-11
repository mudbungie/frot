//! The bundled JS prelude (js.md §3) — the web-facing API, written in JS on top
//! of the syscall table so shim breadth grows without widening the Rust
//! interface. Files are concatenated and evaluated once, before any page
//! script, by [`super::syscall::install`].
//!
//! Coverage note (js.md §3): `cargo llvm-cov` cannot see these JS lines; the
//! prelude is exercised end-to-end by the golden fixture suite (subtask 9). The
//! Rust syscall closures it calls are covered directly in `syscall::tests`.
//!
//! This is the **core** prelude: `Node`/`Element`/`Document`, `querySelector`,
//! `innerHTML`/`textContent`, and `console`. The event loop, timers, `fetch`,
//! storage and `navigator`/`location` breadth arrive with subtasks 4.5–4.8.

/// The concatenated prelude source. Each module is an IIFE over `globalThis`,
/// so order is irrelevant and nothing leaks but the intended globals.
pub const SOURCE: &str = concat!(
    include_str!("prelude/console.js"),
    "\n",
    include_str!("prelude/dom.js"),
);
