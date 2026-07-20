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
axes (`docs/design/js.md` — `EXEC_BUDGET_MS = 1000` wall-clock covering script
*and* network, `JS_MEM_LIMIT = 64 MiB`, `VIRTUAL_HORIZON_MS = 10_000`,
`SUBFETCH_MAX = 16` capping JS-driven fetch/XHR count). `--css` has none: a
page with N stylesheets on a slow host costs up to N × 15s, serially, with no
cap on N and no shared budget. Today's floor is "however many `<link>` tags the
page has," which is attacker/host-controlled.

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

1. **Bound and parallelize the CSS fetch path.** Fetch external stylesheets
   concurrently (no new async runtime needed — `std::thread::scope` over the
   existing synchronous `fetch::fetch` suffices; a thread-pool crate is a
   dependency-tree conversation per `AGENTS.md`, not a given). Add an aggregate
   wall-clock budget for the whole CSS-gather phase, mirroring `--js`'s
   axis-bounding — the fix is a budget *and* concurrency, not concurrency
   alone, or a single slow host still hangs the run.
2. **Index the cascade.** Bucket rules by rightmost simple selector
   (id/class/tag) so per-element matching skips rules that cannot possibly
   apply, the standard technique. Must preserve the existing cascade/specificity
   semantics exactly (`cascade.rs`'s `winner()` tie-break order is correct —
   see Finding 2 audit below — this is a matching-cost fix only, not a
   semantics change).

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
