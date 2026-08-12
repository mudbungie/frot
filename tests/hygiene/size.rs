//! No tracked source file is over the line cap (`bl-a68b`).
//!
//! The cap is a hard rule of the checkout, so it is checked the same way from
//! everywhere: `git ls-files '*.rs'` piped into `scripts/line-limit.sh`, which
//! owns the number. CI runs exactly that sweep, and since bl-a68b so does the
//! pre-commit hook — before, the hook saw only the *staged* files, so a file
//! could go over in one commit and then ride along untouched (and unchecked)
//! in every commit after it while CI stayed red. This gate is the same sweep
//! again at `cargo test` speed, where the failure is cheapest to read.

use super::repo;
use std::fs;
use std::io::Write;
use std::process::{Command, Output, Stdio};

const LIMITER: &str = "scripts/line-limit.sh";

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

/// A scratch file of `lines` newline-terminated lines, named for the caller.
fn scratch(name: &str, lines: usize) -> String {
    let path = std::env::temp_dir().join(format!("frot-line-limit-{}-{name}", std::process::id()));
    fs::write(&path, "x\n".repeat(lines)).expect("write scratch file");
    path.to_string_lossy().into_owned()
}

#[test]
fn no_tracked_source_file_is_over_the_cap() {
    let listed = Command::new("git")
        .args(["ls-files", "*.rs"])
        .current_dir(repo(""))
        .output()
        .expect("run git ls-files");
    assert!(listed.status.success(), "git ls-files failed");
    let paths = String::from_utf8(listed.stdout).expect("paths are utf8");
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
