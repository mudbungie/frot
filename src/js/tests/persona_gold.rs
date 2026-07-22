//! Persona navigator contract (identity.md §8) — the JS-probe half of the
//! deterministic A/B instrument (`bl-46f5`). Sibling to `envgold`: that fixture
//! pins the §7 *host* surface, this one the §8 *identity* surface, and both are
//! offline `make cov` gates so an identity regression fails the build with no
//! network. `bl-3972` (JS persona) edits the fixture, not this driver.

use super::super::{run, Env, StyleSource};
use crate::dom::{Document, NodeKind};
use crate::fetch::{user_agent, FetchSession};

/// The pinned probe page — data, not a dependency (its assertions live in JS).
const PERSONA_NAV: &str = include_str!("../../../tests/fixtures/js/persona-navigator.html");

/// Read a `data-*` attribute the probe wrote onto `<html>` (the documentElement).
fn de_attr(doc: &Document, name: &str) -> Option<String> {
    let de = doc.find_by_tag("html")[0];
    let NodeKind::Element(el) = &doc.node(de).kind else {
        unreachable!("html is an element")
    };
    el.attr(name).map(str::to_string)
}

#[test]
fn navigator_persona_contract_holds() {
    // Drive with the REAL UA frot sends (not the test stub `envgold` uses), so
    // the host-visible `navigator.userAgent` is asserted against the wire persona.
    let ua = user_agent(&[]);
    let env = Env {
        url: "https://example.com/".into(),
        user_agent: ua.clone(),
        accept_language: crate::fetch::accept_language(&[]),
    };
    let (doc, report) = run(
        Document::parse(PERSONA_NAV),
        StyleSource::Bare,
        env,
        &FetchSession::new(Vec::new()),
    );
    // The probe self-checks every §8 fact in JS; `data-fail` is the JSON array of
    // any that regressed. Empty == all held.
    let fails = de_attr(&doc, "data-fail").expect("probe wrote data-fail");
    assert_eq!(fails, "[]", "navigator persona regressions: {fails}");
    assert_eq!(
        de_attr(&doc, "data-done").as_deref(),
        Some("1"),
        "probe did not complete"
    );
    // The load-bearing §8 invariant: `navigator.userAgent` is EXACTLY the UA frot
    // sent — the shim never lies about who fetched. Pinned to the real value, so a
    // 121->140 profile bump is caught here without a fixture edit.
    assert_eq!(de_attr(&doc, "data-ua").as_deref(), Some(ua.as_str()));
    // Facts only the host sees: one script ran, nothing threw, the run settled.
    assert_eq!(
        (report.scripts, report.errors, report.settled),
        (1, 0, true)
    );
}
