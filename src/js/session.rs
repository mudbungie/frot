//! The JS execution [`Session`]: one engine bound to one document (js.md §2).
//!
//! Constructing it installs the syscall table (`super::syscall`) and evaluates
//! the prelude; the run driver (`super::run`) then arms the deadline and feeds it
//! scripts and event-loop driver calls. It owns the engine, shares the arena and
//! the §6 subfetch cache with the syscall closures, and captures `console`.

use std::cell::{Ref, RefCell};
use std::rc::Rc;

use crate::dom::Document;
use crate::fetch::FetchSession;

use super::engine::{self, Deadline, Engine};
use super::probe::ProbeLog;
use super::{geometry, script, subfetch, syscall};
use super::{Env, EvalError, Log, StyleSource};

/// A JS execution session bound to one document. It owns the engine, shares the
/// arena with the syscall closures for the mutable JS window (js.md §2), and
/// captures `console` output. Constructing it installs the syscall table and
/// evaluates the prelude; [`Session::eval`] then runs page scripts.
pub struct Session {
    engine: Engine,
    doc: syscall::SharedDoc,
    console: syscall::Console,
    /// The §10 reporting sinks: the denial/reported counts folded into
    /// `js.errors`, plus the bounded message detail behind them (written by the
    /// `report` syscall and by [`Session::capture`]).
    counters: syscall::Counters,
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
    pub fn new(doc: Document, styles: StyleSource, env: Env, fetch: &FetchSession) -> Self {
        Self::with_bounds(
            doc,
            styles,
            env,
            fetch,
            Deadline::compute(),
            Deadline::network(),
        )
    }

    /// [`Session::new`] with the run's two §5 bounds given explicitly (`bl-8dc0`):
    /// `cpu` is the compute budget the engine interrupt spends, `net` the wall
    /// deadline the §6 seam dispatches inside — and the latter's clock is also
    /// the observable `performance.now`/`Date.now`. Shipping passes
    /// [`Deadline::compute`]/[`Deadline::network`]; a test injects
    /// [`Clock::manual`](super::engine::Clock::manual) windows and drives them
    /// apart, so no verdict depends on host load or real sleeping.
    pub fn with_bounds(
        doc: Document,
        styles: StyleSource,
        env: Env,
        fetch: &FetchSession,
        cpu: Deadline,
        net: Deadline,
    ) -> Self {
        Self::build(doc, styles, env, fetch, cpu, net, None)
    }

    /// [`Session::new`] with the capability-surface probe instrument attached
    /// (`bl-bd4e`, shipping budget): binds the `__frot_probe` syscall and
    /// evaluates the instrumentation prelude, so a measured run records which
    /// surfaces the page touched while producing frot's ordinary output. The
    /// [`super::probe::measure`] entry point owns the `log`.
    pub(crate) fn measuring(
        doc: Document,
        styles: StyleSource,
        env: Env,
        fetch: &FetchSession,
        log: ProbeLog,
    ) -> Self {
        Self::build(
            doc,
            styles,
            env,
            fetch,
            Deadline::compute(),
            Deadline::network(),
            Some(log),
        )
    }

    /// Shared construction for [`with_bounds`](Self::with_bounds) (shipping,
    /// `probe = None`) and [`measuring`](Self::measuring) (`probe = Some`): bind
    /// `doc` to a fresh engine, install the syscall table + prelude, and — when
    /// `probe` is `Some` — the probe syscall and instrumentation prelude too.
    fn build(
        doc: Document,
        styles: StyleSource,
        env: Env,
        fetch: &FetchSession,
        cpu: Deadline,
        net: Deadline,
        probe: Option<ProbeLog>,
    ) -> Self {
        let engine = Engine::with_bounds(engine::JS_MEM_LIMIT, cpu, net);
        let doc = Rc::new(RefCell::new(doc));
        let console = Rc::new(RefCell::new(Vec::new()));
        let geo = Rc::new(RefCell::new(geometry::Geometry::new(styles)));
        let counters = syscall::Counters {
            denials: Rc::new(RefCell::new(0)),
            reported: Rc::new(RefCell::new(0)),
            messages: Rc::new(RefCell::new(Vec::new())),
        };
        // The §6 cache is anchored at the page URL and dispatches through the
        // invocation's shared `fetch` session (its pool + `-H` scoping); build it
        // before install moves `env` into the environment shims. It shares the
        // engine's armed *network* window, so dispatch obeys the run's wall
        // deadline while compute is spent in CPU time elsewhere (§5/§6).
        let subfetch = Rc::new(RefCell::new(subfetch::Subfetch::new(
            fetch.clone(),
            &env.url,
            engine.deadline(),
        )));
        // The shared cookie jar (bl-6dad): the same jar the transport writes
        // `Set-Cookie` into, so `document.cookie` at `env.url` reads it and a JS
        // write feeds a later same-origin subfetch.
        let cookie = fetch.cookie_jar();
        // The ES-module resolver/loader (js.md §4.1/§6) rides the same §6 cache as
        // fetch/XHR/external-src, installed before any module evaluates.
        super::loader::install(&engine, &subfetch);
        let page_url = env.url.clone();
        syscall::install(
            &engine,
            syscall::Host {
                doc: doc.clone(),
                console: console.clone(),
                geo,
                env,
                counters: counters.clone(),
                subfetch: subfetch.clone(),
                cookie,
                clock: engine.clock(),
                probe,
            },
        );
        Session {
            engine,
            doc,
            console,
            counters,
            subfetch,
            page_url,
        }
    }

    /// Arm the run's two §5 windows — the CPU compute budget and the wall network
    /// deadline: both span every script, lifecycle dispatch, and timer callback
    /// that follows, not each one.
    pub fn begin(&self) {
        self.engine.arm();
    }

    /// Evaluate a script/driver call inside the armed bounds (§5), draining
    /// microtasks — the [`super::run`] path's one execution primitive.
    pub fn run_task(&self, src: &str) -> Result<String, EvalError> {
        self.engine.eval_armed(src)
    }

    /// Evaluate `src` as an ES module named `name` (its URL) inside the armed
    /// bounds (§5), draining microtasks — the [`super::run`] path's module
    /// primitive. Imports resolve/load through the §6 loader; failures surface as
    /// [`EvalError`] exactly like [`run_task`](Self::run_task).
    pub fn run_module(&self, name: &str, src: &str) -> Result<String, EvalError> {
        self.engine.eval_module(name, src)
    }

    /// The final page URL — an inline module script's import base (js.md §4.1).
    pub(super) fn page_url(&self) -> &str {
        &self.page_url
    }

    /// Whether the run's armed *network* window has passed — read once after the
    /// settle loop (js.md §5/§6): a loop that concluded only because the §6 seam
    /// refused network dispatch past that deadline reached quiescence, but not
    /// within its bounds, and must not report settled.
    pub(super) fn deadline_expired(&self) -> bool {
        self.engine.deadline().expired()
    }

    /// Evaluate with freshly armed bounds (the facade smoke surface used by
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
        *self.counters.denials.borrow()
    }

    /// Unhandled promise rejections (§10) observed across the run — the engine's
    /// net tally, read once after the settle loop so a late `.catch` un-counts.
    pub fn rejections(&self) -> u32 {
        self.engine.rejections()
    }

    /// Unhandled errors the shim reported (§10) — `reportError`/`window.onerror`/
    /// a dispatched window `'error'` event that nothing suppressed. The counted-
    /// no-op sibling of [`denials`](Self::denials), folded into `js.errors`.
    pub fn reported_errors(&self) -> u32 {
        *self.counters.reported.borrow()
    }

    /// Capture one host-observed §10 error message (a script/module `throw` or a
    /// failed `subfetch`) into the bounded sink — the `report` syscall captures
    /// its own. Detail only; the `js.errors` count is folded separately.
    pub(super) fn capture(&self, kind: &str, text: &str) {
        syscall::push_message(&self.counters.messages, kind, text);
    }

    /// The §10 error messages captured so far (bounded to `MESSAGES_MAX`), read
    /// once after the run to fold into the report.
    pub(super) fn messages(&self) -> Vec<syscall::Message> {
        self.counters.messages.borrow().clone()
    }

    /// Fetch `spec` through the once-then-frozen §6 cache (the external-`src`
    /// path) with its request `intent`; `fetch`/XHR reach the same cache through
    /// the `__frot_subfetch` syscall.
    pub(super) fn subfetch(&self, spec: &str, intent: crate::fetch::Intent) -> subfetch::Outcome {
        self.subfetch.borrow_mut().get(spec, intent)
    }

    /// Warm the §6 cache concurrently with the initial external scripts (bl-08f6):
    /// a preload-scanner pass over the parsed document so the source-ordered
    /// script queue finds each external `src` already frozen instead of blocking
    /// on it one round trip at a time. Discovery releases the document borrow
    /// before the cache's parallel dispatch, which rides the run's one deadline
    /// and byte pool.
    pub(super) fn warm_initial_scripts(&self) {
        let reqs = script::initial_externals(&self.doc.borrow());
        self.subfetch.borrow_mut().warm(&reqs);
    }
}
