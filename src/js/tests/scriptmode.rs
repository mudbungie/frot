//! Script-mode golden (bl-0679): the js.md §4.1 language-mode rule — a classic
//! script is sloppy unless its own source opts in, an ES module is strict —
//! turned into an EXECUTABLE contract. One fixture page
//! (`tests/fixtures/js/script-mode.html`) probes every mode-sensitive fact in JS
//! and self-checks it, recording regressions into `data-fail`; this gate reads
//! that array (empty == all pass) plus the counts only the host can see.
//!
//! The bug it pins was found live on svelte.dev and hn.svelte.dev: rquickjs's
//! `EvalOptions::default` forces `strict`, so the untyped inline bootstrap
//! SvelteKit emits — an undeclared assignment to a generated `__sveltekit_*`
//! global — threw `ReferenceError` and the page rendered nothing. The fixture's
//! *first* statement is that same undeclared assignment, so a regression takes
//! the whole probe dark rather than reporting a tidy failure.

use super::drive;
use crate::dom::{Document, NodeKind};

const SCRIPT_MODE: &str = include_str!("../../../tests/fixtures/js/script-mode.html");

fn data_fail(doc: &Document) -> Option<String> {
    let de = doc.find_by_tag("html")[0];
    let NodeKind::Element(el) = &doc.node(de).kind else {
        unreachable!("html is an element")
    };
    el.attr("data-fail").map(str::to_string)
}

#[test]
fn script_mode_golden_pins_sloppy_classics_and_strict_modules() {
    let (doc, report) = drive(SCRIPT_MODE);
    let fails = data_fail(&doc).expect(
        "probe wrote no data-fail: a script went dark before the last one ran — \
         a classic script forced back into strict mode fails exactly this way",
    );
    assert_eq!(fails, "[]", "script-mode regressions: {fails}");
    // All four scripts ran (two sloppy classics, one `use strict` classic, one
    // module), none threw, and the run settled: the modes are a parse-time fact,
    // so a regression shows up as an error count, not just a failed check.
    assert_eq!(
        (report.scripts, report.errors, report.settled()),
        (4, 0, true)
    );
}
