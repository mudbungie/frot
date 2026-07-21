//! Capability-surface probe measurement (`bl-bd4e`).
//!
//! An **evidence instrument**, not a shipping capability: it records *which*
//! fingerprinting / feature-detection surfaces a page actually touches, so the
//! backlog of "which gaps are real" is measured rather than invented from
//! folklore (`docs/design/identity.md` §8/§10/§11, `bl-0356`).
//!
//! The mechanism is a second, optional prelude ([`INSTRUMENT`], `probe.js`)
//! layered over the shipping one. It wraps each watched surface so that reading
//! or calling it records a probe through the `__frot_probe` syscall and then
//! **returns exactly what frot returns today** — the same `null`/`undefined`/
//! value/throw. Recording without changing the observable outcome is the point:
//! the measured probe path is the path a real `--js` run takes, and the honest
//! line (`identity.md` §10 — "may not fabricate evidence of capabilities it does
//! not have") is not crossed by measuring.
//!
//! The instrument reuses the shipping run driver ([`super::drive`]) verbatim, so
//! a measured page executes under the same bounded virtual-clock loop (§5) as a
//! shipped one. Live-corpus results gathered through `examples/probe_corpus.rs`
//! are dated evidence, not CI; the deterministic half — this module and the
//! offline fixtures below — runs under the 100% coverage gate.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::dom::Document;
use crate::fetch::FetchSession;

use super::{run_session, Env, Report, Session, StyleSource};

/// The shared probe tally: a surface name → the number of times a page touched
/// it across the run. Insertion is the `__frot_probe` syscall's only write.
pub(crate) type ProbeLog = Rc<RefCell<BTreeMap<String, u32>>>;

/// The instrumentation prelude (`probe.js`): recording wrappers over the watched
/// capability surfaces, evaluated after the shipping prelude and before any page
/// script (so it can wrap the `navigator`/`getContext` the shipping prelude
/// defines). JS lines are not `llvm-cov`-visible (js.md §3) — the offline
/// fixture below exercises them end to end, the live corpus for evidence.
pub(crate) const INSTRUMENT: &str = include_str!("probe.js");

/// One measurement run's outcome: the JS [`Report`] (identical to a shipping
/// run's, since the driver is shared) plus the ranked probe tally.
pub struct Measurement {
    /// The execution report — scripts run, errors, settled — exactly as `--js`
    /// would produce, because [`measure`] drives the session through [`drive`].
    pub report: Report,
    /// `(surface, count)` rows, ranked by count descending then name — the
    /// table's raw material. Empty when the page probed nothing watched.
    pub probes: Vec<(String, u32)>,
}

/// Run `doc`'s scripts under the bounded loop with capability-surface
/// instrumentation installed, returning the [`Report`] and the ranked probe
/// tally. Same semantics as [`super::run`]; the only difference is the second
/// prelude and the `__frot_probe` sink. Deterministic and offline for a fixed
/// `doc`/`env` (the fixture test relies on this); over a live page it is dated
/// evidence.
pub fn measure(doc: Document, styles: StyleSource, env: Env) -> Measurement {
    let log: ProbeLog = Rc::new(RefCell::new(BTreeMap::new()));
    // The probe has no caller `-H`; its subfetches ride a fresh headerless
    // session (bl-5191), the same shared-pool seam a shipping `--js` run uses.
    let fetch = FetchSession::new(Vec::new());
    let session = Session::measuring(doc, styles, env, &fetch, log.clone());
    session.begin();
    let report = run_session(&session);
    let probes = rank(&log.borrow());
    Measurement { report, probes }
}

/// Rank the tally by count descending, breaking ties by surface name so the
/// output is deterministic for a fixed input (VISION principle 1).
fn rank(counts: &BTreeMap<String, u32>) -> Vec<(String, u32)> {
    let mut rows: Vec<(String, u32)> = counts.iter().map(|(k, c)| (k.clone(), *c)).collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    rows
}

#[cfg(test)]
mod tests;
