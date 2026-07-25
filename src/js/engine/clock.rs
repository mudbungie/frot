//! The injectable monotonic clocks (`docs/design/js.md` §5, `bl-e707`/`bl-8dc0`)
//! and the budget window read off one.
//!
//! §5 bounds two resources, in two units, because one unit priced both wrongly.
//! frot's *own* work — script compile, interpretation, DOM syscalls, GC — is
//! spent in **CPU** time and bounded by [`EXEC_CPU_MS`] at the engine interrupt;
//! network waiting is elapsed **wall** time and bounded by [`NET_BUDGET_MS`] at
//! the §6 subfetch seam. Charging compute to wall time made a *correct* run on a
//! busy host emit a *different* envelope (the host's scheduling luck priced into
//! frot's product); charging network to CPU time would make a blocked socket
//! free. So a session holds two clocks and arms two windows.
//!
//! The wall clock is also the single *observable* browser clock — `__frot_now`,
//! whence `performance.now`/`Date.now`, and the origin every §6 resource timing
//! is bracketed on — so a blocking subfetch's real elapsed is at once visible to
//! the page and charged against the network deadline.
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

/// §5/§6 network deadline: the wall time within which the subfetch seam may
/// dispatch. Elapsed time, because network wait is not frot's work.
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

/// One §5 bound: a budget spent on one [`Clock`], armed once per run. Cloning
/// shares the armed window, so the engine interrupt handler, the §6 subfetch
/// cache, and the run driver all read one authority rather than a copy.
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

    /// The shipping network bound: [`NET_BUDGET_MS`] of wall time, measured on
    /// the clock the page also observes.
    pub fn network() -> Self {
        Self::on(Clock::wall(), Duration::from_millis(NET_BUDGET_MS))
    }

    /// `budget` measured on `clock` — the injection seam a test drives a
    /// [`Clock::manual`] through, so the compute and network windows move
    /// independently and neither depends on the host.
    pub fn on(clock: Clock, budget: Duration) -> Self {
        Deadline {
            clock,
            budget,
            at: Arc::new(AtomicU64::new(u64::MAX)),
        }
    }

    /// A window nobody arms, so nothing ever passes it — for callers holding no
    /// engine (the subfetch unit tests, whose subject is not time).
    pub fn never() -> Self {
        Self::on(Clock::wall(), Duration::ZERO)
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

    /// Time left in the armed window; `None` once passed. The concurrent warm
    /// (`bl-08f6`) times each parallel request from this, so the wave obeys the
    /// run's network deadline (js.md §5/§6) — nothing outlives it.
    pub fn remaining(&self) -> Option<Duration> {
        let at = self.at.load(Ordering::Relaxed);
        let now = self.clock.elapsed_nanos();
        (at > now).then(|| Duration::from_nanos(at - now))
    }

    /// The clock this window is spent on — for the network window, the observable
    /// browser clock the `__frot_now` syscall reads (`bl-e707`).
    pub fn clock(&self) -> Clock {
        self.clock.clone()
    }

    /// Nanoseconds elapsed on the underlying clock — the reading the §6 cache
    /// brackets a network dispatch with to record its real duration (`bl-e707`).
    pub(crate) fn elapsed_nanos(&self) -> u64 {
        self.clock.elapsed_nanos()
    }
}
