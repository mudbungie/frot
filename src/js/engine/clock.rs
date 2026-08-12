//! The injectable monotonic clocks (`docs/design/js.md` §5, `bl-e707`/`bl-8dc0`)
//! and the budget window read off one.
//!
//! §5 bounds two resources, in two units, because one unit priced both wrongly.
//! frot's *own* work — script compile, interpretation, DOM syscalls, GC — is
//! spent in **CPU** time and bounded by [`EXEC_CPU_MS`] at the engine interrupt
//! ([`Deadline`]); network waiting is elapsed **wall** time and bounded by
//! [`NET_BUDGET_MS`] at the §6 subfetch seam ([`NetBudget`]). Charging compute to
//! wall time made a *correct* run on a busy host emit a *different* envelope (the
//! host's scheduling luck priced into frot's product); charging network to CPU
//! time would make a blocked socket free. So a session holds two clocks and two
//! bounds — one a *window* opened once, the other a *meter* only real dispatch
//! spends.
//!
//! The wall clock is also the single *observable* browser clock — `__frot_now`,
//! whence `performance.now`/`Date.now`, and the origin every §6 resource timing
//! is bracketed on — so a blocking subfetch's real elapsed is at once visible to
//! the page and charged against the network budget.
//!
//! Production reads the host (`Instant`; `CLOCK_THREAD_CPUTIME_ID` for CPU); a
//! test injects [`Clock::manual`] per window and [`advance`](Clock::advance)s it,
//! so the two can be driven apart and no proof ever sleeps.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// §5 compute budget: the CPU time one run may spend on frot's own work.
/// Host-independent — the same verdict on an idle laptop and a saturated CI box.
pub const EXEC_CPU_MS: u64 = 1_000;

/// §5/§6 network budget: the wall time one run may spend *on the wire*. Elapsed
/// time, because network wait is not frot's work — but only the wire's elapsed,
/// not the run's (bl-79dc): see [`NetBudget`].
pub const NET_BUDGET_MS: u64 = 1_000;

/// A monotonic clock. Cloning shares the underlying source, so every reader of
/// one clock — the interrupt handler, a [`Deadline`], the `__frot_now` syscall —
/// observes one coherent timeline.
#[derive(Clone)]
pub struct Clock(Kind);

#[derive(Clone)]
enum Kind {
    /// Elapsed host time since construction — what the page observes and what
    /// network is deadlined on.
    Wall(Instant),
    /// CPU nanoseconds this thread had burned at construction; elapsed is the
    /// delta. The JS run is single-threaded, so this thread's CPU *is* the run's.
    Cpu(u64),
    /// A test clock, frozen until advanced — nanoseconds elapsed, shared by the
    /// `Arc` so a test's handle and the session's engine read the same value.
    Manual(Arc<AtomicU64>),
}

impl Clock {
    /// The shipping observable clock — real host-monotonic time from now.
    pub fn wall() -> Self {
        Clock(Kind::Wall(Instant::now()))
    }

    /// The shipping compute clock — this thread's CPU time from now.
    pub fn cpu() -> Self {
        Clock(Kind::Cpu(thread_cpu_nanos()))
    }

    /// A test clock frozen at zero until [`advance`](Self::advance)d, so a reading
    /// is a pure function of the advances applied — no real time flows, no flake.
    pub fn manual() -> Self {
        Clock(Kind::Manual(Arc::new(AtomicU64::new(0))))
    }

    /// Advance a manual clock by `d` (standing in for elapsed CPU or host/network
    /// time — a busy script, a blocking fetch); a host clock tracks the host and
    /// ignores the call.
    pub fn advance(&self, d: Duration) {
        if let Kind::Manual(n) = &self.0 {
            n.fetch_add(d.as_nanos() as u64, Ordering::Relaxed);
        }
    }

    /// Nanoseconds elapsed on this clock — monotonic, never negative.
    pub fn elapsed_nanos(&self) -> u64 {
        match &self.0 {
            Kind::Wall(base) => base.elapsed().as_nanos() as u64,
            Kind::Cpu(base) => thread_cpu_nanos().saturating_sub(*base),
            Kind::Manual(n) => n.load(Ordering::Relaxed),
        }
    }
}

/// This thread's consumed CPU time, in nanoseconds. `CLOCK_THREAD_CPUTIME_ID` is
/// POSIX (Linux including the static musl target, and macOS) and has no `std`
/// API, which is the whole reason `libc` is a direct dependency — it is already
/// in the tree under tokio/rustls, so it costs no binary and no new transitive.
fn thread_cpu_nanos() -> u64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `clock_gettime` writes only the `timespec` it is handed, a live
    // local, and the clock id is a POSIX constant. A failure would leave the
    // zeroed value, i.e. a clock that never advances — never a bogus reading.
    unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut ts) };
    (ts.tv_sec as u64) * 1_000_000_000 + ts.tv_nsec as u64
}

/// The §5 *compute* bound: a budget spent on one [`Clock`], armed once per run.
/// Cloning shares the armed window, so the engine interrupt handler and the run
/// driver read one authority rather than a copy.
///
/// A fresh `Deadline` is *disarmed* — no window is open, so nothing has passed
/// it. [`arm`](Self::arm) opens `budget` from now; [`disarm`](Self::disarm)
/// returns it to that birth state (host setup is exempt from the page budget).
#[derive(Clone)]
pub struct Deadline {
    clock: Clock,
    budget: Duration,
    /// Nanoseconds on `clock` at which the window closes; `u64::MAX` = disarmed.
    at: Arc<AtomicU64>,
}

impl Deadline {
    /// The shipping compute bound: [`EXEC_CPU_MS`] of this thread's CPU time.
    pub fn compute() -> Self {
        Self::on(Clock::cpu(), Duration::from_millis(EXEC_CPU_MS))
    }

    /// `budget` measured on `clock` — the injection seam a test drives a
    /// [`Clock::manual`] through, so the compute and network bounds move
    /// independently and neither depends on the host.
    pub fn on(clock: Clock, budget: Duration) -> Self {
        Deadline {
            clock,
            budget,
            at: Arc::new(AtomicU64::new(u64::MAX)),
        }
    }

    /// Open the window: `budget` from now on this clock. §5 arms once per run,
    /// so one budget spans every script, lifecycle dispatch and timer callback.
    pub fn arm(&self) {
        let at = self.clock.elapsed_nanos() + self.budget.as_nanos() as u64;
        self.at.store(at, Ordering::Relaxed);
    }

    /// Close the window back to its disarmed birth state — nothing can pass it.
    /// Prelude install is host setup, exempt from the page budget (§5).
    pub fn disarm(&self) {
        self.at.store(u64::MAX, Ordering::Relaxed);
    }

    /// Whether the armed window has passed.
    pub fn expired(&self) -> bool {
        self.clock.elapsed_nanos() >= self.at.load(Ordering::Relaxed)
    }
}

/// The §5/§6 *network* bound: [`NET_BUDGET_MS`] of time actually spent on the
/// wire, metered at the §6 subfetch seam.
///
/// Deliberately **not** a [`Deadline`] (`bl-79dc`). A deadline armed at the run's
/// start is a *window* on the wall clock, and a wall window spans the whole
/// run — frot's own compute and the host's scheduling luck included — so a page
/// heavy enough to spend that window on **compute** had its next subfetch refused
/// and the run reported `stopped: "network"` with no network having been slow at
/// all. That is the same mispricing `bl-8dc0` removed from the compute side: one
/// resource charged in another's unit. A budget only real dispatch can spend
/// makes the §10 name true by construction — a refusal means the wire really did
/// take [`NET_BUDGET_MS`] — and makes `VISION.md`'s arithmetic literal: up to
/// ~1 s of CPU **plus** up to ~1 s of network wall, *not one shared second*.
///
/// Cloning shares the meter, so the engine's handle and the §6 cache's are one.
#[derive(Clone)]
pub struct NetBudget {
    clock: Clock,
    /// The allowance, in nanoseconds on `clock`.
    budget: u64,
    /// Nanoseconds of dispatch charged so far.
    spent: Arc<AtomicU64>,
}

impl NetBudget {
    /// The shipping network bound: [`NET_BUDGET_MS`] of dispatch time, measured
    /// on the clock the page also observes.
    pub fn network() -> Self {
        Self::on(Clock::wall(), Duration::from_millis(NET_BUDGET_MS))
    }

    /// `budget` of dispatch time metered on `clock` — the injection seam a test
    /// drives a [`Clock::manual`] through, so a "slow origin" is a clock the mock
    /// server advances rather than a real sleep.
    pub fn on(clock: Clock, budget: Duration) -> Self {
        NetBudget {
            clock,
            budget: u64::try_from(budget.as_nanos()).unwrap_or(u64::MAX),
            spent: Arc::new(AtomicU64::new(0)),
        }
    }

    /// A budget no run can spend — for callers holding no engine (the subfetch
    /// unit tests, whose subject is not time).
    pub fn never() -> Self {
        Self::on(Clock::wall(), Duration::MAX)
    }

    /// The clock this budget is metered on — the observable browser clock the
    /// `__frot_now` syscall reads (`bl-e707`), so a blocking subfetch's real
    /// elapsed is at once visible to the page and charged here.
    pub fn clock(&self) -> Clock {
        self.clock.clone()
    }

    /// Whether the run has spent its whole network allowance — the §6 seam's one
    /// refusal test, and the sole cause of §10 `stopped: "network"`.
    pub fn spent_out(&self) -> bool {
        self.spent.load(Ordering::Relaxed) >= self.budget
    }

    /// What is left of the allowance. The concurrent warm (`bl-08f6`) times its
    /// parallel wave from this, so the wave obeys the run's network bound
    /// (js.md §5/§6) — nothing outlives it.
    pub fn left(&self) -> Duration {
        Duration::from_nanos(
            self.budget
                .saturating_sub(self.spent.load(Ordering::Relaxed)),
        )
    }

    /// Nanoseconds elapsed on the underlying clock — the reading the §6 cache
    /// brackets a network dispatch with to record its real duration (`bl-e707`).
    pub(crate) fn elapsed_nanos(&self) -> u64 {
        self.clock.elapsed_nanos()
    }

    /// Run `dispatch` and charge its elapsed to the budget, returning what it
    /// produced and how long it took. Every route to the wire goes through here,
    /// so no network time escapes the meter — and nothing but network time
    /// enters it, which is the whole point of the type.
    pub(crate) fn charge<T>(&self, dispatch: impl FnOnce() -> T) -> (T, u64) {
        let start = self.clock.elapsed_nanos();
        let out = dispatch();
        let dur = self.clock.elapsed_nanos().saturating_sub(start);
        self.spent.fetch_add(dur, Ordering::Relaxed);
        (out, dur)
    }
}
