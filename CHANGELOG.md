# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- **`--out ax`: a `<style>` or `<script>` inside an element is no longer
  read into its name.** The text of `<script>`, `<style>`, `<template>` and
  `<noscript>` never contributes to an accessible name, as in Chrome — also
  when `aria-labelledby` points straight at one. A Wikipedia header cell
  carrying inline TemplateStyles was named with the stylesheet's source.
  [bl-b124]

- **`--out ax`: a name keeps its no-break spaces.** Only HTML's ASCII white
  space (space, tab, LF, FF, CR) is collapsed and trimmed in an accessible
  name, as in Chrome: `No&nbsp;break` keeps U+00A0 — it used to come out as a
  plain space — and so do U+2003, U+3000 and the rest. `aria-label` and
  `title` are now collapsed the same way (`aria-label="a  b"` is "a b"; they
  were only trimmed). [bl-3bd4]
- **`--js`: an interface called without `new` throws what Firefox throws.**
  `Event('x')`, `URL('…')`, `Blob()`, `XMLHttpRequest()`, `MutationObserver()`
  and every other constructable interface throw `TypeError: <Name>
  constructor: 'new' is required`; most used to return `undefined`.
  `AbortSignal()` and `new AbortSignal()` throw `Illegal constructor.`
  [bl-6404]

- **`--js`: the document is a node of the tree.** `html.parentNode` is the
  document (it was `null`), and the tree refuses what a browser's does, with
  Firefox's `HierarchyRequestError` messages: a node appended under its own
  descendant (which used to build a cycle), a second element under the
  document, a child under a text node. `el.matches()` and `querySelector` now
  answer for an element that is in no document; `outerHTML` on such an element
  is a no-op. `--out dom` and every other view are unchanged. [bl-196a]

- **The fidelity oracle reads a fourth saved real page** (dev-tree only;
  nothing shipped changes): a Python docs page with its six Sphinx
  stylesheets, one of them reached only through `@import`. It matches Chrome
  on every channel with no ledger row. Link and form references are now
  compared relative to the corpus root, so a saved page may link above its own
  directory. The W3C page stays out (223 differing lines, from absolute
  positioning and `text-transform`; `bl-3248`). [bl-8682]
- **`--out ax`: table and caption edges follow Chrome.** [bl-dfc3]
  - A table of one row holding one cell is a layout table (no `table`, `row`
    or cell node) unless it has a `<caption>`, a non-empty `summary` or a
    `role`; an empty `summary=""` no longer marks a data table.
  - A `<caption>` or `<legend>` that is present is the whole native name:
    `<table title=T><caption></caption>` and `<fieldset title=T><legend>
    </legend>` have no name. A hidden caption is skipped, so the next
    caption, `summary` or `title` names the table.
  - `title` does not name a `<caption>` or a `role=caption` (it still names a
    `<figcaption>`).
  - Whether an empty `<caption>` emits a node now reads the cascade: content
    that is all `display:none` does not keep it, `::before` text does, and a
    caption whose only content is a block-level child, or that is itself
    `display:inline`, has none.
  - `role=form`, `role=region`, `<section>` and a nested `<aside>` keep their
    landmark role when they carry a `title` attribute, even an empty one.
- **`--js`: DOM nodes have the interfaces a browser gives them.** An element
  is an `HTMLDivElement` / `HTMLInputElement` / … (all 71 HTML interfaces, by
  tag) under `HTMLElement`, `Element`, `Node` and `EventTarget`; a text node is
  a `Text` under `CharacterData`. Each member sits on the interface Firefox 153
  puts it on, so a text node has no `getAttribute` and a `<div>` no `value`,
  `href` or `getContext`. A node now has one JS object: `el === el` holds and a
  property a script sets on a node is still there at the next read. New on
  `Node`: `hasChildNodes`, `isSameNode`, `isConnected`, `parentElement` and the
  node-type constants; `new Text()` and `new Comment()` construct.
  `el.textContent = null` or `''` leaves no child. [bl-78cb]

- **`--css` and `querySelector` understand `:first-child` and
  `:last-child`.** List separators drawn with them come out as in a browser:
  `li::after { content: " ·" }` `li:last-child::after { content: none }` no
  longer leaves a trailing `·` (every Wikipedia navbox row). A pseudo-class
  written with two colons (`::first-child`, `::is(a)`) is invalid CSS and no
  longer matches. [bl-e987]

- **`--out ax`: a `<p>` with no line of its own emits no `paragraph`**, as in
  Chrome. That is an empty `<p>` (in every recipe), and under `--css` one
  with any block-level child (`<p><span style="display:block">…`) or one that
  is itself `display:inline` or `flex`; its contents surface in its place. A
  `<p>` carrying `id`, `title`, `tabindex`, `contenteditable`, `role` or any
  `aria-*` attribute keeps its node, as does a `role=paragraph` on another
  element. [bl-5c70]
- **`--css` and `querySelector` understand the sibling combinators `+` and
  `~`.** `h2 + a.self-link::before { content: "§" }` now paints its mark, and
  `document.querySelectorAll('dt ~ dd')` returns the siblings instead of
  throwing. Before, any rule with a sibling step was dropped. [bl-6363]

- **`--out ax`: the DPUB-ARIA roles are honoured.** An author `role` naming
  any of the 41 `doc-*` roles (`doc-noteref`, `doc-footnote`, `doc-chapter`,
  …) is emitted under that name, as in Chrome, instead of falling back to the
  element's own role. `doc-backlink`, `doc-biblioref`, `doc-glossref`,
  `doc-noteref` and `doc-subtitle` are named from their contents; the rest
  only by the author. [bl-fc83]
- **`--css` and `querySelector` understand `:is()`, `:where()` and `:not()`**,
  over lists of complex selectors, nested, with the spec's specificity
  (`:where()` zero; `:is()`/`:not()` their most specific argument). Before, a
  rule using one never applied — and when that rule was the one *revealing*
  content (`.label{display:none}` `.label:not([hidden]){display:inline}`,
  Bootstrap's docs header) the content was wrongly hidden. A `:not()` over
  something frot cannot evaluate (`:not(:hover)`) still never matches, and
  `querySelector` still throws on it. [bl-011e]

- **`--out ax`: a table or list the author gave another role takes its
  parts' roles with it**, as in Chrome. Under `<table role=presentation>` (or
  `none`, `group`, any role but `table`/`grid`/`treegrid`) a `<thead>`,
  `<tr>`, `<td>` or `<th>` emits no node, and under `<ul role=none>` (any
  role but `list`) neither does an `<li>`; their contents surface in place. A
  part with its own `role` keeps it, and a nested `<table>` starts over.
  [bl-2ea9]
- **`--css` follows `@import`.** A stylesheet's `@import "x"` / `@import
  url(x)` rules (in a linked sheet or a `<style>` block, nested imports too)
  are fetched in the same budgeted stylesheet gather and cascade ahead of the
  importing sheet, as in a browser. A media-qualified import is fetched only
  when its query matches the 1280×720 viewport; one carrying `layer` or
  `supports()` is not applied. Before, every imported rule was silently
  missing — a Sphinx docs page kept ~90 hidden pilcrows and headings. A `;`
  inside a quoted at-rule prelude (a Google Fonts URL) no longer swallows the
  rule after it. [bl-3aef]

- **The fidelity oracle now reads three saved real pages** (dev-tree only;
  nothing shipped changes): a go.dev tutorial, a Bootstrap docs page and a
  Wikipedia article, each with its stylesheets, licensed in
  `tests/fixtures/NOTICE.md`. The Go page matches Chrome on every channel;
  the other two surfaced ten divergences, each filed and pinned in the
  ledger. [bl-5334]

- **`FileReader` and `ProgressEvent` exist under `--js`.** A `FileReader`
  reads a `Blob` as a data URL, text, an `ArrayBuffer` or a binary string, and
  fires `loadstart`, `progress`, `load` and `loadend` (or `abort`) in the order
  and with the payloads Firefox does. Text is decoded as UTF-8 only: a legacy
  `encoding` argument or `charset` is ignored. AWS WAF's challenge no longer
  throws on its first `new FileReader()`. [bl-8c46]

- **`--out ax`: `title` no longer names a `term`.** An empty `<dt title=T>`,
  `<dfn title=T>` or `role=term` is `term` with no name, as in Chrome; other
  roles still fall back to `title`. [bl-0794]

- **`--out ax`: `role=form` and `role=region` stand only on a named
  element**, as in Chrome. Unnamed, the next `role` token applies, else the
  element's implicit role: `<div role=region>` is generic,
  `<div role="form button">` is a `button`, `<nav role=region>` is still a
  `navigation`. An unknown first token is likewise skipped rather than voiding
  the attribute (`role="foo button"` is a `button`). [bl-c5a6]

- **`--out ax`: an empty, unnamed table `<caption>` emits no node**, as in
  Chrome. A caption with any element child, any text, or an author name
  (`aria-label`, `aria-labelledby`, `title`) keeps its `caption` node, and so
  do an empty `<figcaption>` and an empty `role=caption`. [bl-fe88]

- **`--out ax`: a `<table>` with no `<caption>` is named from its `summary`
  attribute**, as in Chrome. `aria-label` and a caption outrank it; it
  outranks `title`. [bl-2b09]

- **`Image` and `HTMLImageElement` exist under `--js`.** `new Image()` is an
  `<img>` element outside the tree; `new Image(w, h)` sets its `width` and
  `height`. frot still fetches no images: assigning `src` sends no request (a
  tracking-pixel beacon is not sent), neither `load` nor `error` fires, and
  `complete` stays `false` with a natural size of 0 — for every `<img>`,
  parsed or constructed. A page that waits for an image to load waits until
  the run ends. [bl-1404]

- **`--out ax`: an inline `<svg>` with nothing exposed inside it is an `img`**
  (was always `graphics-document`), as in Chrome. It stays a
  `graphics-document` when it holds text in a `<text>` or a described part.
  A described part — a `<g>`, shape, `<use>` or nested `<svg>` with a
  `<title>`/`<desc>` child or an author name — now has a node of its own
  (`group`, `graphics-symbol`, `graphics-object`), named from its `<title>`;
  inside a link or button it contributes that title and not its contents. An
  SVG `<desc>` no longer leaks into names, and gets no box in `bboxes`.
  [bl-07bd]

- **A vendored React bundle no longer ships in the crate.**
  `src/js/engine/react.production.min.js` — a second, unattributed React
  distribution read by one engine smoke test — is deleted; the test reads the
  licensed React 17.0.2 fixture instead, which is excluded from the package.
  The licensing gate now sweeps the whole tracked tree: every file under
  `tests/fixtures/` needs a `NOTICE.md` row, and no file outside it may carry
  a third-party licence banner. [bl-41e5]

- **`crypto.subtle` now does every secret-key operation Firefox does.** New
  under `--js`: AES-CBC and AES-CTR `encrypt`/`decrypt`; AES-GCM with any IV
  length and all seven tag lengths (it was 96-bit IV, 128-bit tag only);
  AES-KW; PBKDF2 and HKDF through `deriveBits`/`deriveKey`; and
  `wrapKey`/`unwrapKey` under any of the four AES modes — each with
  `importKey`/`exportKey` (`raw`, `jwk`) and `generateKey`. Results, key
  shapes and every error (name, message and which check wins) match Firefox
  153.0esr on 444 captured calls. Two limits: PBKDF2 is refused
  (`OperationError`) past 2 000 000 HMAC iterations × output blocks, where a
  browser would keep computing; and the public-key algorithms (RSA, ECDSA,
  ECDH, Ed25519, X25519) still reject `NotSupportedError`. Amazon's delivered
  home page imports an AES-CBC key, which no longer fails. [bl-db8f]

- **A bot challenge declared at an error status now gets the same treatment as
  any other declared challenge.** A response with status 400 or above that
  carries `cf-mitigated: challenge` or `x-amzn-waf-action: challenge` —
  Cloudflare serves its challenge as a 403 — was `status:"error"`,
  `http.403`, exit 1. It is now `needs:["js"]` without `--js`, and with `--js`
  frot runs the challenge page's script and re-requests once if it changed the
  request, exactly as below 400; `http.status` still reports the 403 and the
  envelope carries the `challenge` block. A 4xx/5xx that declares no challenge
  (including a 429/503 with `retry-after`) is unchanged. Measured: this does
  **not** yet get a Cloudflare-challenged page — its script needs an iframe
  window frot does not have. [bl-a7c5]

- **`addEventListener` / `removeEventListener` / `dispatchEvent` called with
  no receiver act on the window under `--js`**, as in a browser; they threw.
  [bl-a7c5]

- **`--out bboxes` gives a collapsed `<select>`'s options no box.** A select
  is a list box when it has `multiple` (unless `size=1`) or `size` of 2 or
  more; otherwise it is a drop-down, and nothing inside it is laid out — as
  in Chrome, where those options have no client rects. The select keeps its
  own (now empty) box; `text` and `ax` still list every option. The `ax` role
  follows the same rule, so `<select multiple size=1>` is now a `combobox`
  and `size=" 4 "` / `size=2abc` a `listbox`. [bl-2932]

- **The fidelity oracle now checks what it used to excuse** (dev-tree only;
  nothing shipped changes). The recorder reads string `::before`/`::after`
  content and `<textarea>` values from Chrome in paint position, and counts a
  box only where `checkVisibility()` holds, so nine ledger residuals — seven
  `text`, two `bboxes` — became plain equalities and were deleted. Captures
  re-recorded on Chrome 158. [bl-7966]

- **An accessible name computed from contents keeps block boundaries.** The
  contents of any child that is not a plain inline box (a block, table cell,
  list item, inline-block, flex container) and the line breaks `<br>`/`<wbr>`
  are spaced off from their neighbours, as in Chrome:
  `<button><div>a</div><div>b</div></button>` is "a b" (was "ab"), a grid row
  of `<td>`s is "G1 10" (was "G110"). Inline children still join: two
  `<span>`s are "in1in2". Under `--css` the box is the computed `display`.
  [bl-fb69]

- **`--css --out ax` no longer reorders flex children.** The AX tree is in DOM
  order in every recipe, as Chrome's is: flex `order` and
  `flex-direction: *-reverse` do not move AX nodes (measured, Chrome 156).
  `--out ax` builds no layout any more. `--out bboxes` keeps its visual
  reading order. [bl-7430]

- **`Blob`, `URL.createObjectURL` and `URL.revokeObjectURL` exist under
  `--js`.** A `Blob` is real bytes with `size`, `type`, `slice`, `text`,
  `arrayBuffer` and `bytes`. A blob: URL is minted but cannot be loaded (no
  `fetch` of it, and a Blob-URL `Worker` still never runs); there is no
  `stream()` or `File`. [bl-78c5]

- **`document.scripts` exists under `--js`**: the `HTMLCollection` of the
  document's `<script>` elements, as of the moment it is read. [bl-c296]

- **`requestIdleCallback` / `cancelIdleCallback` exist under `--js`.** An idle
  callback runs on the virtual clock once every timer due at that instant has
  run, and receives a real `IdleDeadline`. [bl-b584]

- **`TextEncoder` and `TextDecoder` exist under `--js`**, for UTF-8:
  `encode`, `encodeInto`, and a spec-exact `decode` (replacement characters,
  BOM, `fatal`, `{stream: true}`). Any other `TextDecoder` label is refused
  with a `RangeError` rather than mis-decoded. [bl-53b7]

- **`FormData` exists under `--js`, and a `fetch`/XHR POST can send one.** It
  goes out as the `multipart/form-data` text Firefox produces (same boundary
  format, same escaping), under the existing 64 KiB request-body cap.
  `new FormData(formElement)` is refused — frot does not read a form's
  controls. **With this, frot passes Amazon's declared challenge**: measured
  2026-11-17, `--js` returns the home page with
  `challenge:{rounds:1, passed:true}` every time Amazon declared its
  challenge — 11 runs of 15; the other 4 got an undeclared, textless 200
  (`docs/design/challenge.md` §11.4). [bl-6eb9]

- **`crypto.subtle` exists under `--js`, and it computes real values.** On a
  secure page (`https:`, `file:`, loopback) `crypto.subtle` is a Firefox-shaped
  `SubtleCrypto` with all twelve methods and a real `CryptoKey`. Implemented:
  `digest` (SHA-1/256/384/512); AES-GCM (128/192/256-bit keys, 96-bit IV,
  128-bit tag) and HMAC — `importKey`/`exportKey` as `raw` or `jwk`,
  `generateKey`, `encrypt`/`decrypt`, `sign`/`verify`. Everything else Firefox
  has (RSA, ECDSA/ECDH, Ed25519/X25519, AES-CBC/CTR/KW, HKDF, PBKDF2, other
  AES-GCM IV/tag lengths, `wrapKey`/`unwrapKey`) rejects with
  `NotSupportedError` — never a made-up result. Amazon's challenge script now
  gets through its key import and encryption; it still does not pass (it stops
  at `FormData` next — `docs/design/challenge.md` §11.4). [bl-0647]

- **`window.isSecureContext` exists under `--js`**, as it does in every
  Firefox: `true` on `https:`, `file:` and loopback `http:`, `false` on any
  other `http:` page. `crypto.randomUUID` is now absent on an insecure page,
  which is what Firefox 153 does (`docs/design/identity.md` §3.19). [bl-b0f8]

## [0.0.3](https://github.com/mudbungie/frot/compare/v0.0.2...v0.0.3) - 2026-11-18

### Changes

- build the declared-challenge round trip (challenge.md, signed off by Mark 2026-11-17) [bl-15ff]
- release PR shows no checks: a dispatched CI run stays off the PR's status rollup; restate its verdict as a commit status [bl-4de7]
- make package: cargo publish --dry-run leaves no .crate to leak-scan; CI package step is red [bl-f28a]
- an unnamed <form> is omitted; Chrome exposes it as role form with an empty name [bl-2a3c]
- full CI/CD: fix release-plz PR CI, tags, GitHub Release + musl asset, crates.io publish path [bl-5cf3]
- an inline <svg> is not named from its <title> child (Chrome: "Chart title") [bl-0a0a]
- Adopt the leak-scan stack and wire store publication [bl-59dc]
- <dt> (term) has no name; Chrome names it from its contents (term "Term") [bl-8ad7]
- a row is named from its cells, fused with no separator ("Q110"); Chrome names a row "" [bl-3072]
- <tbody> emits a rowgroup node; Chrome ignores tbody (explicit or implied) and keeps only <thead> [bl-e760]
- a <table> is not named from its <caption> (Chrome: table "Quarterly totals") [bl-fd85]
- HTTP `Refresh:` response header is not followed (sibling of the meta refresh, bl-08d8) [bl-e52a]
- build the fidelity oracle (fidelity.md, signed off by Mark 2026-11-17) [bl-2e08]
- zero-delay meta refresh returns an empty ok impression [bl-08d8]
- renew the persona expiry pin (eol 2027-01-26) from Mozilla's published calendar [bl-d8f1]
- Collapse the pre-commit gate to exec bl-gate (rollout phase 2; ops bl-3166) [bl-8a8b]
- Remote gate: make check is the whole gate; pre-commit delegates to the noodlezoo builder [bl-254b]
- host objects with no interface own their whole surface as instance data [bl-643d]
- the 300-line cap does not see the JS prelude, and elem2.js is already over it [bl-6da7]
- instance backing slots are visible own properties; a real Firefox instance has none [bl-3bdc]

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
