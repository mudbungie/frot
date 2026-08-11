//! Env-contract golden (bl-1463): the js.md §7 environment surface — plus the
//! §4.4/§5 lifecycle ordering — turned into an EXECUTABLE contract, so it cannot
//! drift silently. One fixture page (`tests/fixtures/js/env-contract.html`) probes
//! the whole surface in JS and self-checks every fact, recording any regression
//! into `data-fail`; this gate reads that array (empty == all pass) plus the
//! report facts only Rust can see. The fixture is the single place to edit when a
//! future ball adds or removes an API.

use super::drive;
use crate::dom::{Document, NodeKind};

/// The pinned probe page — data, not a dependency (its assertions live in JS).
const ENV_CONTRACT: &str = include_str!("../../../tests/fixtures/js/env-contract.html");

/// Read a `data-*` attribute the probe wrote onto `<html>` (the documentElement).
fn de_attr(doc: &Document, name: &str) -> Option<String> {
    let de = doc.find_by_tag("html")[0];
    let NodeKind::Element(el) = &doc.node(de).kind else {
        unreachable!("html is an element")
    };
    el.attr(name).map(str::to_string)
}

#[test]
fn env_contract_golden_pins_the_js_surface() {
    let (doc, report) = drive(ENV_CONTRACT);
    // The probe self-checks every §7 fact and each §4.4/§5 lifecycle transition in
    // JS; `data-fail` is the JSON array of any that regressed. Empty == all held.
    let fails = de_attr(&doc, "data-fail").expect("probe wrote data-fail");
    assert_eq!(fails, "[]", "env-contract regressions: {fails}");
    // Proof the run reached the post-load timer (the final phase), so no earlier
    // phase silently short-circuited the self-checks.
    assert_eq!(
        de_attr(&doc, "data-done").as_deref(),
        Some("1"),
        "probe did not reach the post-load timer"
    );
    // Facts only the host sees: both scripts ran (the inline probe + the `defer`
    // one, §4.2), the whole run settled (§5), and exactly two deliberate errors
    // counted into js.errors — the explicit reportError, and the throwing
    // MutationObserver callback (bl-07ab), which routes through the same §10
    // channel. Pinning the total also pins that nothing else threw uncaught.
    assert_eq!(
        (report.scripts, report.errors, report.settled()),
        (2, 2, true)
    );
}
