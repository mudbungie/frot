//! Repo-hygiene gates — invariants of the *checkout*, not of any run of the
//! binary.
//!
//! Two leaks blocked publication once and were fixed by hand: a measurement's
//! egress IP in the design docs (`bl-f521`, [`identity`]), and third-party
//! fixture content carried with no licence or attribution (`bl-11f5`,
//! [`licensing`]). A hand fix does not hold — the next field trial pastes a
//! fresh address into a doc, the next golden ball drops a vendored bundle into
//! `tests/fixtures/js/`.
//!
//! The same shape covers the checkout's own budgets ([`size`]): the 300-line
//! source cap was enforced only over a commit's *staged* files, so a file that
//! went over stayed over — unstaged and unseen — until CI swept the tree.
//!
//! Every gate here fails on the **class** of leak, never on the instance: the
//! scrubbed address is deliberately not written down in this crate, since a
//! test that pinned the literal would re-introduce what it guards against.

use std::fs;
use std::path::{Path, PathBuf};

mod identity;
mod licensing;
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
