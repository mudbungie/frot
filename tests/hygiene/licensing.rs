//! Fixture licensing is complete (`bl-11f5`).
//!
//! `tests/fixtures/` is excluded from the published crate but ships with the
//! repo, so every third-party byte in it is redistributed under someone else's
//! licence. `NOTICE.md` is the map that makes that lawful; these gates keep the
//! map total — no fixture without a row, no row without a licence, no licence
//! without a fixture.

use super::{files_in, read, repo};

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

#[test]
fn every_third_party_fixture_is_named_in_the_notice() {
    let notice = read(NOTICE);
    let unlicensed: Vec<String> = files_in("tests/fixtures/js")
        .into_iter()
        .filter(|name| !notice.contains(name))
        .collect();
    assert!(
        unlicensed.is_empty(),
        "fixture(s) with no row in {NOTICE}: {unlicensed:?}\n\
         Everything under tests/fixtures/js/ is someone else's work, \
         redistributed with the repo under its own licence, so a file with no \
         row is a compliance bug: add the row (upstream, licence, source, \
         revision, modifications) and vendor the text into \
         tests/fixtures/licenses/. If the test needs only a *shape*, write a \
         synthetic fixture in tests/fixtures/needs/ instead."
    );
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
