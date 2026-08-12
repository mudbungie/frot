//! The tracked tree is formatted, and the formatter's sweep reaches all of it
//! (`bl-0066`).
//!
//! `.githooks/pre-commit` runs `cargo fmt --check`, but it used to run it only
//! when `git diff --cached` listed a `.rs` file. `bl close` runs the hook on a
//! worktree whose work is already **committed**, so the index equals HEAD and
//! that list is empty: the fmt gate — and clippy, POSIX and coverage with it —
//! skipped on every close, and unformatted code reached main whenever the
//! author had not run `cargo fmt` by hand. Four commits landed that way on
//! 2026-08-11 (`f5be7af`, `66666b5`, `b16cc46`, `75b2d42`), each dirtying a
//! file its own diff had edited, and each blocking every other agent's close
//! until an unrelated ball happened to carry the reflow.
//!
//! So this gate runs the same check at `cargo test` speed, where the failure is
//! cheapest to read, and it checks both halves of "the landed tree is clean":
//! that `cargo fmt --check` passes, and that its sweep covers every tracked
//! `.rs` file — `cargo fmt` reaches sources through `mod` declarations, so a
//! tracked file no module names would be exempt rather than clean.

use super::repo;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

/// Run `cargo fmt` in `dir` with `args`.
fn cargo_fmt(dir: &Path, args: &[&str]) -> Output {
    Command::new("cargo")
        .arg("fmt")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("run cargo fmt: {e}"))
}

/// Sorted `git ls-files '*.rs'` for the checkout.
fn tracked_sources() -> Vec<String> {
    let listed = Command::new("git")
        .args(["ls-files", "*.rs"])
        .current_dir(repo(""))
        .output()
        .expect("run git ls-files");
    assert!(listed.status.success(), "git ls-files failed");
    let mut paths: Vec<String> = String::from_utf8(listed.stdout)
        .expect("paths are utf8")
        .lines()
        .map(str::to_owned)
        .collect();
    paths.sort();
    assert!(
        paths.contains(&"src/run.rs".to_owned()),
        "sweep listed nothing"
    );
    paths
}

/// The files `cargo fmt` actually visits, repo-relative and sorted. `--emit
/// stdout` prefixes each file it formats with `<absolute path>:`, which is the
/// only place rustfmt names its own reach.
fn swept_sources() -> Vec<String> {
    let emitted = cargo_fmt(&repo(""), &["--", "--emit", "stdout"]);
    assert!(emitted.status.success(), "cargo fmt --emit stdout failed");
    let root = format!("{}/", env!("CARGO_MANIFEST_DIR"));
    let mut paths: Vec<String> = String::from_utf8_lossy(&emitted.stdout)
        .lines()
        .filter_map(|l| l.strip_suffix(':')?.strip_prefix(&root).map(str::to_owned))
        .filter(|p| p.ends_with(".rs"))
        .collect();
    paths.sort();
    paths.dedup();
    paths
}

#[test]
fn the_tracked_tree_is_formatted() {
    let verdict = cargo_fmt(&repo(""), &["--check"]);
    assert!(
        verdict.status.success(),
        "{}\nRun 'cargo fmt' — the tree that lands must be the tree rustfmt \
         would write (bl-0066).",
        String::from_utf8_lossy(&verdict.stdout)
    );
}

#[test]
fn the_formatter_sweep_reaches_every_tracked_source_file() {
    let swept = swept_sources();
    let unswept: Vec<String> = tracked_sources()
        .into_iter()
        .filter(|p| !swept.contains(p))
        .collect();
    assert!(
        unswept.is_empty(),
        "tracked but never formatted (no `mod` declaration reaches them): {unswept:?}"
    );
}

#[test]
fn the_fmt_check_rejects_an_unformatted_crate() {
    // Negative control: without it, a check that passed everything — a rustfmt
    // that silently skipped its inputs, a `--check` that stopped meaning
    // "fail on a diff" — would read exactly like a clean tree.
    let dir = std::env::temp_dir().join(format!("frot-fmt-control-{}", std::process::id()));
    fs::create_dir_all(dir.join("src")).expect("create control crate");
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"fmt-control\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )
    .expect("write control manifest");
    // The repo's own pin, copied in, so the control exercises the same rustfmt
    // the hook and CI run rather than whatever the machine defaults to.
    fs::write(
        dir.join("rust-toolchain.toml"),
        super::read("rust-toolchain.toml"),
    )
    .expect("write control pin");
    fs::write(dir.join("src/lib.rs"), "pub fn  f( ) ->u8{ 1 }\n").expect("write control source");

    let rejected = cargo_fmt(&dir, &["--check"]);
    assert!(
        !rejected.status.success(),
        "an unformatted crate must not pass `cargo fmt --check`"
    );

    fs::remove_dir_all(&dir).expect("remove control crate");
}
