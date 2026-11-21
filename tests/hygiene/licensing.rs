//! Third-party licensing is complete (`bl-11f5`, `bl-41e5`).
//!
//! `tests/fixtures/` is excluded from the published crate but ships with the
//! repo, so every third-party byte in it is redistributed under someone else's
//! licence. `NOTICE.md` is the map that makes that lawful; these gates keep the
//! map total — no fixture without a row, no row without a licence, no licence
//! without a fixture — and keep third-party bytes from living anywhere else.
//!
//! The second half is `bl-41e5`: the sweep was one directory
//! (`tests/fixtures/js`), and a React bundle vendored under `src/` sat outside
//! it for months with no row and no licence text, and went out in the
//! published crate. So the sweep is now the tracked tree: every file under
//! `tests/fixtures/` needs a row, and no file outside it may carry someone
//! else's licence banner.

use super::{files_in, read, repo, tracked};
use std::fs;

const NOTICE: &str = "tests/fixtures/NOTICE.md";

/// The file names `text` references after `prefix`, e.g. `licenses/x.LICENSE`.
/// A bare mention of the directory names no file and yields nothing.
fn referenced_after(text: &str, prefix: &str) -> Vec<String> {
    text.match_indices(prefix)
        .map(|(i, _)| {
            text[i + prefix.len()..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || "._-".contains(*c))
                .collect::<String>()
        })
        .filter(|name: &String| !name.is_empty())
        .collect()
}

const FIXTURES: &str = "tests/fixtures/";

/// Whether `notice` has a row for the fixture at `rel` (its path below
/// `tests/fixtures/`): the path itself, or `dir/*.ext` for its own directory —
/// the form a row takes for a family this project authored.
fn has_row(notice: &str, rel: &str) -> bool {
    let (dir, name) = rel.rsplit_once('/').unwrap_or(("", rel));
    let family = name
        .rsplit_once('.')
        .map(|(_, ext)| format!("`{dir}/*.{ext}`"));
    notice.contains(&format!("`{rel}`")) || family.is_some_and(|f| notice.contains(&f))
}

#[test]
fn every_fixture_is_named_in_the_notice() {
    let notice = read(NOTICE);
    let all = tracked(FIXTURES);
    assert!(
        all.iter().any(|p| p == "tests/fixtures/js/jquery.min.js"),
        "the sweep of {FIXTURES} listed nothing"
    );
    let unlicensed: Vec<&str> = all
        .iter()
        .filter(|path| *path != NOTICE)
        .map(|path| &path[FIXTURES.len()..])
        .filter(|rel| !has_row(&notice, rel))
        .collect();
    assert!(
        unlicensed.is_empty(),
        "fixture(s) with no row in {NOTICE}: {unlicensed:?}\n\
         A file under tests/fixtures/ is redistributed with the repo, and \
         unless this project wrote it, under someone else's licence — so a \
         file with no row is a compliance bug: add the row (upstream, licence, \
         source, revision, modifications) and vendor the text into \
         tests/fixtures/licenses/. If the test needs only a *shape*, write a \
         synthetic fixture in tests/fixtures/needs/ instead."
    );
}

#[test]
fn a_row_covers_only_the_file_or_family_it_names() {
    let notice = "| `js/a.min.js` | x |\n| `fidelity/*.html` | y |";
    assert!(has_row(notice, "js/a.min.js"));
    assert!(has_row(notice, "fidelity/page.html"));
    assert!(!has_row(notice, "js/b.min.js"), "an unnamed file");
    assert!(!has_row(notice, "js/a.min.js.map"), "a longer name");
    assert!(!has_row(notice, "fidelity/page.css"), "another extension");
    assert!(
        !has_row(notice, "fidelity/saved/page.html"),
        "a family row does not reach a subdirectory"
    );
    assert!(!has_row(notice, "LICENSE"), "a bare, unnamed file");
}

/// The marks a vendored distribution carries: a minifier-preserved licence
/// tag, an SPDX line, or a copyright notice. Spelled in pieces so that this
/// file, which must stay outside the fixtures, does not carry one itself.
fn banner(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    let marks = [
        ["@", "license"],
        ["@", "preserve"],
        ["spdx-license", "-identifier"],
        ["copyright", " (c)"],
        ["copyright", " ©"],
    ];
    marks.iter().any(|m| text.contains(&m.concat()))
}

/// This project's own licence texts: the only banners that belong outside the
/// fixtures.
const OURS: &[&str] = &["LICENSE-MIT", "LICENSE-APACHE"];

#[test]
fn no_third_party_banner_lives_outside_the_fixtures() {
    let all = tracked(".");
    assert!(
        all.iter().any(|p| p == "src/lib.rs"),
        "the sweep listed nothing"
    );
    let strays: Vec<&String> = all
        .iter()
        .filter(|path| !path.starts_with(FIXTURES) && !OURS.contains(&path.as_str()))
        .filter(|path| {
            let bytes = fs::read(repo(path)).unwrap_or_else(|e| panic!("read {path}: {e}"));
            banner(&String::from_utf8_lossy(&bytes))
        })
        .collect();
    assert!(
        strays.is_empty(),
        "file(s) outside {FIXTURES} carrying a licence banner: {strays:?}\n\
         That is someone else's work, vendored where {NOTICE} does not reach \
         — and under src/ it ships in the published crate. Move it into \
         {FIXTURES} with a row and its licence text; a test can still \
         `include_str!` it from there (bl-41e5)."
    );
}

#[test]
fn a_vendored_banner_is_recognized() {
    let react = ["/** @", "license React\n * Copy", "right (c) Facebook */"].concat();
    assert!(banner(&react));
    assert!(banner(&["// SPDX-License", "-Identifier: MIT"].concat()));
    assert!(!banner("the copyright holder's address; licensed code"));
}

#[test]
fn every_vendored_licence_is_referenced_and_every_reference_is_vendored() {
    let notice = read(NOTICE);
    let vendored = files_in("tests/fixtures/licenses");
    let orphaned: Vec<&String> = vendored.iter().filter(|n| !notice.contains(*n)).collect();
    assert!(
        orphaned.is_empty(),
        "vendored licence(s) no row in {NOTICE} points at: {orphaned:?} — \
         either the fixture they covered is gone (delete them) or a row lost \
         its reference."
    );
    let dangling: Vec<String> = referenced_after(&notice, "licenses/")
        .into_iter()
        .filter(|n| !vendored.contains(n))
        .collect();
    assert!(
        dangling.is_empty(),
        "{NOTICE} points at licence file(s) that do not exist: {dangling:?} — \
         a licence named but not carried is the same defect as no licence."
    );
}

#[test]
fn no_licence_sidecar_reference_dangles() {
    // The exact defect this pins: `react-todomvc.bundle.js` opens with
    // `/*! For license information please see app.bundle.js.LICENSE.txt */` —
    // webpack's pointer at the notices it stripped out of the code — and that
    // file was never vendored, so the bundle's own attribution was
    // unreadable. Convention (NOTICE.md §2.1): the sidecar rides beside the
    // fixture under the fixture's own name, since the fixture is renamed.
    for name in files_in("tests/fixtures/js") {
        if !name.ends_with(".js") {
            continue;
        }
        let head: String = read(&format!("tests/fixtures/js/{name}"))
            .chars()
            .take(200)
            .collect();
        if !head.contains(".LICENSE.txt") {
            continue;
        }
        let sidecar = format!("tests/fixtures/js/{name}.LICENSE.txt");
        assert!(
            repo(&sidecar).is_file(),
            "{name} points at a licence sidecar that is not vendored; \
             fetch it from the deployment and save it as {sidecar}"
        );
    }
}

#[test]
fn every_needs_fixture_has_a_row_in_its_readme() {
    let readme = read("tests/fixtures/needs/README.md");
    let undocumented: Vec<String> = files_in("tests/fixtures/needs")
        .into_iter()
        .filter(|name| name != "README.md" && !readme.contains(name))
        .collect();
    assert!(
        undocumented.is_empty(),
        "needs fixture(s) with no row in tests/fixtures/needs/README.md: \
         {undocumented:?}\n\
         That table is where a fixture declares the real page whose *shape* it \
         models; the directory's rule is that the bytes are synthetic. An \
         undeclared file is how verbatim third-party markup gets back in."
    );
}
