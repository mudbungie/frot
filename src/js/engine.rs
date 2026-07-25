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

use rquickjs::function::This;
use rquickjs::loader::{Loader, Resolver};
use rquickjs::promise::{Promise, PromiseState};
use rquickjs::{CatchResultExt, Coerced, Context, Ctx, FromJs, Function, Module, Runtime, Value};

mod clock;
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
/// deadline the §6 subfetch seam dispatches inside; [`Engine::eval_armed`] runs
/// inside them (the event loop drives every script, lifecycle dispatch, and
/// timer callback of one JS run under one arming, js.md §5), while
/// [`Engine::eval`] is `arm` + `eval_armed` for a standalone one-shot. Each eval
/// drains the microtask queue before returning.
pub struct Engine {
    rt: Runtime,
    ctx: Context,
    cpu: Deadline,
    net: Deadline,
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

    /// `arm` then [`eval_armed`](Self::eval_armed) — a standalone one-shot eval
    /// with its own budget window (`docs/design/js.md` §1 smoke surface).
    pub fn eval(&self, src: &str) -> Result<String, EvalError> {
        self.arm();
        self.eval_armed(src)
    }

    /// Evaluate host setup source (the syscall prelude) exempt from the page
    /// budget (js.md §5: the bounds cover page scripts and the event loop, never
    /// the one-time API install). Disarms both windows first, so however slow the
    /// environment (llvm-cov, a loaded CI box) the interrupt cannot trip
    /// mid-install and turn setup into a spurious budget failure. Page scripts
    /// re-arm via [`arm`](Self::arm)/[`eval_armed`](Self::eval_armed) first.
    pub fn eval_setup(&self, src: &str) -> Result<String, EvalError> {
        self.tripped.store(false, Ordering::Relaxed);
        self.cpu.disarm();
        self.net.disarm();
        self.eval_armed(src)
    }

    /// Evaluate inside the current budget window, drain the microtask queue,
    /// and return the result coerced to a string. Once the budget has tripped
    /// the result is [`EvalError::Budget`] regardless of the eval's own outcome —
    /// so a JS `try/catch` that swallows the interrupt still stops the loop.
    pub fn eval_armed(&self, src: &str) -> Result<String, EvalError> {
        let res = self.ctx.with(|ctx| {
            ctx.eval::<Value, _>(src)
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

    /// Evaluate `src` as an ES module named `name` (its URL, the base every
    /// `import` resolves against, js.md §4.1/§6) inside the current budget
    /// window, then drain microtasks. The whole `import` graph loads synchronously
    /// through the installed loader ([`Engine::set_loader`]); an unresolvable
    /// specifier or failed module fetch throws here and surfaces as
    /// [`EvalError::Exception`] (a counted §10 error), sibling scripts continuing.
    /// Module evaluation is async by spec — the returned promise settles as its
    /// top-level await / dynamic `import()` drain here and, for timer-bound waits,
    /// across the §5 settle loop; a late unhandled rejection is caught by the §10
    /// net ([`Engine::rejections`]). A budget trip anywhere is [`EvalError::Budget`].
    pub fn eval_module(&self, name: &str, src: &str) -> Result<String, EvalError> {
        let before = self.rejections.get();
        let res = self.ctx.with(|ctx| -> Result<(), String> {
            let promise = Module::evaluate(ctx.clone(), name.to_string(), src.to_string())
                // A synchronous hard failure — syntax error, an unresolvable
                // specifier, or a failed module fetch (the loader threw) — reaches
                // here before a promise exists; counted once (js.md §4.1/§10).
                .catch(&ctx)
                .map_err(|e| e.to_string())?;
            self.watch_rejection(&ctx, &promise);
            // A module that throws *synchronously* at top level is already rejected
            // here, and quickjs may report that rejection to the tracker more than
            // once. The watcher owns this promise's outcome, so undo the tracker's
            // report for it (nothing else ran between `before` and now) — the watcher
            // counts it exactly once when its reject microtask drains.
            if promise.state() == PromiseState::Rejected {
                self.rejections.set(before);
            }
            Ok(())
        });
        self.drain_jobs();
        if self.tripped.load(Ordering::Relaxed) {
            Err(EvalError::Budget)
        } else {
            res.map(|()| String::new()).map_err(EvalError::Exception)
        }
    }

    /// Attach a rejection watcher to a module evaluation promise (js.md §10). As a
    /// handler it keeps the promise's *own* rejection from double-reporting through
    /// the tracker, and its reject arm counts that rejection once via the same net —
    /// so a top-level throw or a rejected top-level await counts as one, whenever it
    /// settles (inline, or later across the §5 loop). A `.catch` a page attaches
    /// itself still nets out through the tracker as before.
    fn watch_rejection<'js>(&self, ctx: &Ctx<'js>, promise: &Promise<'js>) {
        let rej = self.rejections.clone();
        let on_reject = Function::new(ctx.clone(), move |_v: Value<'js>| {
            rej.set(rej.get() + 1);
        })
        .expect("module rejection watcher");
        let noop = Function::new(ctx.clone(), || {}).expect("module settle noop");
        promise
            .then()
            .and_then(|t| t.call::<_, ()>((This(promise.clone()), noop, on_reject)))
            .expect("attach module rejection watcher");
    }

    /// Net unhandled promise rejections observed so far (§10). A late `.catch`
    /// un-counts an earlier report, so the net never ends negative in practice;
    /// it is clamped at zero and reported as a `u32`.
    pub fn rejections(&self) -> u32 {
        self.rejections.get().max(0) as u32
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
