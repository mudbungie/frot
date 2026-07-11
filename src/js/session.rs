//! The JS execution [`Session`]: one engine bound to one document (js.md §2).
//!
//! Constructing it installs the syscall table (`super::syscall`) and evaluates
//! the prelude; the run driver (`super::run`) then arms the deadline and feeds it
//! scripts and event-loop driver calls. It owns the engine, shares the arena and
//! the §6 subfetch cache with the syscall closures, and captures `console`.

use std::cell::{Ref, RefCell};
use std::rc::Rc;
use std::time::Duration;

use crate::dom::Document;

use super::engine::{self, Engine, EXEC_BUDGET_MS};
use super::{geometry, subfetch, syscall};
use super::{Env, EvalError, Log, StyleSource};

/// A JS execution session bound to one document. It owns the engine, shares the
/// arena with the syscall closures for the mutable JS window (js.md §2), and
/// captures `console` output. Constructing it installs the syscall table and
/// evaluates the prelude; [`Session::eval`] then runs page scripts.
pub struct Session {
    engine: Engine,
    doc: syscall::SharedDoc,
    console: syscall::Console,
    denials: syscall::Denials,
    subfetch: subfetch::SharedSubfetch,
    /// The final page URL — the base an inline `type="module"` script's imports
    /// resolve against (js.md §4.1); external modules use their own fetched URL.
    page_url: String,
}

impl Session {
    /// Bind `doc` to a fresh engine (shipping budget), install the syscall table,
    /// and load the prelude. `styles` is the geometry cache's styling policy
    /// (js.md §8): the pipeline passes [`StyleSource::Authored`] under `--css`,
    /// else [`StyleSource::Bare`]. `env` carries the static facts the §7
    /// environment shims (navigator/location/matchMedia) derive from.
    pub fn new(doc: Document, styles: StyleSource, env: Env) -> Self {
        Self::with_budget(doc, styles, env, Duration::from_millis(EXEC_BUDGET_MS))
    }

    /// [`Session::new`] with an explicit wall-clock budget for the engine — the
    /// event loop's single deadline (§5), dialed down by the budget-trip tests.
    pub fn with_budget(doc: Document, styles: StyleSource, env: Env, budget: Duration) -> Self {
        let engine = Engine::with_limits(engine::JS_MEM_LIMIT, budget);
        let doc = Rc::new(RefCell::new(doc));
        let console = Rc::new(RefCell::new(Vec::new()));
        let geo = Rc::new(RefCell::new(geometry::Geometry::new(styles)));
        let denials = Rc::new(RefCell::new(0));
        // The §6 cache is anchored at the page URL and rides the caller's -H
        // headers same-origin; both live in `env`, so build it before install
        // moves `env` into the environment shims.
        let subfetch = Rc::new(RefCell::new(subfetch::Subfetch::new(&env.url, env.headers.clone())));
        // The ES-module resolver/loader (js.md §4.1/§6) rides the same §6 cache as
        // fetch/XHR/external-src, installed before any module evaluates.
        super::loader::install(&engine, &subfetch);
        let page_url = env.url.clone();
        syscall::install(
            &engine,
            doc.clone(),
            console.clone(),
            geo,
            env,
            denials.clone(),
            subfetch.clone(),
        );
        Session {
            engine,
            doc,
            console,
            denials,
            subfetch,
            page_url,
        }
    }

    /// Arm the run's single wall-clock deadline (§5): the budget spans every
    /// script, lifecycle dispatch, and timer callback that follows, not each one.
    pub fn begin(&self) {
        self.engine.arm();
    }

    /// Evaluate a script/driver call inside the armed deadline (§5), draining
    /// microtasks — the [`super::run`] path's one execution primitive.
    pub fn run_task(&self, src: &str) -> Result<String, EvalError> {
        self.engine.eval_armed(src)
    }

    /// Evaluate `src` as an ES module named `name` (its URL) inside the armed
    /// deadline (§5), draining microtasks — the [`super::run`] path's module
    /// primitive. Imports resolve/load through the §6 loader; failures surface as
    /// [`EvalError`] exactly like [`run_task`](Self::run_task).
    pub fn run_module(&self, name: &str, src: &str) -> Result<String, EvalError> {
        self.engine.eval_module(name, src)
    }

    /// The final page URL — an inline module script's import base (js.md §4.1).
    pub(super) fn page_url(&self) -> &str {
        &self.page_url
    }

    /// Evaluate with a fresh per-call budget (the facade smoke surface used by
    /// the tests); [`run_task`](Self::run_task) is the event-loop path.
    pub fn eval(&self, src: &str) -> Result<String, EvalError> {
        self.engine.eval(src)
    }

    /// Borrow the post-mutation document (the pipeline consumes this after
    /// settle, js.md §9).
    pub fn document(&self) -> Ref<'_, Document> {
        self.doc.borrow()
    }

    /// Consume the session and reclaim the post-JS document. Dropping the engine
    /// first releases the syscall closures' shared handles, so the arena is
    /// solely owned again (js.md §2: mutation ends at settle).
    pub fn into_document(self) -> Document {
        let Session { engine, doc, .. } = self;
        drop(engine);
        Rc::into_inner(doc)
            .expect("engine drop leaves the arena solely owned")
            .into_inner()
    }

    /// The `console` lines captured so far, in emission order.
    pub fn console(&self) -> Ref<'_, Vec<Log>> {
        self.console.borrow()
    }

    /// Navigations the shim refused (js.md §7/§11) — the counted-no-op tally the
    /// wiring layer folds into the envelope `js.errors` count (§10).
    pub fn denials(&self) -> u32 {
        *self.denials.borrow()
    }

    /// Unhandled promise rejections (§10) observed across the run — the engine's
    /// net tally, read once after the settle loop so a late `.catch` un-counts.
    pub fn rejections(&self) -> u32 {
        self.engine.rejections()
    }

    /// Fetch `spec` through the once-then-frozen §6 cache (the external-`src`
    /// path); `fetch`/XHR reach the same cache through the `__frot_subfetch`
    /// syscall.
    pub(super) fn subfetch(&self, spec: &str) -> subfetch::Outcome {
        self.subfetch.borrow_mut().get(spec)
    }
}
