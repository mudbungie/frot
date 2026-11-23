# CSS engine: cost and correctness (Phase 2 follow-up)

Living design note for `--css` (`src/css.rs` + submodules). Written from the
bl-5d13 field-trial investigation (2026-07-19, session Polythene): `--css` cost
2x-18x wall-clock vs bare, and shrank `--out text` by 3%-48% on real pages. Two
independent findings, two independent root causes. When code and this doc
disagree, fix whichever is wrong.

## What `--css` supports — the whole claim

Anything not listed here is not applied. This section is the statement of
record; when it and the code disagree, one of them is a bug.

**Where rules come from.** Every `<link rel=stylesheet>` whose `media`
matches, every `<style>` block, and every sheet either one `@import`s
(below). Inline `style=` attributes. Cascade order is: fetched sheets in
document order, then `<style>` blocks in document order — a `<link>` that
follows a `<style>` still cascades ahead of it, which a browser does not do.

**`@import`** (`bl-3aef`; `src/css/import.rs` finds them, `src/run/gather.rs`
fetches them).

- Forms: `@import "x";`, `@import url(x);`, `@import url("x");`, each
  optionally followed by a media-query list. The list goes to the one media
  authority (`media.rs`); an import whose list does not match is not fetched,
  exactly like a non-matching `<link media>`.
- `layer`, `layer(…)` and `supports(…)` on an import are not supported: such
  an import is not applied at all.
- An import counts only where CSS allows it — before every rule other than
  `@charset` and a body-less `@layer`.
- The imported sheet's rules cascade *ahead* of the importing sheet's own
  (the rule stands at the top of the sheet), depth-first for nested imports.
  A relative target resolves against the importing sheet's URL (after
  redirects); a `<style>` block's against the document base.
- It is the same gather: the one `GATHER_BUDGET_MS` budget covers links and
  imports together, through the same session (pool, cache, `-H` scoping,
  the stylesheet request intent). Each import level is one concurrent wave. A sheet
  that fails or does not arrive in time is skipped; its importer still applies.
- Bounds: a sheet that imports itself or any sheet on the chain that led to
  it is not fetched again, and a chain is followed at most `IMPORT_DEPTH = 8`
  imports deep.
- A sheet served over the network cannot import a `file:` URL (the rule that
  already stops a remote page linking a local sheet).

**At-rules.** `@media` (see `media.rs` for the exact query grammar) and
`@import`. Every other at-rule — `@supports`, `@layer`, `@container`,
`@font-face`, … — is skipped with its whole body.

**Selectors.** `*`, type, `.class`, `#id`, `[attr]`, `[attr=value]`,
compounds of those, all four combinators — descendant (space), child (`>`),
next-sibling (`+`) and subsequent-sibling (`~`; `bl-6363`) —
`:is()` / `:where()` / `:not()` (below), the child-indexed pseudo-classes
(below; `bl-e987`, `bl-00d1`), the state pseudo-classes (below; `bl-1508`),
and `::before` / `::after`. Any other pseudo-class, pseudo-element or
attribute operator makes its selector never match. (**Superseded 2026-11-22,
`bl-00d1`:** this sentence went on "the other structural pseudo-classes
included (`:only-child`, `:nth-child()`, `:nth-last-child()`,
`:first-of-type` and its family, `:empty`, `:root`)"; all of those are
supported now.)

**Child-indexed pseudo-classes** (`bl-e987`, `bl-00d1`; `src/css/nth.rs`).
`:first-child`, `:last-child`, `:only-child`, `:nth-child(An+B)`,
`:nth-last-child(An+B)`, and the same five counted among siblings of the
element's own tag name: `:first-of-type`, `:last-of-type`, `:only-of-type`,
`:nth-of-type(An+B)`, `:nth-last-of-type(An+B)`. `:nth-child()` and
`:nth-last-child()` also take ` of S` — `:nth-child(2 of .on, b)` counts only
the siblings matching the selector list `S`, and the element must match it.

- They count element siblings only (text and comments do not). The root
  element is a first, last and only child.
- `An+B` is the grammar Chrome 158 accepts, measured case by case in
  `tests/fixtures/fidelity/structural.html`: `odd`, `even`, an integer, or
  `An` with an optional signed `B` (`2n+1`, `-n+3`, `n`, `+n`, `3n - 2`), in
  any letter case; whitespace only around the sign of `B`. `2 n`, `+ n`,
  `n+ +1`, `2n1`, `2.5` and an empty argument are invalid.
- `of` is lower-case and set off by whitespace (`2 OF li` and `2of li` are
  invalid, as in Chrome); `S` is not parsed forgivingly; the `-of-type`
  forms take no `of`.
- Each weighs as a class, plus the most specific selector of `S`.
- An invalid form never matches. One whose `S` holds an unsupported piece
  (`:nth-child(2 of :target)`) never matches and fails closed under `:not()`.
- `-of-type` compares tag names and ignores namespaces: an SVG `<a>` and an
  HTML `<a>` side by side count as one type.

These are pseudo-*classes*: the two-colon spelling (`::first-child`,
`::nth-child(1)`, `::is(a)`) is invalid CSS and never matches. A selector is matched against the element where it stands in the DOM
(`selector::matches(selector, document, node)`): a sibling step skips text
and comments, as CSS does, and costs a scan of the element's earlier siblings.

**`:is()`, `:where()`, `:not()`** (`bl-011e`; `src/css/pseudo.rs`). Each takes
a comma-separated list of complex selectors (`:not(.a > .b, #c)`), nests, and
may stand in any compound. Specificity is the spec's: `:is()` and `:not()`
count as their most specific argument, `:where()` counts zero. Three limits,
each of which makes the rule never apply rather than apply wrongly:

- The list is not parsed forgivingly. One argument that does not parse, or
  that carries a pseudo-element, voids the whole pseudo-class (a browser
  drops just that argument of `:is()` / `:where()`).
- `:not()` over anything the engine cannot answer — `:not(:target)`,
  `:not(:is(a, :has(b)))` — never matches. Reading an unknown as "false" is
  safe; reading its negation as "true" would apply rules a browser does not.
  **Superseded 2026-11-22 (`bl-1508`)** where this line read "`:not(:hover)`,
  `:not(:is(a, :focus))` — never matches": `:hover` and `:focus` are not
  unknown, and their negations now match (next paragraph).
- Inside `:is()` / `:where()` an unsupported argument never matches; the
  other arguments still do (`:is(:target, .open)` matches `.open`).

**State pseudo-classes** (`bl-1508`; `src/css/pseudo.rs`, `state.rs`,
`focus.rs`). frot loads a page that nobody is using: no pointer, no keyboard,
no history. What a browser answers for such a page was measured in headless
Chrome 158 and is held in `tests/fixtures/fidelity/state.html` and
`focus.html`. Each of these weighs as a class, and each is *known* — so
`:not()` of it matches when it does not.

- **Always false:** `:hover`, `:active`, `:visited`, `:popover-open`,
  `:modal`, `:fullscreen`, `:picture-in-picture`, `:autofill`,
  `:-webkit-autofill`, `:user-valid`, `:user-invalid`.
  `.menu:not(:hover) { display: none }` hides the menu, as in Chrome.
- **`:focus`, `:focus-visible`** — the first element in tree order that has
  `autofocus` and can take focus: a button, select, textarea or input that is
  not disabled (nor `type=hidden`), an `<a href>`, an element with an integer
  `tabindex`, a `contenteditable` host; and that is not under `hidden` or
  `inert` or inside a closed `<details>`. With no such element, nothing is
  focused. **`:focus-within`** — that element and its ancestors.
- **`:root`** — the document's root element. **`:empty`** — no element child
  and no text (whitespace is text; comments are not).
- **`:link`, `:any-link`, `:-webkit-any-link`** — an `<a>` or `<area>` with
  `href` (an SVG `<a>` with `xlink:href` too).
- **`:checked`** — a checkbox with `checked`; a radio with `checked` unless a
  later radio of the same name and form has it too; an option its select
  picks: with `multiple`, each one with `selected`; otherwise the last one
  with `selected`, else — in a drop-down, not a list box (`size` above 1) —
  the first that is neither `disabled` nor in a disabled `<optgroup>`.
- **`:disabled`** — a button, input, select, textarea or fieldset with
  `disabled`, or inside a disabled `<fieldset>` other than in its first
  `<legend>` child; an option or optgroup with `disabled`, in a disabled
  optgroup, or in a disabled select. **`:enabled`** — any of those seven
  elements that is not `:disabled`.
- **`:required`** — a select, textarea or input with `required`, except input
  types `hidden`, `range`, `color`, `submit`, `image`, `reset`, `button`.
  **`:optional`** — an input, select, textarea or button that is not
  `:required`.
- **`:read-write`** — a textarea, or an input of a text-holding type (not the
  seven above, nor `checkbox`, `radio`, `file`), that is neither `readonly`
  nor disabled; any other element whose nearest `contenteditable` (its own or
  an ancestor's) is on. **`:read-only`** — every other element.

All of these read the tree's attributes. What a script changes without
leaving an attribute — `el.focus()`, `input.checked = true`, `showModal()` —
is not seen, and an `autofocus` element hidden only by a stylesheet still
counts as focused (`bl-bc6d`).

Still unsupported, so never matching and failing closed under `:not()`:
`:target` (the engine is not given the URL; `bl-5852`), and `:has()`,
`:lang()`, `:dir()`, `:scope`, `:open`, `:defined`, `:default`,
`:indeterminate`, `:placeholder-shown`, `:valid` / `:invalid`, `:in-range` /
`:out-of-range`, the media-element states (`bl-2a85`).

JS `querySelector` / `querySelectorAll` / `matches` / `closest` share this
grammar through `css::query_all`: the combinators, the three logical
pseudo-classes, the child-indexed and the state pseudo-classes
are accepted — `querySelector(":hover")` returns `null`, as in a browser. A
selector holding an unsupported piece at any depth (`p:is(:target)`) throws
`SyntaxError` as before. (**Superseded 2026-11-22, `bl-1508`:** this
paragraph gave `p:is(:hover)` as the selector that throws.)

**Properties.** `display`, `visibility` (inherited), `content` on
`::before`/`::after`, `flex-direction` and `order`.

## Finding 1 — wall-clock cost: two additive, independent causes

`--css` has no single bottleneck; it has two, and which dominates depends on
the target site's CSS shape.

### 1a. External stylesheet fetch is serial and unbounded

`external_css` (`src/run.rs`) fetches every `<link rel=stylesheet>` href with a
plain `.filter_map()` over `fetch::fetch` — one request at a time, no shared
connection, no concurrency:

```rust
fn external_css(doc: &Document, base: &str, headers: &[(String, String)]) -> Vec<String> {
    external_hrefs(doc, base)
        .into_iter()
        .filter_map(|u| {
            let h = if fetch::same_origin(&u, base) { headers } else { &[] };
            fetch::fetch(&u, h).ok().map(|r| r.body)
        })
        .collect()
}
```

Each call to `fetch::fetch` carries the same `TIMEOUT_SECS = 15` (`src/fetch.rs`)
as the page fetch itself, applied per-request. There is no aggregate CSS-fetch
budget. Confirmed live: `linear.app` serves **68** separate stylesheet chunks
(Next.js CSS-module splitting); its measured 707ms → 5010ms (7x) is dominated
by this path — 68 serial round-trips to the same origin, none reused, none
parallel.

This is the asymmetry the field trial flagged: `--js` is bounded on multiple
axes (`docs/design/js.md` §5 — `EXEC_CPU_MS = 1000` of CPU time for script
execution and `NET_BUDGET_MS = 1000` of wall time for network, two bounds in two
units since `bl-8dc0` split the one wall-clock `EXEC_BUDGET_MS`;
`JS_MEM_LIMIT = 64 MiB`, `VIRTUAL_HORIZON_MS = 10_000`, and at
the time of this investigation `SUBFETCH_MAX = 16` capping JS-driven fetch/XHR
count — since retired by bl-c7e9 on the grounds that a request count prices no
real resource; the surviving JS bounds are time, engine heap, and fetched
bytes). `--css` has none: a
page with N stylesheets on a slow host costs up to N × 15s, serially, with no
cap on N and no shared budget. Today's floor is "however many `<link>` tags the
page has," which is attacker/host-controlled.

#### Fixed — bl-8ba2 (`src/run/gather.rs`)

The gather phase is now concurrent and bounded. Two bounds, one per real
resource — the shape js.md §6 settled on when `SUBFETCH_MAX` was retired
(bl-c7e9): a *count* of requests prices no resource, so there is deliberately
no cap on how many stylesheets a page may link. Every sheet is still
attempted, in document order, until time runs out.

- **`GATHER_BUDGET_MS = 5_000`** — one aggregate wall-clock ceiling for the
  whole phase, all sheets combined. Rationale: the budget is a *backstop, not
  the mechanism*. Concurrency is what makes the common case fast; this exists
  so one slow or dead host cannot set the floor for the run. It is sized from
  both ends — comfortably above a full wave of legitimately slow sheets (a
  cold cross-origin CDN handshake plus transfer, ~1–2 s), and far below the
  per-request `fetch::TIMEOUT_SECS = 15` that was previously the phase's
  effective limit, so `--css` can no longer cost 15 s × N. On the measured
  worst real page (linear.app, 68 chunks) it does not trip at all.
- **`POOL_PER_HOST = 6`** — simultaneous requests, via `std::thread::scope`
  over the blocking `FetchSession` surface (`bl-08f6` moved this into the one
  shared `fetch::fetch_many` primitive the JS initial-script warm also rides —
  gather no longer owns its own worker loop). This bounds *sockets and OS
  threads*, which are real resources, and is not a count cap in the
  retired-`SUBFETCH_MAX` sense: nothing is dropped for being the Nth sheet.
  Six is Firefox's per-server limit
  (`network.http.max-persistent-connections-per-server`), the same constant
  the h1 connection pool keeps warm; frot already presents as Firefox — 68
  parallel connections would be both a fingerprint tell and rude to the origin.
  On h2 the wave multiplexes onto one connection; on h1 it opens at most six.

Both are constants, not flags: same severability posture as the 1280 px
viewport (`layout.md` §4) and `EXEC_CPU_MS`. Nothing about a caller's page
makes a different number right.

The deadline is the single authority. Each request's timeout is *derived* from
what the budget has left (`fetch::fetch_within`), rather than the phase
tolerating one in-flight overshoot as the JS subfetch seam does — so a host
that accepts the connection and never answers costs the phase its remaining
budget and nothing more. Statelessness holds: the budget is a parameter
(`gather_within`, the seam the budget-trip test dials down, mirroring
`Session::with_bounds`), the only shared state is an atomic work cursor, and
workers return their results at join rather than sharing a buffer.

A tripped budget is not a run failure — CSS stays best-effort, as
`external_css` always documented: whatever arrived applies, in source order.
Note this is silent, consistent with the pre-existing treatment of individual
sheet failures. Surfacing "the impression was gathered under a tripped CSS
budget" in the envelope would be the honest-signals move (VISION principle 5),
but it changes the output schema and so is escalation-gated; deliberately not
taken here.

Measured on a local stub origin serving 30 stylesheets at 100 ms each
(release build, same machine, three runs): **3.03 s → 0.50 s**. The after
number is 5 waves × 100 ms, exactly the `MAX_IN_FLIGHT` prediction; the before
number is the serial sum.

### 1b. Selector matching is unindexed — O(elements × rules)

`cascade`/`resolve` (`src/css/cascade.rs`) computes styles by, for every
element in the document, scanning every stylesheet, every rule, every
selector, and testing it against the element plus its full ancestor chain:

```rust
for sheet in sheets {
    for rule in &sheet.rules {
        for sel in &rule.selectors {
            if !super::selector::matches(sel, el, &near_first) { continue; }
            ...
```

No bucketing by tag/class/id (the standard technique real engines use to cut
average per-element work close to O(1)). Cost is `O(elements × total
selectors)`, multiplied again by ancestor-chain walks for descendant/child
combinators.

Confirmed by isolating network from computation: serving discord.com's HTML
and its two stylesheets from `localhost` (fetch latency <30ms) still cost
0.65-0.79s of wall-clock under `--css`, vs 0.01s with the external stylesheets
removed. The site's one large stylesheet (a Webflow-generated CDN asset, 2.5MB,
**30,864** rules) drove the entire delta over a ~1,059-element DOM — CPU-bound
matching, not fetch latency. A second, smaller same-origin sheet (730KB, 428
rules) cost 0.02-0.03s in the same harness — the delta tracks *rule count*, not
byte size (30,864 vs 428 rules for only 3.5x more bytes, but ~30x more time).


#### Fixed — bl-fce3 (`src/css/index.rs`)

Rules are now bucketed by the **key simple** of their subject (rightmost)
compound — id, else class, else type — so an element tests only the rules its
own id / classes / tag can reach, plus a small always-tested bucket. Built
once per `cascade` call and passed through the walk (`Index::build(sheets)`);
no global, no cache, statelessness intact.

Two invariants make the indexed walk *observationally identical* to the
exhaustive one, and both are asserted mechanically in `src/css/index/tests.rs`:

- **Completeness.** Every simple in a compound is *required* for that compound
  to match, so bucketing on any one of them cannot lose a match. Selectors with
  no usable key — `*`, attribute-only, and `Simple::Unsupported` — land in the
  always-tested `others` bucket rather than being dropped. This is load-bearing
  for `selector.rs`'s fail-closed contract ("a rule we cannot evaluate must
  never hide or reveal content"): a dropped unsupported rule would silently
  turn fail-closed into wrongly-*revealed* content.
- **Order.** Bucketing reorders iteration, and `winner()`'s `max_by_key`
  tie-break resolves exact ties to the *last* applied declaration — so order is
  semantics, not incidental. Each (rule, selector) pair therefore carries the
  sequence number it held in the flat scan, and `candidates()` re-sorts by it.
  Declarations reach the cascade in exactly the original order.

`winner()`'s `(important, inline, specificity, source-order)` key is untouched,
as is every selector-matching rule; only the *set of rules tested* changed.

Measured on the reproduction the investigation described — a synthetic 30k-rule
utility sheet over a 1,000-element DOM, release build, same machine:
**1.074 s → 0.039 s (27x)**, identical output.

The `cascade.rs` split that came with it is a line-budget consequence, not a
redesign: the file sat at exactly 300 lines. It now divides on a real seam —
`cascade.rs` decides *which* declarations apply, `computed.rs` decides *what
they compute to* (`Applied`, `winner`, the per-property getters), `content.rs`
builds `content:` strings.

### Which one shows up depends on the site

- **Many small stylesheets** (linear.app, 68 files) → fetch/serial-bound.
- **Few huge-rule-count stylesheets** (discord.com, one 30k-rule sheet) →
  matching-bound.
- discord.com's measured 18x (207ms → 3808ms; reproduced locally as ~15x,
  0.11s → 1.6s) splits roughly evenly between the two: ~0.75-1s serial fetch of
  two remote sheets, ~0.65s matching cost for the 30k-rule sheet, isolated from
  network above.

### Recommendation

Two independent fixes, two independent balls (different code paths, different
risk, different tests — don't land them together):

1. ~~**Bound and parallelize the CSS fetch path.**~~ **Done — bl-8ba2**, see
   "Fixed" under Finding 1a above. Landed std-only (`std::thread::scope`, no
   new dependency): `MAX_IN_FLIGHT = 6` concurrent requests under one
   `GATHER_BUDGET_MS = 5_000` aggregate budget. The fix had to be a budget
   *and* concurrency — concurrency alone still lets a single slow host hang
   the run.
2. ~~**Index the cascade.**~~ **Done — bl-fce3**, see "Fixed" under Finding 1b
   above. Bucketed by the subject compound's key simple (id/class/tag), with a
   keyless always-tested bucket; cascade and specificity semantics are
   bit-for-bit preserved, including the fail-closed treatment of unsupported
   selectors.

## Finding 2 — `--out text` shrink under `--css`: correct behavior, not a bug

Prior art: **bl-4a9e** traced a vuejs.org -51% / theguardian.com -32% AX
shrink to CSS; **bl-8ff4** then root-caused it to `@media` blocks being
skipped wholesale (module doc: "`@media` visibility rules are intentionally
not evaluated; we have no viewport without layout" — stale even at the time,
since the fixed viewport landed with bl-e961). That fix shipped as commit
`99834c7` on **2026-07-17**. The bl-5d13 field trial ran **2026-07-19** — two
days later — so the trial numbers already reflect the fix, and the worktree
this investigation used is built from `HEAD` at `99834c7`. This is not the
same bug resurfacing.

Traced the actual vanished content by class name:

- **figma.com** (bare 3971 → `--css` 2424 chars, -39%): removed words
  (Government, Education, Gallery, Demos, Config, BuzzBeta, …) all live under
  `li.fig-av9obh` / `span.nav-highlight` inside the site's mega-nav — a
  hover/click-triggered dropdown panel, `display:none` in the authored CSS
  until interaction.
- **discord.com** (4117 → 2135, -48%): removed content splits into two real
  components — a language switcher (`Dansk`, `Čeština`, … under
  `div.dropdown-language-item` inside `li.dropdown-list-container`) and a
  header mega-menu (`Careers`, `"Case Studies"` under `a.nav_link`/
  `a.dd_nav-link`, `dd_` = dropdown) — both `display:none` pre-interaction,
  both classic Webflow dropdown-widget markup (the site's one 30k-rule
  stylesheet is Webflow-generated, breakpoint/state utility classes baked in).

A real desktop Firefox with no synthetic interaction does not paint or expose
either of these either — this is exactly the case VISION already calls out:
"Some shrink is correct (`display:none` filtering is the point)." frot has no
synthetic click (`docs/design/js.md` §11, non-goals: "No interaction. No
synthetic clicks, no form submission, no input events, no scrolling.") — a
permanent, load-bearing property — so hover/click-revealed content can never
appear in `--out text`/`--out ax` under `--css`, with or without `--js`. The
magnitude gradient across the trial (figma -39%, discord -48%, linear -10%,
svelte -3%) tracks how much dropdown/mega-menu/language-switcher chrome each
site's header carries, not a uniform engine defect — a real engine bug would
be expected to hit structurally similar pages proportionally, not track how
flyout-heavy each site's nav happens to be.

Also specifically audited, and ruled out, the two mechanisms the field trial
raised as suspects for "over-hiding":

- **"a selector it half-supports matching too broadly"** — disproved.
  `src/css/selector.rs`'s `compound_matches` maps `Simple::Unsupported =>
  false`: any unrecognized pseudo-class/pseudo-function (`:hover`,
  `:nth-child(...)`, `::first-line`, …) makes the *whole compound* fail to
  match, never partially match. (**Superseded 2026-11-22, `bl-1508`:**
  `:hover` is no longer in that list — it is known-false, which matches
  nothing just the same but lets `:not(:hover)` match; see "State
  pseudo-classes" above.) The module doc states the invariant directly:
  "a rule we cannot evaluate must never hide or reveal content." Unsupported
  attribute-selector operators (`^=`, `*=`, `~=`, `$=`, `|=`) aren't parsed by
  `parse_attr` at all, so the whole selector is dropped from the sheet — never
  treated as a wildcard match. Both paths fail closed by construction.
- **"an unsupported construct defaulting to hidden"** — disproved for
  `@supports`/`@layer`/other at-rules. `src/css/parse.rs` skips their bodies
  wholesale — the rules inside never apply. An unmatched/skipped at-rule can
  only cause *under*-application (a rule that would have changed `display`
  never runs), which leaves the UA-implicit display standing — visible. It
  cannot force a hide.
  **Corrected 2026-11-21 (`bl-011e`, `bl-3aef`; found by the fidelity oracle
  on saved real pages):** that holds only when the rule that failed to apply
  was the one doing the hiding. When an applied rule hides and a skipped rule
  *reveals* — `.label{display:none}` then `.label:not([hidden]){display:inline}`
  (Bootstrap's docs header) — under-application leaves the content hidden.
  `:is()` / `:where()` / `:not()` are supported since `bl-011e`; the hazard
  remains for every selector still listed as unsupported above.
  The same went for a skipped `@import` whose sheet reveals, until `bl-3aef`
  taught the gather to follow imports.
- Cascade tie-break (`cascade.rs::winner`, keyed
  `(important, inline, specificity, order)`, highest wins) matches the real
  CSS cascade order faithfully: importance first, inline-vs-selector second at
  equal importance, specificity third, source order last. No bug found here.

### Recommendation

No code fix. Close this half of the investigation as "correct, already
improved by bl-8ff4." Optional low-priority follow-up: a documentation-only
ball to record, near VISION.md's capability description or js.md §11, that
pre-interaction chrome (dropdowns, language switchers, mega-menus) is expected
to be absent from `--out text`/`--out ax` under `--css` — not a gap, a
permanent consequence of "no synthetic interaction" — so future field trials
don't re-open this exact question on a fresh site.

## Finding 3 — `<link media>` was ignored, so print CSS applied on screen

Filed as **bl-4f0a**, observed 2026-08-11 on w3.org and the Apache HTTP Server
docs. `run/gather::external_hrefs` selected every `<link rel=stylesheet>` by
`href` alone and discarded `media` before fetching, so a sheet the browser
loads but never applies at a screen viewport cascaded anyway. On w3.org the
print sheet's `#global-nav ul { display:none!important }` deleted the whole
site navigation from `--out text` (raw 5102 chars → CSS 2903); Chrome at
1280×720 keeps that list visible at 953×49.

### Fixed — bl-4f0a (`src/run/gather.rs`)

A `<link>`'s `media` attribute now decides whether its sheet participates,
evaluated by **`src/css/media.rs`** — the same single authority that evaluates
`@media` preludes and that JS `matchMedia` delegates to. The attribute *is* a
media-query list, the same grammar, so it is the same call
(`gather::media_applies`); there is deliberately no second evaluator, and
`<link media=print>` and `@media print` cannot disagree. Consequences all fall
out of that one delegation, none of them separately coded:

- Absent or empty `media` is `all` (participates); comma lists are OR;
  `only`/`not`, and width/height conditions resolve against the fixed
  1280×720 viewport.
- A malformed or unknown list is `not all` and never participates —
  media.rs's documented conservative rule. Unlike the fail-closed rule for
  *selectors* (`selector.rs`: never hide or reveal content), dropping a sheet
  can only cause **under**-application, which leaves content visible; it can
  never hide something a browser shows.
- Nested `@media` inside a sheet that *did* qualify is still evaluated on its
  own by `parse.rs`, one level down, through the same authority.

The non-matching sheet is dropped *before* the fetch: the contract is cascade
behaviour, and a sheet that cannot affect styles has no reason to cost a
request or a slot in the gather budget. Browser fetch-priority behaviour (real
engines do download print sheets, at low priority) is not modelled — it is
unobservable in every frot output. Source order is otherwise untouched, so
cascade order among the surviving sheets is unchanged.
