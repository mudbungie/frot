//! Live capability-surface probe over the field corpus (`bl-bd4e`).
//!
//! Dated EVIDENCE, not CI: it hits the real network, so it is an `example`
//! (never run by `cargo test`, outside the 100% coverage gate) rather than a
//! test. The deterministic half of the instrument — `frot::js::measure` and its
//! fixtures — is gated; this driver only points it at live pages and prints a
//! ranked table per page, separating the three corpus classes.
//!
//! Run:  `cargo run --release --example probe_corpus`
//!
//! Corpus taken verbatim from `docs/design/identity.md` §3.6/§3.7 so every row
//! is traceable. Declared-challenge pages are fetched with EXACTLY ONE request
//! and their scripts are NEVER executed (`bl-abe5` boundary): the harness prints
//! the declaration and moves on.

use frot::dom::Document;
use frot::fetch::{self, FetchSession};
use frot::js::{measure, Env, StyleSource};

/// (class, url). `Challenge` pages are fetched once and never executed.
enum Class {
    Control,
    SoftGate,
    Fingerprint,
    Challenge,
}

const CORPUS: &[(Class, &str)] = &[
    // Access-focused corpus, verbatim from identity.md §3.6/§3.7.
    (Class::Control, "https://en.wikipedia.org/wiki/Main_Page"),
    (Class::Control, "https://news.ycombinator.com/"),
    (Class::Control, "https://lobste.rs/"),
    (Class::Control, "https://react.dev/"),
    (Class::SoftGate, "https://stackoverflow.com/questions"),
    (Class::SoftGate, "https://www.amazon.com/"),
    (Class::Challenge, "https://www.reddit.com/"),
    (Class::Challenge, "https://www.g2.com/"),
    // Fingerprint testbeds: pages that deliberately probe rendering/hardware
    // surfaces, added to answer *which* high-entropy surfaces get touched when a
    // page actually tries (the access corpus rarely reaches them). Still one
    // request, still bounded, still no challenge executed.
    (
        Class::Fingerprint,
        "https://fingerprintjs.github.io/fingerprintjs/",
    ),
    (Class::Fingerprint, "https://browserleaks.com/canvas"),
    (Class::Fingerprint, "https://browserleaks.com/webgl"),
    (
        Class::Fingerprint,
        "https://webbrowsertools.com/canvas-fingerprint/",
    ),
];

fn class_name(c: &Class) -> &'static str {
    match c {
        Class::Control => "control",
        Class::SoftGate => "soft-gate",
        Class::Fingerprint => "fingerprint-testbed",
        Class::Challenge => "declared-challenge",
    }
}

fn main() {
    println!("# frot capability-surface probe — live corpus");
    println!(
        "# date: run `date -u` alongside; UA: {}",
        fetch::user_agent(&[])
    );
    for (class, url) in CORPUS {
        println!("\n## [{}] {}", class_name(class), url);
        let fetched = match FetchSession::new(Vec::new()).navigate(url) {
            Ok(f) => f,
            Err(e) => {
                println!("  fetch error: {} — {}", e.kind, e.message);
                continue;
            }
        };
        let status = fetched
            .status
            .map(|s| s.to_string())
            .unwrap_or_else(|| "?".into());
        println!("  http: {status}");
        if matches!(class, Class::Challenge) {
            // One request, never executed (bl-abe5). Report and stop.
            println!("  declared challenge — scripts NOT executed (bl-abe5 boundary)");
            continue;
        }
        if fetched.status.is_some_and(|c| c >= 400) {
            println!("  blocked/error status — not executed");
            continue;
        }
        let env = Env {
            url: fetched.final_url.clone(),
            user_agent: fetch::user_agent(&[]).to_string(),
            accept_language: fetch::accept_language(&[]),
        };
        let m = measure(Document::parse(&fetched.body), StyleSource::Bare, env);
        println!(
            "  js: scripts={} errors={} settled={}",
            m.report.scripts, m.report.errors, m.report.settled
        );
        if m.probes.is_empty() {
            println!("  (no watched surface probed)");
        }
        for (surface, count) in &m.probes {
            println!("  {count:>4}  {surface}");
        }
    }
}
