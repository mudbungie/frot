//! The one injectable monotonic clock (`docs/design/js.md` §5, `bl-e707`) and the
//! deadline window read off it.
//!
//! A per-invocation session has exactly one [`Clock`] — the single authority the
//! §5 wall-clock budget (the engine interrupt and this [`Deadline`]) *and* the
//! observable browser clock (`performance.now`/`Date.now`, via `__frot_now`) both
//! read. Because the two share one source, a blocking subfetch's real wall time
//! is at once charged against the budget and visible to `performance.now()`;
//! timer jumps add a virtual offset in JS on top of the same reading, never a
//! second clock.
//!
//! Production wraps the host [`Instant`]; a test injects [`Clock::manual`] and
//! [`advance`](Clock::advance)s it explicitly, so every injected-clock proof is
//! exact and never sleeps.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The per-invocation monotonic clock. Cloning shares the underlying source, so
/// every reader — the interrupt handler, the [`Deadline`], and the `__frot_now`
/// syscall — observes one coherent timeline.
#[derive(Clone)]
pub struct Clock(Kind);

#[derive(Clone)]
enum Kind {
    /// The shipping clock: real elapsed since construction.
    Real(Instant),
    /// A test clock, frozen until advanced — nanoseconds elapsed, shared by the
    /// `Arc` so a test's handle and the session's engine read the same value.
    Manual(Arc<AtomicU64>),
}

impl Clock {
    /// The shipping clock — real host-monotonic time from now.
    pub fn real() -> Self {
        Clock(Kind::Real(Instant::now()))
    }

    /// A test clock frozen at zero until [`advance`](Self::advance)d, so a reading
    /// is a pure function of the advances applied — no real time flows, no flake.
    pub fn manual() -> Self {
        Clock(Kind::Manual(Arc::new(AtomicU64::new(0))))
    }

    /// Advance a manual clock by `d` (standing in for elapsed CPU/host/network
    /// time — a busy script, a blocking fetch); a real clock tracks the host and
    /// ignores the call.
    pub fn advance(&self, d: Duration) {
        match &self.0 {
            Kind::Manual(n) => {
                n.fetch_add(d.as_nanos() as u64, Ordering::Relaxed);
            }
            Kind::Real(_) => {}
        }
    }

    /// Nanoseconds elapsed on this clock — monotonic, never negative.
    pub fn elapsed_nanos(&self) -> u64 {
        match &self.0 {
            Kind::Real(base) => base.elapsed().as_nanos() as u64,
            Kind::Manual(n) => n.load(Ordering::Relaxed),
        }
    }
}

/// A shareable read handle on the engine's armed wall-clock window — the same
/// clock/deadline pair the §5 interrupt handler reads, one authoritative home.
/// The §6 subfetch cache consults it before dispatching network (the interrupt
/// fires only between JS instructions, so a blocking host-fetch chain would else
/// outrun the budget) and records each request's real duration off it.
pub struct Deadline {
    clock: Clock,
    deadline: Arc<AtomicU64>,
}

impl Deadline {
    /// A window that never closes — the disarmed engine's own representation
    /// (`u64::MAX`), for callers holding no engine (the subfetch unit tests).
    pub fn never() -> Self {
        Deadline {
            clock: Clock::real(),
            deadline: Arc::new(AtomicU64::new(u64::MAX)),
        }
    }

    /// The window over `clock` bounded at `deadline` nanos — [`Engine::deadline`]
    /// hands out one sharing the engine's own clock and deadline cell.
    ///
    /// [`Engine::deadline`]: super::Engine::deadline
    pub(crate) fn new(clock: Clock, deadline: Arc<AtomicU64>) -> Self {
        Deadline { clock, deadline }
    }

    /// Whether the armed window has passed.
    pub fn expired(&self) -> bool {
        self.clock.elapsed_nanos() >= self.deadline.load(Ordering::Relaxed)
    }

    /// Time left in the armed window; `None` once passed. The concurrent warm
    /// (`bl-08f6`) times each parallel request from this, so the wave obeys the
    /// run's *one* deadline (js.md §5/§6) — nothing outlives it.
    pub fn remaining(&self) -> Option<Duration> {
        let dl = self.deadline.load(Ordering::Relaxed);
        let now = self.clock.elapsed_nanos();
        (dl > now).then(|| Duration::from_nanos(dl - now))
    }

    /// Nanoseconds elapsed on the underlying clock — the reading the §6 cache
    /// brackets a network dispatch with to record its real duration (`bl-e707`).
    pub(crate) fn elapsed_nanos(&self) -> u64 {
        self.clock.elapsed_nanos()
    }
}
