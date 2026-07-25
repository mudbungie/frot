//! Persona navigator contract (identity.md §8) — the JS-probe half of the
//! deterministic A/B instrument (`bl-46f5`). Sibling to `envgold`: that fixture
//! pins the §7 *host* surface, this one the §8 *identity* surface, and both are
//! offline `make cov` gates so an identity regression fails the build with no
//! network. `bl-3972` (JS persona) edits the fixture, not this driver.

use super::super::Env;
use super::drive_env;
use crate::dom::{Document, NodeKind};
use crate::fetch::user_agent;

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
    // `drive_env` runs on frozen manual clocks (bl-1e54). This probe renders the
    // canvas/WebGL/audio fingerprints and is the heaviest page in the suite, so on
    // a wall clock a saturated `cargo test` overran the §5 budget and the run
    // honestly reported `errors: 1, settled: false` — a load-dependent flake.
    // bl-701b widened the budget to 60 s; frozen clocks supersede that, since a
    // wider wall-clock window is still a wall clock. Shipping now spends the
    // compute budget in CPU time instead (bl-8dc0), which is the same cure at the
    // production seam rather than only in the test.
    let (doc, report) = drive_env(PERSONA_NAV, env);
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

    // The 2D-canvas fingerprint (bl-05e6, identity.md §11) must be DETERMINISTIC
    // across independent invocations — the property a real device shows and the
    // whole point of seeding from a fixed profile const, not host entropy. Drive
    // the same page a SECOND time and assert the digest a fingerprinter would hash
    // is byte-identical. The in-JS checks above already proved it is well-formed
    // (image/png prefix), stable within a run, and content-varying; this is the
    // cross-invocation half the JS cannot see. A non-trivial hash guards against a
    // regression that silently emptied it.
    let canvas = de_attr(&doc, "data-canvas").expect("probe wrote data-canvas");
    assert!(
        canvas.starts_with("data:image/png;base64,") && canvas.len() > 64,
        "canvas fingerprint is not a well-formed, non-trivial PNG data URL"
    );
    // The WebGL fingerprint (bl-f624, identity.md §11) carries the same
    // cross-invocation determinism requirement: a coherent Firefox-on-Linux WebGL
    // masquerade whose `toDataURL` a fingerprinter hashes must be byte-identical
    // across independent runs, not merely within one (the in-JS checks proved the
    // vendor/renderer masking, the coherent Mesa/llvmpipe UNMASKED strings, and
    // within-run stability; this is the half the JS cannot see).
    let webgl = de_attr(&doc, "data-webgl").expect("probe wrote data-webgl");
    assert!(
        webgl.starts_with("data:image/png;base64,") && webgl.len() > 64,
        "webgl fingerprint is not a well-formed, non-trivial PNG data URL"
    );
    // The Web Audio fingerprint (bl-8733, identity.md §11) carries the same
    // cross-invocation determinism requirement: OfflineAudioContext.startRendering()
    // resolves an AudioBuffer whose float samples a fingerprinter hashes must be
    // byte-identical across independent runs, not merely within one (the in-JS
    // checks proved the surface shape, the [-1,1] range, within-run stability, and
    // graph-variance; this is the half the JS cannot see). The hash is a non-empty
    // numeric string (a rendered, non-silent buffer), guarding a silent regression.
    let audio = de_attr(&doc, "data-audio").expect("probe wrote data-audio");
    assert!(
        !audio.is_empty() && audio.parse::<f64>().is_ok(),
        "audio fingerprint is not a well-formed numeric digest: {audio:?}"
    );
    let (doc2, _) = drive_env(
        PERSONA_NAV,
        Env {
            url: "https://example.com/".into(),
            user_agent: ua,
            accept_language: crate::fetch::accept_language(&[]),
        },
    );
    assert_eq!(
        de_attr(&doc2, "data-canvas").as_deref(),
        Some(canvas.as_str()),
        "canvas fingerprint drifted between two invocations (must be deterministic)"
    );
    assert_eq!(
        de_attr(&doc2, "data-webgl").as_deref(),
        Some(webgl.as_str()),
        "webgl fingerprint drifted between two invocations (must be deterministic)"
    );
    assert_eq!(
        de_attr(&doc2, "data-audio").as_deref(),
        Some(audio.as_str()),
        "audio fingerprint drifted between two invocations (must be deterministic)"
    );
}
