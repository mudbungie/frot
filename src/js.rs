//! `--js` capability (Phase 4).
//!
//! The one capability permitted to mutate the DOM: page scripts run in an
//! embedded engine against a facade over the *same* arena everything else
//! reads (single source of truth — no mirror DOM). See `docs/design/js.md`.
//!
//! Layering (js.md §3): [`engine`] is the swappable rquickjs seam; [`syscall`]
//! installs the narrow host-function table over a shared [`Document`];
//! [`prelude`] is the JS web-API built on those calls. No `rquickjs` type
//! escapes `src/js/`, mirroring how no `markup5ever` type leaks past `dom.rs`.

pub mod engine;
mod geometry;
mod prelude;
mod syscall;

use std::cell::{Ref, RefCell};
use std::rc::Rc;

use crate::dom::Document;
use engine::Engine;

pub use engine::EvalError;
pub use geometry::StyleSource;
pub use syscall::Log;

/// A JS execution session bound to one document. It owns the engine, shares the
/// arena with the syscall closures for the mutable JS window (js.md §2), and
/// captures `console` output. Constructing it installs the syscall table and
/// evaluates the prelude; [`Session::eval`] then runs page scripts.
pub struct Session {
    engine: Engine,
    doc: syscall::SharedDoc,
    console: syscall::Console,
}

impl Session {
    /// Bind `doc` to a fresh engine, install the syscall table, and load the
    /// prelude. `styles` is the geometry cache's styling policy (js.md §8): the
    /// pipeline passes [`StyleSource::Authored`] under `--css`, else
    /// [`StyleSource::Bare`].
    pub fn new(doc: Document, styles: StyleSource) -> Self {
        let engine = Engine::new();
        let doc = Rc::new(RefCell::new(doc));
        let console = Rc::new(RefCell::new(Vec::new()));
        let geo = Rc::new(RefCell::new(geometry::Geometry::new(styles)));
        syscall::install(&engine, doc.clone(), console.clone(), geo);
        Session {
            engine,
            doc,
            console,
        }
    }

    /// Evaluate a page script, draining microtasks, and coerce its value to a
    /// string (the engine's smoke surface; the event loop lands in subtask 4.5).
    pub fn eval(&self, src: &str) -> Result<String, EvalError> {
        self.engine.eval(src)
    }

    /// Borrow the post-mutation document (the pipeline consumes this after
    /// settle, js.md §9).
    pub fn document(&self) -> Ref<'_, Document> {
        self.doc.borrow()
    }

    /// The `console` lines captured so far, in emission order.
    pub fn console(&self) -> Ref<'_, Vec<Log>> {
        self.console.borrow()
    }
}

#[cfg(test)]
mod tests;
