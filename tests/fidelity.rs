//! The fidelity oracle (`docs/design/fidelity.md`): frot's views against a
//! real browser's answers over the same bytes.
//!
//! Chrome is not here. `scripts/fidelity-record` drove one over every page in
//! `tests/fixtures/fidelity/` and checked its raw answers in beside the page;
//! this suite runs the compiled `frot` over the same files and compares,
//! offline — no Chrome, no network (F5). Each channel projects both sides
//! onto frot's declared model (`fidelity/project.rs`, `fidelity/ax.rs`) and on
//! the projection the predicate is equality (F3): no tolerance, no score.
//! What differs anyway is a delta, and every delta is a row in
//! `fidelity/ledger.rs` — a reasoned residual or a filed defect — asserted in
//! both directions (F4).
//!
//! Nothing here skips. A page with no capture, a capture with no page, a
//! capture whose stamp is incomplete or whose bytes are not the page's: each
//! is a failure, because an oracle that can pass vacuously is not one.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "fidelity/ax.rs"]
mod ax;
#[path = "fidelity/diff.rs"]
mod diff;
#[path = "fidelity/ledger.rs"]
mod ledger;
#[path = "fidelity/project.rs"]
mod project;
#[path = "../examples/ab_harness/sha256.rs"]
mod sha256;

use ledger::{Why, LEDGER};

const CHANNELS: &[&str] = &["text", "ax", "links", "forms", "bboxes"];

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fidelity")
}

/// The corpus files with extension `ext`, as paths below the corpus with the
/// extension dropped, sorted: the authored pages at the top, the saved real
/// pages in `saved/` (`fidelity.md` §4).
fn stems(ext: &str) -> Vec<String> {
    let mut names = Vec::new();
    for dir in ["", "saved/"] {
        for entry in fs::read_dir(corpus().join(dir)).expect("a fidelity corpus directory") {
            let path = entry.expect("dir entry").path();
            if path.extension().is_some_and(|e| e == ext) {
                let stem = path.file_stem().expect("stem").to_string_lossy();
                names.push(format!("{dir}{stem}"));
            }
        }
    }
    names.sort();
    names
}

/// Every page in the corpus. Never empty: no pages is a failure, not a pass.
fn fixtures() -> Vec<String> {
    let pages = stems("html");
    assert!(!pages.is_empty(), "the fidelity corpus has no pages");
    pages
}

/// The capture recorded for `fixture`. Missing or unreadable fails (F5).
fn capture(fixture: &str) -> Value {
    let path = corpus().join(format!("{fixture}.json"));
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("{fixture}.html has no capture ({e}); run `make fidelity-record`")
    });
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("{fixture}.json is not a capture: {e}"))
}

/// frot's `view` of `fixture` under the capture's recipe: the envelope, which
/// must be `ok` — the harness reads frot's public surface as any caller does.
fn frot(fixture: &str, recipe: &Value, view: &str) -> Value {
    let page = format!(
        "file://{}",
        corpus().join(format!("{fixture}.html")).display()
    );
    let recipe = recipe
        .as_array()
        .expect("stamp.recipe")
        .iter()
        .map(|f| f.as_str().expect("flag"));
    let out = Command::new(env!("CARGO_BIN_EXE_frot"))
        .arg(&page)
        .args(recipe)
        .args(["--out", view])
        .output()
        .expect("spawn frot");
    let envelope: Value = serde_json::from_slice(&out.stdout).expect("an envelope");
    assert_eq!(
        envelope["status"], "ok",
        "{fixture} --out {view}: {envelope}"
    );
    envelope
}

/// Both sides of `channel` for one page, each projected onto frot's model.
fn sides(fixture: &str, channel: &str) -> (Vec<String>, Vec<String>) {
    let cap = capture(fixture);
    let stamp = &cap["stamp"];
    let envelope = frot(fixture, &stamp["recipe"], channel);
    let out = &envelope["out"];
    let ours = envelope["url"]["final"].as_str().expect("url.final");
    let theirs = format!(
        "{}{fixture}.html",
        stamp["base"].as_str().expect("stamp.base")
    );
    match channel {
        "text" => (project::tokens(&cap["text"]), project::tokens(out)),
        "ax" => (ax::chrome(&cap), ax::frot(out)),
        "links" => (
            project::chrome_links(&cap["links"], &theirs),
            project::frot_links(out, ours),
        ),
        "forms" => (
            project::chrome_forms(&cap["forms"], &theirs),
            project::frot_forms(out, ours),
        ),
        "bboxes" => (
            project::chrome_boxes(&cap["boxes"]),
            project::frot_boxes(out),
        ),
        other => panic!("no channel {other}"),
    }
}

/// The gate for one channel: over every page, the deltas are exactly the
/// ledger's rows for that page and channel.
fn holds(channel: &str) {
    let mut failures = Vec::new();
    for fixture in fixtures() {
        let (chrome, frot) = sides(&fixture, channel);
        let declared: Vec<&str> = LEDGER
            .iter()
            .filter(|r| r.fixture == fixture && r.channel == channel)
            .map(|r| r.delta)
            .collect();
        for complaint in diff::undisposed(&diff::delta(&chrome, &frot), &declared) {
            failures.push(format!("{fixture} [{channel}] {complaint}"));
        }
    }
    assert!(
        failures.is_empty(),
        "frot and Chrome-as-recorded differ outside the ledger \
         (`- x`: Chrome has x, frot lacks it; `+ x`: the reverse).\n\
         Each line is a defect to file or a residual to declare in \
         tests/fidelity/ledger.rs — never a tolerance:\n{}",
        failures.join("\n")
    );
}

#[test]
fn text_matches_chrome() {
    holds("text");
}

#[test]
fn ax_matches_chrome() {
    holds("ax");
}

#[test]
fn links_match_chrome() {
    holds("links");
}

#[test]
fn forms_match_chrome() {
    holds("forms");
}

#[test]
fn bboxes_paintedness_matches_chrome() {
    holds("bboxes");
}

/// F7 and F1: a capture names its oracle, and was recorded from the bytes that
/// are in the tree now. A page edited without a re-record fails here.
#[test]
fn every_capture_is_stamped_and_read_the_same_bytes() {
    assert_eq!(
        stems("json"),
        fixtures(),
        "pages and captures must pair off one to one"
    );
    for fixture in fixtures() {
        let cap = capture(&fixture);
        let stamp = &cap["stamp"];
        let text = |key: &str| {
            stamp[key]
                .as_str()
                .unwrap_or_else(|| panic!("{fixture}: no stamp.{key}"))
        };
        let version: Vec<&str> = text("chrome").split('.').collect();
        assert!(
            version.len() == 4 && version.iter().all(|p| p.parse::<u32>().is_ok()),
            "{fixture}: stamp.chrome is not a Chrome version"
        );
        let date: Vec<&str> = text("date").split('-').collect();
        assert!(
            date.len() == 3 && date.iter().all(|p| p.parse::<u32>().is_ok()),
            "{fixture}: stamp.date is not a date"
        );
        assert!(
            text("base").ends_with('/'),
            "{fixture}: stamp.base is not a directory"
        );
        let flags = stamp["flags"].as_array().expect("stamp.flags");
        assert!(
            flags.iter().any(|f| f == "--headless=new"),
            "{fixture}: stamp.flags"
        );
        // frot lays out at a fixed 1280×720 (`README.md`); a capture at any
        // other viewport answered different `@media` queries.
        assert_eq!(
            stamp["viewport"],
            serde_json::json!([1280, 720]),
            "{fixture}: viewport"
        );
        let recipe = &stamp["recipe"];
        assert!(
            *recipe == serde_json::json!(["--css"])
                || *recipe == serde_json::json!(["--css", "--js"]),
            "{fixture}: stamp.recipe is not a browser-comparable recipe (§2.1)"
        );
        let bytes = stamp["bytes"].as_object().expect("stamp.bytes");
        assert!(
            bytes.contains_key(&format!("{fixture}.html")),
            "{fixture}: page not in stamp.bytes"
        );
        for (file, recorded) in bytes {
            let now = fs::read(corpus().join(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
            assert_eq!(
                recorded.as_str(),
                Some(sha256::hex(&now).as_str()),
                "{file} changed since {fixture}.json was recorded; run `make fidelity-record`"
            );
        }
    }
}

/// The ledger is code and is held to it: a row must name a real page and
/// channel (or it asserts nothing), and must say why.
#[test]
fn every_ledger_row_is_addressed_and_reasoned() {
    let pages = fixtures();
    for row in LEDGER {
        let at = format!(
            "ledger row {} [{}] {:?}",
            row.fixture, row.channel, row.delta
        );
        assert!(pages.iter().any(|p| p == row.fixture), "{at}: no such page");
        assert!(CHANNELS.contains(&row.channel), "{at}: no such channel");
        assert!(
            row.delta.starts_with("- ") || row.delta.starts_with("+ "),
            "{at}: not a delta"
        );
        match row.why {
            Why::Residual(reason) => {
                assert!(reason.len() > 20, "{at}: a residual needs its reason")
            }
            Why::Defect(ball) => assert!(ball.starts_with("bl-"), "{at}: a defect names its ball"),
        }
    }
}
