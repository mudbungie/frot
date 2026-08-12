//! JS-layer tests, split to stay under the 300-line source cap: [`facade`]
//! guards the prelude web-API over the syscalls (Node/Element/Document, console,
//! geometry, the script queue), [`evloop`] the bounded virtual-clock event loop
//! (§5). Prelude JS lines are not `llvm-cov` visible; these guard the facade and
//! loop end-to-end until the golden fixture suite (4.9) owns them.

use std::time::Duration;

use super::engine::{Clock, Deadline, EXEC_CPU_MS, NET_BUDGET_MS};
use super::{run, Bounds, Env, Report, Session, StyleSource};
use crate::dom::Document;
use crate::fetch::FetchSession;

mod clock;
mod envgold;
mod evloop;
mod facade;
mod iterator;
mod net;
mod persona_gold;
mod scriptmode;

fn test_env() -> Env {
    Env {
        url: "https://example.com/".into(),
        user_agent: "frot-test/1".into(),
        accept_language: "en-US,en;q=0.5".into(),
    }
}

/// The shipping bounds measured on *frozen* manual clocks (bl-1e54/bl-8dc0):
/// nothing the test never advances can expire, so neither §5 window can be spent
/// by a loaded host. Two separate clocks, because the two windows are two
/// resources and a test may drive them apart.
fn frozen_bounds() -> Bounds {
    Bounds {
        cpu: Deadline::on(Clock::manual(), Duration::from_millis(EXEC_CPU_MS)),
        net: Deadline::on(Clock::manual(), Duration::from_millis(NET_BUDGET_MS)),
    }
}

fn sess(html: &str) -> Session {
    let bounds = frozen_bounds();
    Session::with_bounds(
        Document::parse(html),
        StyleSource::Bare,
        test_env(),
        &FetchSession::new(Vec::new()),
        bounds.cpu,
        bounds.net,
    )
}

/// Drive a page with the shipping bounds measured on *manual* clocks (bl-1e54).
/// These tests assert what the run produced — DOM effects, counts, `settled` —
/// never how long it took, so a host clock only exposed them to the host: under
/// contention the same work costs more elapsed time, a §5 window expires mid-run,
/// and a correct engine reports unsettled. Frozen clocks the test never advances
/// remove that input; the budget-trip tests below keep a real CPU clock, because
/// there the compute budget *is* the subject.
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
    run(
        Document::parse(html),
        StyleSource::Bare,
        env,
        &FetchSession::new(Vec::new()),
        frozen_bounds(),
    )
}

/// Drive a page with a tight *CPU* budget so budget-trip paths resolve fast and
/// deterministically (the 4.1 spike's short-budget pattern): a spinning script
/// burns CPU, so the real production clock trips it without any host dependence.
fn drive_bounded(html: &str, budget_ms: u64) -> (Document, Report) {
    run(
        Document::parse(html),
        StyleSource::Bare,
        test_env(),
        &FetchSession::new(Vec::new()),
        Bounds {
            cpu: Deadline::on(Clock::cpu(), Duration::from_millis(budget_ms)),
            net: Deadline::network(),
        },
    )
}
