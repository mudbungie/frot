//! Repo-hygiene gates — invariants of the *checkout*, not of any run of the
//! binary.
//!
//! Third-party fixture content carried with no licence or attribution blocked
//! publication once and was fixed by hand (`bl-11f5`, [`licensing`]). A hand
//! fix does not hold — the next golden ball drops a vendored bundle into
//! `tests/fixtures/js/`.
//!
//! The other leak that blocked publication — a measurement's egress IP and ASN
//! in the design docs (`bl-f521`) — was gated here until `bl-59dc` moved it to
//! `scripts/leak-scan.sh` (`make leak-scan`), the one home for "what may not be
//! published": the same table also judges the packaged crate and the task
//! store, which no test in this crate can reach.
//!
//! The same shape covers the checkout's own budgets ([`size`]) and its
//! formatting ([`fmt`]): both were gated off a commit's *staged* files, and
//! `bl close` gates a worktree whose work is already committed, where nothing
//! is staged — so a file went over the 300-line cap, or landed unformatted,
//! and stayed that way until CI swept the tree.
//!
//! And it covers what the checkout goes stale *against* ([`persona`]): the
//! pinned browser profile expires on the ESR calendar, so the gate is the wall
//! clock rather than an edit, and nothing but a test run can notice it.
//!
//! Every gate here fails on the **class** of defect, never on the instance.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod fmt;
mod licensing;
mod persona;
mod size;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn read(rel: &str) -> String {
    fs::read_to_string(repo(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

/// File names directly in `rel`, sorted. Panics if the directory is missing —
/// a fixture directory that vanished is a failure, not an empty pass.
fn files_in(rel: &str) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(repo(rel))
        .unwrap_or_else(|e| panic!("read_dir {rel}: {e}"))
        .map(|e| {
            e.expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
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
