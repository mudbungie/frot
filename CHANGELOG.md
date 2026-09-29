# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.3](https://github.com/mudbungie/frot/compare/v0.0.2...v0.0.3) - 2026-09-29

### Changes

- Collapse the pre-commit gate to exec bl-gate (rollout phase 2; ops bl-3166) [bl-8a8b]
- Remote gate: make check is the whole gate; pre-commit delegates to the noodlezoo builder [bl-254b]
- host objects with no interface own their whole surface as instance data [bl-643d]
- the 300-line cap does not see the JS prelude, and elem2.js is already over it [bl-6da7]
- instance backing slots are visible own properties; a real Firefox instance has none [bl-3bdc]

- **Under `--js`, a DOM object no longer carries frot's implementation on its
  face.** Every instance a page can reach — an element, a `Notification`, an
  audio node, a canvas or WebGL context, an observer, a `URL` — kept its state
  in `_`-prefixed own properties, which `Object.getOwnPropertyNames` reports
  whether or not they are enumerable. A real Firefox instance reports nothing
  there (measured); frot's `Notification` reported ten entries. The state moved
  behind the prototype accessors WebIDL puts it behind, so anything walking an
  instance now sees a browser's shape. (`[bl-3bdc]`)

## [0.0.2](https://github.com/mudbungie/frot/releases/tag/v0.0.2) - 2026-08-12

> **`0.0.0` and `0.0.1` are withdrawn — upgrade to `0.0.2`.**
> Both of those releases packaged design documents that recorded personal
> network identifiers belonging to the author: the egress address of a field
> trial and the geolocation derived from it. They were measurement notes; they
> should never have been published. The identifiers are gone from the tree, the
> affected versions are being yanked from crates.io, and `0.0.2` is the first
> clean release. A published crates.io version can never be edited or replaced,
> so the fix ships as a new version rather than a corrected `0.0.1`. If you have
> `0.0.0` or `0.0.1` pinned or vendored, move to `0.0.2` — the upgrade itself
> changes no behaviour, and everything below is a bug fix on top of it.

### Changes

- **Personal network identity is scrubbed, and gated against return** — the
  packaged design docs no longer carry an egress address or its geolocation
  gloss, and `tests/hygiene/` fails the build if a globally-routable IPv4
  literal or a bare autonomous-system number reappears in any tracked file.
  (`[bl-f521]`, `[bl-f90c]`, `[bl-5638]`)
- **Vendored test fixtures carry their licence and attribution**, checked by
  the same hygiene suite. (`[bl-11f5]`)
- **Public docs match the product** — corrected claims about cookies, phase
  status and challenge passing; removed directional refusal language.
  (`[bl-6f85]`, `[bl-0af9]`)
- **`--out text` and `--out ax` report what a browser actually exposes:**
  - a closed `<details>` body is no longer emitted as visible text, with or
    without `--css` (`[bl-74a6]`, `[bl-d470]`);
  - SVG and MathML text a browser paints is no longer omitted (`[bl-c0a4]`);
  - adjacent `<option>` texts no longer fuse into one token (`[bl-66ed]`);
  - fallback content for `<video>`, `<object>`, `<canvas>` and `<picture>` is
    no longer emitted as page content, and no longer names its ancestor
    (`[bl-0f83]`, `[bl-0aaf]`, `[bl-e79a]`);
  - hidden fallback text no longer masks a failed SPA as status `ok`
    (`[bl-eeb4]`);
  - accessible names: `title` is a name source, `aria-labelledby` resolves at
    an element that has its own alternative, `<optgroup label>` and
    `<option label>` name their elements, and name-from-contents descends into
    descendant text alternatives (`[bl-d8ff]`, `[bl-0482]`, `[bl-4093]`,
    `[bl-3d2e]`);
  - the AX tree honours `aria-hidden` and `inert` subtree exclusion, and
    implicit roles honour HTML context and conditions (`[bl-2fa6]`,
    `[bl-a189]`).
- **CSS and geometry** — inline descendant bboxes keep their containing `x`
  origin (`[bl-2161]`); a linked stylesheet's `media` is honoured, so print CSS
  no longer applies on screen (`[bl-4f0a]`); generated pseudo-content
  contributes layout geometry and its Unicode escapes are decoded instead of
  emitted literally (`[bl-6fcb]`, `[bl-e94a]`).
- **`--js` gets through more real-world pages:**
  - `document.currentScript` exists, so Turbopack chunk loaders work
    (`[bl-a19d]`);
  - classic scripts are no longer forced into strict mode (`[bl-0679]`);
  - `Element.id` and four further DOM properties are writable, as in a browser
    (`[bl-d313]`, `[bl-273b]`);
  - Astro pages no longer abort on DOM child insertion or in engine teardown
    (`[bl-ae88]`, `[bl-5249]`);
  - a starved heap no longer segfaults during parse (`[bl-c385]`);
  - `js.stopped` names the bound that actually ended an unsettled run — the
    compute budget or the network one (`[bl-79dc]`).
- **Browser-surface fidelity re-measured against a real Firefox** — interface
  prototypes and their own-property lists, `IDBKeyRange` range semantics and
  absent statics, `Worker.prototype`'s flattened `EventTarget`, an undeclared
  `crypto.subtle`, the 2D canvas surface, `navigator.appVersion`, and the
  remaining capability surfaces are re-pinned to the measured profile, and the
  profile's end-of-life date is corrected. (`[bl-706b]`, `[bl-6438]`,
  `[bl-d22f]`, `[bl-1ab7]`, `[bl-b128]`, `[bl-6491]`, `[bl-3595]`, `[bl-04ab]`,
  `[bl-7523]`)
- **Fetching and URL resolution** — a relative `<base href>` resolves links
  (`[bl-409e]`); a fetch with no redirect reports the resolved final URL rather
  than the raw spelling (`[bl-2832]`); a binary image response is no longer
  parsed as HTML and reported `ok` (`[bl-0c3e]`); `data:` script sources are no
  longer counted as failed subfetches (`[bl-91bf]`).
- **Repo gates** — the formatting and 300-line source caps are keyed on tracked
  files rather than the staged set, so neither can be skipped at delivery; a
  golden that measured real wall time is deterministic; dead code removed; the
  release-plz branch prune no longer fails open and deletes every release
  branch. (`[bl-0066]`, `[bl-a68b]`, `[bl-c81a]`, `[bl-d673]`, `[bl-a237]`)

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
