//! JS-layer tests, split to stay under the 300-line source cap: [`facade`]
//! guards the prelude web-API over the syscalls (Node/Element/Document, console,
//! geometry, the script queue), [`evloop`] the bounded virtual-clock event loop
//! (§5). Prelude JS lines are not `llvm-cov` visible; these guard the facade and
//! loop end-to-end until the golden fixture suite (4.9) owns them.

use std::time::Duration;

use super::{run, run_with, Env, Report, Session, StyleSource};
use crate::dom::Document;

mod evloop;
mod facade;

fn test_env() -> Env {
    Env {
        url: "https://example.com/".into(),
        user_agent: "frot-test/1".into(),
    }
}

fn sess(html: &str) -> Session {
    Session::new(Document::parse(html), StyleSource::Bare, test_env())
}

fn drive(html: &str) -> (Document, Report) {
    run(Document::parse(html), StyleSource::Bare, test_env())
}

/// Drive a page with a tight wall-clock budget so budget-trip paths resolve fast
/// and deterministically (the 4.1 spike's short-budget pattern).
fn drive_bounded(html: &str, budget_ms: u64) -> (Document, Report) {
    run_with(
        Document::parse(html),
        StyleSource::Bare,
        test_env(),
        Duration::from_millis(budget_ms),
    )
}
