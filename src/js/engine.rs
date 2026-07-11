//! Embedded JS engine seam (rquickjs / quickjs-ng).
//!
//! Proves the four bounding primitives `docs/design/js.md` §1 requires of the
//! engine: `eval`, a wall-clock interrupt hook (`EXEC_BUDGET_MS`), a memory
//! cap (`JS_MEM_LIMIT`), and a host-driven microtask queue. Everything the
//! rest of Phase 4 builds sits behind this narrow surface. The binding layer
//! (`js::syscall`) reaches the realm through [`Engine::context`]; no `rquickjs`
//! type escapes `src/js/` (js.md §1), mirroring how no `markup5ever` type leaks
//! past `dom.rs`.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rquickjs::{CatchResultExt, Coerced, Context, Ctx, FromJs, Runtime, Value};

/// §5 / §13 OQ-1 defaults. Constants, not flags.
pub const EXEC_BUDGET_MS: u64 = 1_000;
/// Engine heap cap. `set_memory_limit` is only honoured on the default C
/// allocator, so `js::engine` never enables rquickjs's `allocator` feature.
pub const JS_MEM_LIMIT: usize = 64 * 1024 * 1024;

/// Why an `eval` stopped short of a value.
#[derive(Debug, PartialEq, Eq)]
pub enum EvalError {
    /// The wall-clock budget tripped the interrupt handler mid-run.
    Budget,
    /// A JS exception (throw, OOM, syntax error) reached the top.
    Exception(String),
}

/// An embedded engine with one realm. `new` installs the interrupt hook and
/// memory cap once. [`Engine::arm`] opens a single wall-clock deadline window;
/// [`Engine::eval_armed`] runs inside it (the event loop drives every script,
/// lifecycle dispatch, and timer callback of one JS run under one deadline,
/// js.md §5), while [`Engine::eval`] is `arm` + `eval_armed` for a standalone
/// one-shot. Each eval drains the microtask queue before returning.
pub struct Engine {
    rt: Runtime,
    ctx: Context,
    budget: Duration,
    base: Instant,
    deadline: Arc<AtomicU64>,
    tripped: Arc<AtomicBool>,
    rejections: Rc<Cell<i64>>,
}

impl Engine {
    /// Engine with the shipping budget and memory cap.
    pub fn new() -> Self {
        Self::with_limits(JS_MEM_LIMIT, Duration::from_millis(EXEC_BUDGET_MS))
    }

    /// Engine with explicit limits (the spike's tests dial these down).
    pub fn with_limits(mem_limit: usize, budget: Duration) -> Self {
        let rt = Runtime::new().expect("quickjs runtime");
        rt.set_memory_limit(mem_limit);
        let base = Instant::now();
        let deadline = Arc::new(AtomicU64::new(u64::MAX));
        let tripped = Arc::new(AtomicBool::new(false));
        let (dl, tr) = (deadline.clone(), tripped.clone());
        rt.set_interrupt_handler(Some(Box::new(move || {
            if base.elapsed().as_nanos() as u64 >= dl.load(Ordering::Relaxed) {
                tr.store(true, Ordering::Relaxed);
                true
            } else {
                false
            }
        })));
        // Unhandled promise rejection tracking (js.md §10). quickjs reports a
        // rejection with no handler (`is_handled == false`) and, if one is later
        // attached, a matching handle (`is_handled == true`); the running net is
        // the count of still-unhandled rejections, read after the settle loop.
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
            budget,
            base,
            deadline,
            tripped,
            rejections,
        }
    }

    /// The engine's realm, for the binding layer to install host functions on
    /// (`js::syscall`). rquickjs types stay inside `src/js/` (js.md §1).
    pub fn context(&self) -> &Context {
        &self.ctx
    }

    /// Open one wall-clock deadline window (`EXEC_BUDGET_MS` from now) and clear
    /// the trip flag. The event loop arms once per JS run so the budget spans the
    /// whole run — scripts *and* the settle loop — not each script (js.md §5).
    pub fn arm(&self) {
        self.tripped.store(false, Ordering::Relaxed);
        let deadline = (self.base.elapsed() + self.budget).as_nanos() as u64;
        self.deadline.store(deadline, Ordering::Relaxed);
    }

    /// `arm` then [`eval_armed`](Self::eval_armed) — a standalone one-shot eval
    /// with its own budget window (`docs/design/js.md` §1 smoke surface).
    pub fn eval(&self, src: &str) -> Result<String, EvalError> {
        self.arm();
        self.eval_armed(src)
    }

    /// Evaluate inside the current deadline window, drain the microtask queue,
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
    Coerced::<String>::from_js(ctx, v).map(|c| c.0).unwrap_or_default()
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
