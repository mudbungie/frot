//! Embedded JS engine seam (rquickjs / quickjs-ng).
//!
//! Proves the four bounding primitives `docs/design/js.md` §1 requires of the
//! engine: `eval`, an interrupt hook spending the CPU budget (`EXEC_CPU_MS`), a
//! memory cap (`JS_MEM_LIMIT`), and a host-driven microtask queue. Everything the rest of
//! Phase 4 builds sits behind this narrow surface; the binding layer
//! (`js::syscall`) reaches the realm through [`Engine::context`]. No `rquickjs`
//! type escapes `src/js/` (js.md §1), as no `markup5ever` type leaks past `dom.rs`.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use rquickjs::context::EvalOptions;
use rquickjs::loader::{Loader, Resolver};
use rquickjs::{CatchResultExt, Coerced, Context, Ctx, FromJs, Runtime, Value};

mod clock;
mod module;
pub use clock::{Clock, Deadline, EXEC_CPU_MS, NET_BUDGET_MS};

/// Engine heap cap. `set_memory_limit` is only honoured on the default C
/// allocator, so `js::engine` never enables rquickjs's `allocator` feature.
pub const JS_MEM_LIMIT: usize = 64 * 1024 * 1024;

/// Why an `eval` stopped short of a value.
#[derive(Debug, PartialEq, Eq)]
pub enum EvalError {
    /// The `EXEC_CPU_MS` compute budget tripped the interrupt handler mid-run.
    Budget,
    /// A JS exception (throw, OOM, syntax error) reached the top.
    Exception(String),
}

/// An embedded engine with one realm. `new` installs the interrupt hook and
/// memory cap once. [`Engine::arm`] opens the run's two §5 windows — the
/// `cpu` compute budget the interrupt handler spends, and the `net` wall
/// deadline the §6 subfetch seam dispatches inside; the three evals run inside
/// them (the event loop drives every script, lifecycle dispatch, and timer
/// callback of one JS run under one arming, js.md §5), one per kind of source:
/// [`Engine::eval_armed`] for frot's own strict JS, [`Engine::eval_script`] for a
/// sloppy page classic script, and `eval_module` (`engine::module`) for an ES
/// module. [`Engine::eval`] is `arm` + `eval_script` for a standalone one-shot.
/// Each eval drains the microtask queue before returning.
pub struct Engine {
    rt: Runtime,
    ctx: Context,
    cpu: Deadline,
    net: Deadline,
    mem_limit: usize,
    tripped: Arc<AtomicBool>,
    rejections: Rc<Cell<i64>>,
}

impl Engine {
    /// Engine with the shipping bounds and memory cap.
    pub fn new() -> Self {
        Self::with_bounds(JS_MEM_LIMIT, Deadline::compute(), Deadline::network())
    }

    /// Engine with explicit limits: a heap cap plus the two §5 windows. Shipping
    /// passes [`Deadline::compute`] (CPU) and [`Deadline::network`] (wall); a
    /// test substitutes a [`Clock::manual`] window through [`Deadline::on`] and
    /// moves the two apart, so neither the budget interrupt, the §6 dispatch
    /// seam, nor the observable `__frot_now` reading depends on the host.
    pub fn with_bounds(mem_limit: usize, cpu: Deadline, net: Deadline) -> Self {
        let rt = Runtime::new().expect("quickjs runtime");
        rt.set_memory_limit(mem_limit);
        let tripped = Arc::new(AtomicBool::new(false));
        let (tr, budget) = (tripped.clone(), cpu.clone());
        rt.set_interrupt_handler(Some(Box::new(move || {
            if budget.expired() {
                tr.store(true, Ordering::Relaxed);
                true
            } else {
                false
            }
        })));
        // Unhandled promise rejection tracking (js.md §10). quickjs reports an
        // unhandled rejection (`is_handled == false`) and a later handle (`true`);
        // the running net is the still-unhandled count, read after the settle loop.
        let rejections = Rc::new(Cell::new(0));
        let rej = rejections.clone();
        rt.set_host_promise_rejection_tracker(Some(Box::new(
            move |_: Ctx<'_>, _: Value<'_>, _: Value<'_>, is_handled: bool| {
                rej.set(rej.get() + if is_handled { -1 } else { 1 });
            },
        )));
        let ctx = Context::full(&rt).expect("quickjs context");
        Self {
            rt,
            ctx,
            cpu,
            net,
            mem_limit,
            tripped,
            rejections,
        }
    }

    /// The engine's armed *network* window, shared with the §6 subfetch cache so
    /// dispatch obeys the run's one wall deadline. Compute is not this bound: a
    /// blocked socket burns no CPU, so the two resources are priced apart (§5).
    pub fn deadline(&self) -> Deadline {
        self.net.clone()
    }

    /// The engine's observable [`Clock`] — the wall clock the browser clock reads
    /// through the `__frot_now` syscall (`bl-e707`), the same one
    /// [`deadline`](Self::deadline) bounds network with.
    pub fn clock(&self) -> Clock {
        self.net.clock()
    }

    /// The engine's realm, for the binding layer to install host functions on
    /// (`js::syscall`). rquickjs types stay inside `src/js/` (js.md §1).
    pub fn context(&self) -> &Context {
        &self.ctx
    }

    /// Install the ES-module resolver/loader (js.md §4.1/§6) on the runtime — the
    /// `import` graph resolves specifiers and loads source through it. Gated by
    /// rquickjs's `loader` feature; `js::loader` supplies the concrete pair over
    /// the shared §6 subfetch cache. rquickjs types stay inside `src/js/`.
    pub fn set_loader<R, L>(&self, resolver: R, loader: L)
    where
        R: Resolver + 'static,
        L: Loader + 'static,
    {
        self.rt.set_loader(resolver, loader);
    }

    /// Open both §5 windows (`EXEC_CPU_MS` of CPU, `NET_BUDGET_MS` of wall, each
    /// from now) and clear the trip flag. The event loop arms once per JS run so
    /// the bounds span the whole run — scripts *and* the settle loop — not each
    /// script (js.md §5).
    pub fn arm(&self) {
        self.tripped.store(false, Ordering::Relaxed);
        self.cpu.arm();
        self.net.arm();
    }

    /// `arm` then [`eval_script`](Self::eval_script) — one page classic script
    /// with its own budget window (`docs/design/js.md` §1 smoke surface, and the
    /// shape every test that stands in for a page script wants).
    pub fn eval(&self, src: &str) -> Result<String, EvalError> {
        self.arm();
        self.eval_script(src)
    }

    /// Run one-time host setup (`js::syscall::install`) outside *all three* page
    /// bounds (js.md §5: they cover page scripts and the event loop, never the API
    /// install). Both §5 windows are disarmed, so however slow the environment
    /// (llvm-cov, a loaded CI box) the interrupt cannot trip mid-install and turn
    /// setup into a spurious budget failure (bl-5ac3); and the heap cap is lifted
    /// for the duration and restored after, so the prelude — frot's own
    /// fixed-size program, not page input — is never *parsed* under starvation
    /// (bl-c385: quickjs's `js_parse_block` ignores a failed `push_scope` and
    /// `pop_scope` then reads a garbage scope index — a segfault, not an error).
    /// Restoring from the constructed limit makes nesting safe. Page scripts
    /// re-arm via [`arm`](Self::arm)/[`eval_armed`](Self::eval_armed) first.
    pub fn setup<T>(&self, f: impl FnOnce() -> T) -> T {
        self.tripped.store(false, Ordering::Relaxed);
        self.cpu.disarm();
        self.net.disarm();
        // 0 is quickjs's "unlimited" (`malloc_limit - 1` wraps to `SIZE_MAX`).
        self.rt.set_memory_limit(0);
        let out = f();
        self.rt.set_memory_limit(self.mem_limit);
        out
    }

    /// [`setup`](Self::setup) around one eval — host setup source (the prelude).
    pub fn eval_setup(&self, src: &str) -> Result<String, EvalError> {
        self.setup(|| self.eval_armed(src))
    }

    /// Evaluate **frot's own** JS — the prelude and the event-loop drivers —
    /// inside the current budget window. Strict, because that is the mode the
    /// prelude is written in; page input goes through
    /// [`eval_script`](Self::eval_script) or [`eval_module`](Self::eval_module).
    pub fn eval_armed(&self, src: &str) -> Result<String, EvalError> {
        self.eval_global(src, true)
    }

    /// Evaluate one **page classic script** inside the current budget window
    /// (js.md §4.1). Classic scripts are *sloppy* unless their own source opts
    /// in with a `'use strict'` directive, which the parser still honours — so
    /// this is the one eval that turns rquickjs's `EvalOptions::default`
    /// strictness **off** (bl-0679: forcing it made every SvelteKit page throw
    /// on the generated `__sveltekit_*` global its untyped inline script assigns
    /// undeclared). ES modules stay strict by definition ([`eval_module`]).
    pub fn eval_script(&self, src: &str) -> Result<String, EvalError> {
        self.eval_global(src, false)
    }

    /// Global-code eval in the named language mode, draining the microtask queue,
    /// returning the result coerced to a string. Once the budget has tripped the
    /// result is [`EvalError::Budget`] regardless of the eval's own outcome — so
    /// a JS `try/catch` that swallows the interrupt still stops the loop.
    fn eval_global(&self, src: &str, strict: bool) -> Result<String, EvalError> {
        let res = self.ctx.with(|ctx| {
            // Every flag stated, none inherited: `global` is script-not-module
            // code, `promise` off because top-level await is a module thing, and
            // the backtrace barrier off so a throw names the page's own frames.
            let mut opts = EvalOptions::default();
            opts.global = true;
            opts.strict = strict;
            opts.backtrace_barrier = false;
            opts.promise = false;
            ctx.eval_with_options::<Value, _>(src, opts)
                .catch(&ctx)
                .map(|v| coerce_string(&ctx, v))
                .map_err(|e| e.to_string())
        });
        self.drain_jobs();
        if self.tripped.load(Ordering::Relaxed) {
            Err(EvalError::Budget)
        } else {
            res.map_err(EvalError::Exception)
        }
    }

    /// Host-driven microtask drain (§5): run pending jobs until the queue is
    /// empty or the budget trips.
    fn drain_jobs(&self) {
        while self.rt.is_job_pending() && !self.tripped.load(Ordering::Relaxed) {
            let _ = self.rt.execute_pending_job();
        }
    }
}

/// Coerce a completion value to a string for the host protocol, defensively: a
/// page script's completion is *not* the page's output, and some values won't
/// `ToString` (a bare `Object.create(null)`, a framework's public proxy — Vue's
/// `mount()` returns one). Such a coercion throw is the host's problem, never a
/// script error (js.md §10: errors are the page's throws), so it degrades to the
/// empty string rather than surfacing as an [`EvalError::Exception`]. Primitive
/// completions (the event-loop drivers' integers, string evals) coerce as usual.
fn coerce_string<'js>(ctx: &Ctx<'js>, v: Value<'js>) -> String {
    Coerced::<String>::from_js(ctx, v)
        .map(|c| c.0)
        .unwrap_or_default()
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
