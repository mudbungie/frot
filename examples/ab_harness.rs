//! Field-corpus A/B harness (`bl-46f5`): prove the identity work changes outcomes.
//!
//! Runs a fixed corpus (identity.md §3.6/§3.7) against **two** frot binaries —
//! a baseline commit and a candidate — from one host, close in time, and diffs
//! the outcomes. Dated EVIDENCE, never CI: it hits the real network, so it is an
//! `example` (outside the 100% coverage gate), exactly like `probe_corpus.rs`.
//! The deterministic half of the instrument — the persona-contract fixtures under
//! `make cov` (`src/run/persona_tests.rs`, `src/fetch/firefox_tls/tests.rs`,
//! `src/js/tests/persona_gold.rs`) — is what CI runs with no network.
//!
//! ## Baseline pin
//!
//! Pin the baseline to a PRE-PHASE-5 commit so it captures genuine pre-transport
//! behavior. The recommended pin is `940ae36` (post-CI-wiring, pre-transport):
//!   git worktree add /tmp/frot-base 940ae36 && (cd /tmp/frot-base && make build)
//!   cargo build --release   # the candidate (Phase-5 tip)
//!   cargo run --release --example ab_harness -- \
//!       --baseline /tmp/frot-base/target/release/frot \
//!       --candidate ./target/release/frot --runs 3
//!
//! A NULL RESULT IS PUBLISHABLE (identity.md §14). If the candidate clears no
//! additional gate, this driver says so — it never adds a bypass to force one.

mod ab_harness {
    pub mod corpus;
    pub mod sha256;
}
use ab_harness::corpus::{self, Class, Probe, Target, CORPUS};

/// Recipes per class. Controls/soft-gate get a static and a `--js` recipe so a
/// JS-induced impression change is visible; challenge negative controls get one
/// recipe only and must prove non-execution (§3.7: one request, never executed).
fn recipes(class: Class) -> &'static [(&'static str, &'static [&'static str])] {
    match class {
        Class::Challenge => &[("dom", &["--out", "dom"])],
        Class::Oracle => &[("text", &["--out", "text"])],
        _ => &[
            ("dom", &["--out", "dom"]),
            ("js+dom", &["--js", "--out", "dom"]),
        ],
    }
}

struct Agg {
    probe: Probe,
    p50: u128,
    p95: u128,
    stable: bool,
}

fn aggregate(bin: &str, url: &str, recipe: &[&str], runs: usize) -> Agg {
    let mut probes: Vec<Probe> = (0..runs).map(|_| corpus::probe(bin, url, recipe)).collect();
    let mut walls: Vec<u128> = probes.iter().map(|p| p.wall_ms).collect();
    walls.sort_unstable();
    let first = probes.remove(0);
    // Stable iff every later run agrees on the identity-relevant outcome.
    let stable = probes.iter().all(|p| {
        p.status == first.status
            && p.needs == first.needs
            && p.error_kind == first.error_kind
            && p.http_status == first.http_status
            && p.sha == first.sha
    });
    Agg {
        probe: first,
        p50: percentile(&walls, 50),
        p95: percentile(&walls, 95),
        stable,
    }
}

fn percentile(sorted: &[u128], pct: usize) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = (pct * (sorted.len() - 1) + 50) / 100;
    sorted[idx.min(sorted.len() - 1)]
}

struct Args {
    baseline: String,
    candidate: String,
    runs: usize,
}

fn parse_args() -> Args {
    let mut a = Args {
        baseline: String::new(),
        candidate: "./target/release/frot".into(),
        runs: 3,
    };
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i + 1 < argv.len() {
        match argv[i].as_str() {
            "--baseline" => a.baseline = argv[i + 1].clone(),
            "--candidate" => a.candidate = argv[i + 1].clone(),
            "--runs" => a.runs = argv[i + 1].parse().unwrap_or(3),
            _ => {}
        }
        i += 2;
    }
    a
}

fn main() {
    let args = parse_args();
    if args.baseline.is_empty() {
        eprintln!("usage: ab_harness --baseline <frot> [--candidate <frot>] [--runs N]");
        eprintln!("  baseline should be built from a pre-Phase-5 commit (e.g. 940ae36).");
        std::process::exit(2);
    }
    println!("# frot A/B harness — field corpus (bl-46f5)");
    println!(
        "# run `date -u` alongside; baseline={} candidate={} runs={}",
        args.baseline, args.candidate, args.runs
    );
    println!("# baseline pin: build from 940ae36 (pre-transport); a null result is publishable.\n");

    let mut violations: Vec<String> = Vec::new();
    let mut parity_drift: Vec<String> = Vec::new();
    let mut gate_change = false;

    for target in CORPUS {
        report_target(
            target,
            &args,
            &mut violations,
            &mut parity_drift,
            &mut gate_change,
        );
    }

    verdict(&violations, &parity_drift, gate_change);
}

fn report_target(
    target: &Target,
    args: &Args,
    violations: &mut Vec<String>,
    parity_drift: &mut Vec<String>,
    gate_change: &mut bool,
) {
    println!("## [{}] {}", target.class.name(), target.url);
    for (label, recipe) in recipes(target.class) {
        let base = aggregate(&args.baseline, target.url, recipe, args.runs);
        let cand = aggregate(&args.candidate, target.url, recipe, args.runs);
        print_row(label, &base, &cand);
        check_contract(target, label, &base, "baseline", violations);
        check_contract(target, label, &cand, "candidate", violations);
        check_challenge(target, label, &base, "baseline", violations);
        check_challenge(target, label, &cand, "candidate", violations);
        // Parity is an IDENTITY claim, so check it only on the deterministic
        // static recipe (a page's own `--js` is nondeterministic — React
        // hydration, `Date`, dynamic content — and not frot's identity), and
        // only when each binary is internally stable across its runs.
        if target.class == Class::Control && !recipe.contains(&"--js") && base.stable && cand.stable
        {
            check_parity(target, label, &base, &cand, parity_drift);
        }
        if gate_cleared(target, &base, &cand) {
            *gate_change = true;
        }
    }
    println!();
}

/// A gate is "cleared" when the candidate turns a non-`ok` baseline outcome into
/// an `ok` one on a soft-gate site — the only publishable positive result.
fn gate_cleared(target: &Target, base: &Agg, cand: &Agg) -> bool {
    target.class == Class::SoftGate && base.probe.status != "ok" && cand.probe.status == "ok"
}

fn print_row(label: &str, base: &Agg, cand: &Agg) {
    println!("- recipe `{label}`");
    print_side("baseline", base);
    print_side("candidate", cand);
}

fn print_side(which: &str, a: &Agg) {
    let p = &a.probe;
    let stable = if a.stable { "" } else { " UNSTABLE" };
    let wire = if p.peetprint.is_empty() && p.ja4.is_empty() {
        String::new()
    } else {
        format!(" ja4={} proto={} peet={}", p.ja4, p.protocol, p.peetprint)
    };
    let decl = if p.decl.is_empty() {
        String::new()
    } else {
        format!(" decl[{}]", p.decl)
    };
    println!(
        "    {which:>9}: exit={:?} status={} needs={} err={} http={:?} bytes={} elems={} text={} sha={} exec={} p50={}ms p95={}ms{decl}{wire}{stable}",
        p.exit, p.status, p.needs, p.error_kind, p.http_status,
        p.bytes, p.elements, p.text_chars, short(&p.sha), p.executed, a.p50, a.p95,
    );
}

fn short(sha: &str) -> &str {
    if sha.len() >= 12 {
        &sha[..12]
    } else {
        sha
    }
}

fn check_contract(target: &Target, label: &str, a: &Agg, which: &str, out: &mut Vec<String>) {
    if !a.probe.exit_contract_ok {
        out.push(format!(
            "exit-contract: [{}] {} {which} recipe `{label}` status={} exit={:?}",
            target.class.name(),
            target.url,
            a.probe.status,
            a.probe.exit
        ));
    }
}

/// §3.7: a declared-challenge negative control must make exactly one request and
/// never be executed. The live view of that: `needs:["human"]` or a plain 4xx,
/// and NO `js` block (nothing ran). One request cannot be counted black-box; the
/// deterministic `src/run/persona_tests.rs` proves the exact count offline.
fn check_challenge(target: &Target, label: &str, a: &Agg, which: &str, out: &mut Vec<String>) {
    if target.class != Class::Challenge {
        return;
    }
    let p = &a.probe;
    let refused = p.needs == "human" || p.status == "error";
    if !refused || p.executed {
        out.push(format!(
            "challenge-control: [{}] {} {which} recipe `{label}` refused={refused} executed={} — must refuse pre-execution",
            target.class.name(), target.url, p.executed
        ));
    }
}

/// Controls must show an identical IDENTITY impression across binaries. Diffed on
/// identity-relevant fields only — NOT the whole envelope — so bl-acec's approved
/// additive `http.headers` schema change is not mistaken for an identity
/// regression (it lives in `decl`, deliberately excluded here).
fn check_parity(target: &Target, label: &str, base: &Agg, cand: &Agg, out: &mut Vec<String>) {
    let (b, c) = (&base.probe, &cand.probe);
    let same = b.status == c.status
        && b.needs == c.needs
        && b.error_kind == c.error_kind
        && b.http_status == c.http_status
        && b.sha == c.sha
        && b.elements == c.elements
        && b.text_chars == c.text_chars;
    if !same {
        out.push(format!(
            "parity-drift: {} recipe `{label}` control impression differs (sha {} vs {})",
            target.url,
            short(&b.sha),
            short(&c.sha)
        ));
    }
}

fn verdict(violations: &[String], parity_drift: &[String], gate_change: bool) {
    println!("## verdict");
    if violations.is_empty() {
        println!("- exit contract & challenge controls: OK");
    } else {
        println!("- CONTRACT VIOLATIONS ({}):", violations.len());
        violations.iter().for_each(|v| println!("    - {v}"));
    }
    if parity_drift.is_empty() {
        println!("- control identity parity: OK (identity impression identical across binaries)");
    } else {
        println!("- CONTROL PARITY DRIFT ({}):", parity_drift.len());
        parity_drift.iter().for_each(|v| println!("    - {v}"));
    }
    if gate_change {
        println!(
            "- RESULT: candidate cleared a soft-gate the baseline did not (measured positive)."
        );
    } else {
        println!("- RESULT: NULL — candidate cleared no additional gate. This is a publishable");
        println!("  result for identity.md §14; do NOT respond by adding a bypass.");
    }
}
