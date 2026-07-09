//! Embedded JS engine seam (rquickjs / quickjs-ng).
//!
//! Proves the four bounding primitives `docs/design/js.md` §1 requires of the
//! engine: `eval`, a wall-clock interrupt hook (`EXEC_BUDGET_MS`), a memory
//! cap (`JS_MEM_LIMIT`), and a host-driven microtask queue. Everything the
//! rest of Phase 4 builds sits behind this narrow surface; no `rquickjs` type
//! escapes it.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rquickjs::{CatchResultExt, Coerced, Context, Runtime};

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
/// memory cap once; each `eval` re-arms the deadline and drains microtasks.
pub struct Engine {
    rt: Runtime,
    ctx: Context,
    budget: Duration,
    base: Instant,
    deadline: Arc<AtomicU64>,
    tripped: Arc<AtomicBool>,
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
        let ctx = Context::full(&rt).expect("quickjs context");
        Self {
            rt,
            ctx,
            budget,
            base,
            deadline,
            tripped,
        }
    }

    /// Evaluate a script, drain the microtask queue, and return the result
    /// coerced to a string (`docs/design/js.md` §1 smoke surface).
    pub fn eval(&self, src: &str) -> Result<String, EvalError> {
        self.tripped.store(false, Ordering::Relaxed);
        let deadline = (self.base.elapsed() + self.budget).as_nanos() as u64;
        self.deadline.store(deadline, Ordering::Relaxed);
        let res = self.ctx.with(|ctx| {
            ctx.eval::<Coerced<String>, _>(src)
                .catch(&ctx)
                .map(|c| c.0)
                .map_err(|e| e.to_string())
        });
        self.drain_jobs();
        res.map_err(|msg| {
            if self.tripped.load(Ordering::Relaxed) {
                EvalError::Budget
            } else {
                EvalError::Exception(msg)
            }
        })
    }

    /// Host-driven microtask drain (§5): run pending jobs until the queue is
    /// empty or the budget trips.
    fn drain_jobs(&self) {
        while self.rt.is_job_pending() && !self.tripped.load(Ordering::Relaxed) {
            let _ = self.rt.execute_pending_job();
        }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
