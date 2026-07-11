//! The bundled JS prelude (js.md §3) — the web-facing API, written in JS on top
//! of the syscall table so shim breadth grows without widening the Rust
//! interface. Files are concatenated and evaluated once, before any page
//! script, by [`super::syscall::install`].
//!
//! Coverage note (js.md §3): `cargo llvm-cov` cannot see these JS lines; the
//! prelude is exercised end-to-end by the golden fixture suite (subtask 9). The
//! Rust syscall closures it calls are covered directly in `syscall::tests`.
//!
//! This is the **core** prelude plus the geometry facade and the §7 environment
//! breadth: `Node`/`Element`/`Document`, `querySelector`, `innerHTML`/
//! `textContent`, `console`, the geometry facade (`getBoundingClientRect`/
//! `offset*`/`getComputedStyle`, js.md §8), and the environment shims (`env.js`:
//! storage, cookie, `navigator`/`location`/`matchMedia`, `self`/`window`
//! aliasing, spec-legal denials). The event loop, timers, and `fetch` breadth
//! arrive with subtasks 4.5–4.6.

/// The concatenated prelude source. Each module is an IIFE over `globalThis`, so
/// order matters only where one module extends another's globals: `env.js` runs
/// last because it extends the `document` and `Node` that `dom.js` defines.
pub const SOURCE: &str = concat!(
    include_str!("prelude/console.js"),
    "\n",
    include_str!("prelude/dom.js"),
    "\n",
    include_str!("prelude/geometry.js"),
    "\n",
    include_str!("prelude/env.js"),
);
