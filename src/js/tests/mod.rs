//! JS-layer tests, split to stay under the 300-line source cap: [`facade`]
//! guards the prelude web-API over the syscalls (Node/Element/Document, console,
//! geometry, the script queue), [`evloop`] the bounded virtual-clock event loop
//! (§5). Prelude JS lines are not `llvm-cov` visible; these guard the facade and
//! loop end-to-end until the golden fixture suite (4.9) owns them.

use std::time::Duration;

use super::engine::{Clock, EXEC_BUDGET_MS};
use super::{run_with, Env, Report, Session, StyleSource};
use crate::dom::Document;
use crate::fetch::FetchSession;

mod clock;
mod envgold;
mod evloop;
mod facade;
mod net;
mod persona_gold;

fn test_env() -> Env {
    Env {
        url: "https://example.com/".into(),
        user_agent: "frot-test/1".into(),
        accept_language: "en-US,en;q=0.5".into(),
    }
}

fn sess(html: &str) -> Session {
    Session::with_budget(
        Document::parse(html),
        StyleSource::Bare,
        test_env(),
        &FetchSession::new(Vec::new()),
        Duration::from_millis(EXEC_BUDGET_MS),
        Clock::manual(),
    )
}

/// Drive a page with the shipping budget measured on a *manual* clock (bl-1e54).
/// These tests assert what the run produced — DOM effects, counts, `settled` —
/// never how long it took, so the shipping `Clock::real` only exposed them to the
/// host: under CPU contention the same work costs more wall time, the §5 budget
/// expires mid-run, and a correct engine reports unsettled. A frozen clock the
/// test never advances removes that input; the budget-trip tests below keep the
/// real clock, because there timing *is* the subject.
fn drive(html: &str) -> (Document, Report) {
    drive_env(html, test_env())
}

/// Drive a page whose §6 subfetches resolve against `url` — used by the external-
/// `src` path, where a `file://` base with an absent sibling fails deterministically
/// and offline (the repo test rule: never touch the real network).
fn drive_at(html: &str, url: &str) -> (Document, Report) {
    drive_env(
        html,
        Env {
            url: url.into(),
            user_agent: "frot-test/1".into(),
            accept_language: "en-US,en;q=0.5".into(),
        },
    )
}

/// [`drive`] with a caller-supplied [`Env`] — the golden persona gate drives the
/// real wire UA through here rather than the test stub.
fn drive_env(html: &str, env: Env) -> (Document, Report) {
    run_with(
        Document::parse(html),
        StyleSource::Bare,
        env,
        &FetchSession::new(Vec::new()),
        Duration::from_millis(EXEC_BUDGET_MS),
        Clock::manual(),
    )
}

/// Drive a page with a tight wall-clock budget so budget-trip paths resolve fast
/// and deterministically (the 4.1 spike's short-budget pattern).
fn drive_bounded(html: &str, budget_ms: u64) -> (Document, Report) {
    run_with(
        Document::parse(html),
        StyleSource::Bare,
        test_env(),
        &FetchSession::new(Vec::new()),
        Duration::from_millis(budget_ms),
        Clock::real(),
    )
}
