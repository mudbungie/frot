# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- **frot now answers a server-declared bot challenge the way a browser does
  unattended — and tells you it did.** When a response declares itself a
  challenge (`x-amzn-waf-action: challenge`, `cf-mitigated: challenge`, or
  `retry-after` on a success response), `--js` now runs the challenge page's own
  script. If that script changes the request frot would send (it mints a
  cookie), frot requests your URL **once** more and returns what comes back. If
  it changes nothing, or the answer is still a challenge, the result is
  `needs: ["human"]` as before. frot never clicks, types, submits a form or
  solves a CAPTCHA; the 1 s CPU and 1 s network budgets are unchanged and are
  shared by the challenge and the page behind it. (`[bl-15ff]`, design
  `docs/design/challenge.md`)
  - **New envelope block `challenge`**, present only when a challenge was
    declared: `"challenge":{"declared":"x-amzn-waf-action","rounds":1,"passed":true}`.
    `declared` is the response header that declared it, `rounds` the extra
    navigations taken (0 or 1), `passed` whether the last one was no longer a
    challenge. It rides `ok`, `needs` and `error` envelopes alike.
  - **Changed verdict:** a declared challenge fetched **without** `--js` is now
    `needs: ["js"]` (it was `needs: ["human"]`), because adding `--js` is the
    recipe that can answer it. With `--js`, an unanswered challenge is still
    `needs: ["human"]`, now with a `js` block describing the script's run.
  - **Page scripts can POST.** `fetch` and `XMLHttpRequest` accept method
    `POST` with a text body (a string or `URLSearchParams`) up to 64 KiB; it is
    sent with the script's `Content-Type` and an `Origin`, and its response is
    never cached. Every other method is still rejected, a non-text body fails
    the request, and frot's own navigation is always a GET.
  - **Pages that declare no challenge are unchanged**, byte for byte, and make
    no extra request.
  - **Measured, not promised:** on 2026-11-17 this did **not** get Amazon's
    page. The AWS WAF script cannot finish in frot's JS environment (it needs
    `crypto.subtle`), so no round is taken and the result is `needs: ["human"]`
    with `rounds: 0`. A Cloudflare challenge served with status 403 stays an
    `http.403` error and is not attempted.

- **`ax`: an unnamed `<form>` is now a `form` node.** `<form><input><button>
  Send</button></form>` used to put the textbox and button at the level of the
  form's parent; they are now children of a `form` with a null name, as in
  Chrome's tree — at top level, inside `<search>` or `<nav>`, or empty. This
  reverses the earlier named-only rule for forms: the HTML-AAM and ARIA make a
  form a *landmark* only when named, but the browser keeps the node either
  way, and frot reports the tree. Consumers counting `form` nodes as landmarks
  should now require a name. `<section>` is unchanged (unnamed, no node), and
  an author `role=` on a form still replaces the role. Found by the fidelity
  oracle. (`[bl-2a3c]`)

- **`ax`: an inline `<svg>` is now named from its `<title>`.** `<svg><title>
  Chart title</title>…</svg>` was `graphics-document` with a null name; it is
  `graphics-document "Chart title"`, as Chrome reports it. The title is the
  first `<title>` *child* — one nested under a `<g>` belongs to the `<g>` —
  and `aria-label`/`aria-labelledby` still win. The same rule now governs what
  a graphic contributes to the name of a link or button around it, with two
  consequences there: a `<title>` that is not a child no longer stands in for
  the whole graphic (its text is still read, along with the rest of the
  graphic's text), and an empty `<title>` no longer silences the graphic's
  text. Found by the fidelity oracle. (`[bl-0a0a]`)

- **`ax`: a `term` is now named from its contents.** `<dt>Term</dt>` was
  `term` with a null name; it is `term "Term"`, as Chrome reports it, and the
  same goes for `<dfn>` and `role=term`. ARIA makes `term` a name-from-author
  role; the browser names it from contents anyway, and frot follows the
  browser. `definition` (`<dd>`) is still unnamed, as in Chrome. Found by the
  fidelity oracle. (`[bl-8ad7]`)

- **`ax`: a row of a plain table no longer has a name.** `<tr><th>Q1</th>
  <td>10</td></tr>` was `row "Q110"` — its cells' text, fused; it is now a
  `row` with a null name, as in Chrome, and the cells keep their own names. A
  row is still named from its cells inside a `grid` or `treegrid`, where
  Chrome names it too, and `aria-label`/`aria-labelledby`/`title` on a row
  count everywhere. A `<td>`'s `cell`/`gridcell` choice now reads the same
  one fact — the nearest ancestor with a table role — which differs from
  before only for a table given a non-table `role=` nested inside a grid.
  Found by the fidelity oracle. (`[bl-3072]`)

- **`ax`: an unnamed `<tbody>` no longer emits a `rowgroup`.** The rows of a
  table body — written out or implied by the parser — are now direct children
  of the `table`, as in Chrome's tree; before, every data table carried a
  `rowgroup` wrapper Chrome does not have. `<thead>` and `<tfoot>` are still
  `rowgroup`s, and so is a `<tbody>` the author named (`aria-label`,
  `aria-labelledby`, `title`) or gave `role=rowgroup`. This departs from the
  HTML-AAM's flat `tbody` → `rowgroup` mapping in favour of what the browser
  exposes. Found by the fidelity oracle. (`[bl-e760]`)

- **`ax`: a `<table>` is now named from its `<caption>`.** `<table><caption>
  Quarterly totals</caption>…` was `table` with a null name; it is
  `table "Quarterly totals"`, as Chrome reports it. The first `<caption>` child
  is read the way any descendant is read into a name — its `alt`s and
  `aria-label` count, a hidden caption says nothing — and `aria-label` /
  `aria-labelledby` on the table still win. The `caption` node itself is
  unchanged. Found by the fidelity oracle. (`[bl-fd85]`)

- **An immediate `Refresh:` response header is now followed, as the
  `<meta http-equiv="refresh">` already was.** A response carrying
  `Refresh: 0; url=/real` sends a browser to `/real` unattended — the same HTML
  rule as the meta, spelled as an HTTP header — but frot ignored the header and
  returned the interim page (often an empty `ok`). It is now the same rule and
  the same hop: redirect budget, cookies, `Referer`, `url.final` all as for the
  meta. When a response declares both, the header is read first (it exists
  before the markup does), so an immediate header wins and a delayed header
  means nothing is followed. The header's relative target resolves against the
  page URL, not a `<base>`. No flag, field or exit code changed. (`[bl-e52a]`)

- **A page that is only an immediate `<meta http-equiv="refresh">` now returns
  the page it points at, not an empty `ok`.** `frot https://docs.astro.build/`
  returned `status: ok` with nothing in `out`: the root document is a
  zero-delay refresh to `/en/getting-started/`, which a browser follows with no
  script and no human. frot now follows it as it follows an HTTP redirect — same
  connection pool and cookies, the refreshing page as `Referer`, `url.final`
  reporting where it landed — and shares the ten-request redirect budget, so a
  refresh loop ends in the existing `fetch.redirect` error. A refresh with a
  delay above zero is not followed (that page is what a browser shows first),
  nor is one inside `<noscript>`, one a script inserts, or one whose target is
  not `http(s)`. Under `--js` the refresh is taken before any script runs. No
  flag, field or exit code changed. (`[bl-08d8]`, decided in `[bl-c262]`)

- **The Firefox 153 ESR persona's expiry moved from 2027-01-26 to 2027-09-07.**
  The build fails on the day the pinned browser line passes the last release
  Mozilla has scheduled for it. Mozilla's calendar now runs 153.x through 153.27
  on 2027-09-07 (read 2026-11-17), with ESR 175 beside it from 2027-06-29. No
  wire or script-visible value changed. (`[bl-d8f1]`)

- **frot's views are now checked against a real browser on every test run.**
  `tests/fidelity.rs` compares `text`, `ax`, `links`, `forms` and `bboxes`
  with what Chrome answered about the same pages, from captures recorded once
  by `scripts/fidelity-record` and checked in — so the comparison needs no
  browser and no network. Agreement is exact, not scored: each view is
  compared only on what frot claims to produce, and every known difference is
  listed with its reason or its open ball. No behaviour of the binary changes.
  (`[bl-2e08]`)

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
