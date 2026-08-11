# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.1](https://github.com/mudbungie/frot/releases/tag/v0.0.1) - 2026-08-11

The first release meant to be installed. `0.0.0` existed only to register the
crate name and should not be used.

> **Changed behaviour — a lost impression is now a failed exit status.**
> Until now, if frot could not write its envelope to stdout, it still exited
> `0`: the product was gone and nothing said so. From `0.0.1`, a broken pipe
> kills frot with `SIGPIPE` (a shell reports `141`) and any other stdout write
> failure exits `3` with a diagnostic on stderr. If you pipe frot into a
> consumer that can close early (`frot … | head -1`) and you treat a zero exit
> as "impression taken", that assumption no longer holds — check the status.

### Changes

- **Exit status: stdout write failures are detected** — `EPIPE` dies by
  `SIGPIPE` (`141`), every other write failure exits `3`, diagnostic on stderr.
  Previously the envelope could be silently lost under a successful exit
  status. (`[bl-34fb]`)
- **Argv: the `--` end-of-options delimiter is accepted** — everything after
  `--` is an operand, so a URL or path that starts with `-` can be passed
  (POSIX XBD 12.2 Guideline 10). (`[bl-28d1]`)
- **The CLI contract is written down and gated** — `docs/design/posix.md` pins
  the profile (POSIX.1-2017) frot claims for argv syntax, streams, exit
  statuses, signals and lifecycle, and `scripts/posix-suite.sh` (`make posix`)
  tests the binary against it. (`[bl-5382]`)
- **`--js`: real client-rendered React and Vue apps now settle** instead of
  coming back `needs-js`. The environment gained the surface deployed bundles
  actually touch:
  - `NodeList` and `HTMLCollection` as named interfaces, returned by every DOM
    query (`[bl-e5c3]`);
  - element breadth — `focus`/`blur`/`document.activeElement`, `DOMTokenList`
    for `classList` and `relList` (including `supports()`), `createElementNS`,
    `closest`, sibling navigation (`[bl-3a36]`);
  - `MutationObserver`, implemented at the mutation seam so it observes real
    tree changes; `IntersectionObserver` and `ResizeObserver` deliver a genuine
    initial batch (`[bl-07ab]`);
  - comment nodes are real nodes in the tree, so frameworks that navigate from
    comment anchors (Vue's `RouterView`, `v-if`) work (`[bl-79db]`);
  - the event contract — `DocumentFragment` is an `EventTarget`, the global is
    an `EventTarget`, and `AbortController`/`AbortSignal` exist (`[bl-e81b]`).
- **Licence text ships with the crate** — `LICENSE-MIT` and `LICENSE-APACHE`
  are in the tree and in the package, matching the declared
  `MIT OR Apache-2.0`. (`[bl-cd36]`)
- **Crate description corrected** — the old tagline claimed frot never executes
  or renders, which `--js` and `--out bboxes` contradict; it now says scripts
  run only when you ask and frot never drives the page. (`[bl-419f]`)
- **Release pipeline publishes to crates.io** and no longer uses Node-20
  actions. (`[bl-a498]`)
