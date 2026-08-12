#!/usr/bin/env bash
# Single home for "what is frot's source", newline-delimited on stdout.
#
# It exists because the answer was written out at each call site and drifted:
# the line cap's callers both said `git ls-files '*.rs'`, so the 21 JS files of
# `src/js/prelude/` — the entire DOM/JS shim, unambiguously source — were never
# checked, and `elem2.js` sat over the cap unnoticed (bl-6da7). A gate that
# examines the wrong set reports success either way.
#
# The set is defined by what is source, not by one language:
#
#   * Included: every tracked file in a language frot is written in — Rust, the
#     JS prelude, and the shell of the gates themselves — wherever it lives.
#     `.githooks/` holds only hook scripts, which carry no extension.
#   * Excluded: `tests/fixtures/`, the tree's declared home for *data*. Every
#     byte there is third-party (mapped row-by-row in `tests/fixtures/NOTICE.md`
#     and gated by `tests/hygiene/licensing.rs`) or a synthetic page modelling a
#     real one. Data has no line cap; the react/vue/jQuery distributions would
#     fail one, and a gate that fires on something nobody may edit gets turned
#     off. `NOTICE.md`, not the cap, is what governs that directory.
#   * Not enumerated by exception: an allow-list of languages, rather than
#     "everything but docs and config", because `src/` also carries binary test
#     data (`firefox_tls/testdata/*.der`) that `wc -l` reads as noise.
#
# AGENTS.md exempts docs and config, which is why `.md`/`.toml`/`.yml` are
# absent. A new source language belongs in the list below and nowhere else.
set -euo pipefail

git ls-files '*.rs' '*.js' '*.mjs' '*.sh' '.githooks/*' ':(exclude)tests/fixtures/*'
