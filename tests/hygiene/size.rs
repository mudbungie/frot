//! No tracked source file is over the line cap, in any language (`bl-a68b`,
//! `bl-6da7`).
//!
//! The cap is a hard rule of the checkout, so it is checked the same way from
//! everywhere: `scripts/source-files.sh` piped into `scripts/line-limit.sh`,
//! which is exactly `make size` — what CI and the pre-commit hook both run.
//! Two facts, one home each: which files are source, and how long one may be.
//!
//! Both halves have gone wrong the same way, and neither failure could be seen
//! from inside the gate. bl-a68b: the hook swept only the *staged* files, so a
//! file could go over in one commit and ride along unchecked in every commit
//! after while CI stayed red. bl-6da7: both callers spelled out `git ls-files
//! '*.rs'`, so the 21-file JS prelude — the entire DOM/JS shim, unambiguously
//! source — was never examined, and `elem2.js` sat at 313 lines for some time.
//!
//! **A gate that cannot fail is indistinguishable from a gate that passes.** So
//! this file checks three separable things: the tree is clean, the sweep really
//! reaches every source language (a spot check would pass on a sweep that had
//! quietly narrowed again), and the limiter really rejects an oversized file.

use super::repo;
use std::fs;
use std::io::Write;
use std::process::{Command, Output, Stdio};

const LIMITER: &str = "scripts/line-limit.sh";
const SWEEP: &str = "scripts/source-files.sh";

/// Feed newline-delimited `paths` to the limiter and collect its verdict.
fn limit(paths: &str) -> Output {
    let mut child = Command::new(repo(LIMITER))
        .current_dir(repo(""))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {LIMITER}: {e}"));
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(paths.as_bytes())
        .expect("write paths");
    child.wait_with_output().expect("limiter output")
}

/// The swept set, exactly as the hook and CI compute it.
fn swept() -> String {
    let listed = Command::new(repo(SWEEP))
        .current_dir(repo(""))
        .output()
        .unwrap_or_else(|e| panic!("spawn {SWEEP}: {e}"));
    assert!(listed.status.success(), "{SWEEP} failed");
    String::from_utf8(listed.stdout).expect("paths are utf8")
}

/// Tracked files matching `pathspec`, as `git ls-files` lists them.
fn tracked(pathspec: &str) -> Vec<String> {
    let listed = Command::new("git")
        .args(["ls-files", pathspec])
        .current_dir(repo(""))
        .output()
        .expect("run git ls-files");
    assert!(listed.status.success(), "git ls-files failed");
    String::from_utf8(listed.stdout)
        .expect("paths are utf8")
        .lines()
        .map(str::to_owned)
        .collect()
}

/// A scratch file of `lines` newline-terminated lines, named for the caller.
fn scratch(name: &str, lines: usize) -> String {
    let path = std::env::temp_dir().join(format!("frot-line-limit-{}-{name}", std::process::id()));
    fs::write(&path, "x\n".repeat(lines)).expect("write scratch file");
    path.to_string_lossy().into_owned()
}

#[test]
fn no_tracked_source_file_is_over_the_cap() {
    let paths = swept();
    assert!(paths.contains("src/run.rs"), "sweep listed nothing");
    let verdict = limit(&paths);
    assert!(
        verdict.status.success(),
        "{}\nSplit the file on a subject seam — the budget is not negotiable \
         (AGENTS.md, \"Source files <= 300 lines\").",
        String::from_utf8_lossy(&verdict.stderr)
    );
}

#[test]
fn the_sweep_reaches_every_source_language_and_stops_at_the_fixtures() {
    let paths = swept();
    let swept_lines: Vec<&str> = paths.lines().collect();

    // Totally, not by spot check: every tracked prelude module is in the set.
    // The prelude is what bl-6da7 found unexamined, and one named file would
    // still pass a sweep that had narrowed to just that file.
    let missing: Vec<String> = tracked("src/js/prelude/*.js")
        .into_iter()
        .filter(|p| !swept_lines.contains(&p.as_str()))
        .collect();
    assert!(
        !swept_lines.is_empty() && missing.is_empty(),
        "prelude module(s) the line cap never sees: {missing:?} — the JS \
         prelude is source (AGENTS.md exempts only docs and config)."
    );
    for language in ["src/run.rs", "src/js/prelude/dom.js", SWEEP] {
        assert!(
            swept_lines.contains(&language),
            "{language} is source and the sweep does not list it"
        );
    }

    // And it stops at the data. tests/fixtures/ is third-party or synthetic
    // page bytes nobody here may edit (tests/fixtures/NOTICE.md governs it);
    // a cap that fired on a vendored React distribution would get switched off.
    let data: Vec<&&str> = swept_lines
        .iter()
        .filter(|p| p.starts_with("tests/fixtures/"))
        .collect();
    assert!(data.is_empty(), "fixture data swept as source: {data:?}");
    assert!(
        !tracked("tests/fixtures/js/*.js").is_empty(),
        "no fixture JS tracked — the exclusion above proves nothing"
    );
}

#[test]
fn the_limiter_rejects_an_oversized_file_and_passes_a_small_one() {
    // Negative control: without it, a limiter that silently passed everything
    // (an unreadable path, a broken `wc` pipeline) would look like a clean tree.
    let big = scratch("over", 10_000);
    let rejected = limit(&format!("{big}\n"));
    assert!(!rejected.status.success(), "10k lines must not pass");
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("line-limit:"));

    let small = scratch("under", 1);
    assert!(limit(&format!("{small}\n")).status.success());

    for path in [big, small] {
        fs::remove_file(path).expect("remove scratch file");
    }
}
