# CSS engine: cost and correctness (Phase 2 follow-up)

Living design note for `--css` (`src/css.rs` + submodules). Written from the
bl-5d13 field-trial investigation (2026-07-19, session Polythene): `--css` cost
2x-18x wall-clock vs bare, and shrank `--out text` by 3%-48% on real pages. Two
independent findings, two independent root causes. When code and this doc
disagree, fix whichever is wrong.

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
  match, never partially match. The module doc states the invariant directly:
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
