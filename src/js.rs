//! `--js` capability (Phase 4).
//!
//! The one capability permitted to mutate the DOM: page scripts run in an
//! embedded engine against a facade over the *same* arena everything else
//! reads (single source of truth — no mirror DOM). See `docs/design/js.md`.
//!
//! This module is the containment boundary for the JS engine: nothing outside
//! `js::engine` names an `rquickjs` type, mirroring how no `markup5ever` type
//! leaks past `dom.rs`. The engine is swappable without touching the shim.

pub mod engine;
