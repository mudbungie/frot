# View fidelity — a differential oracle against a browser

Living design for `bl-55cd`. **Status: PROPOSAL, awaiting Mark's sign-off. No
code has changed and no sibling balls exist** — decomposition happens only
after sign-off (the `bl-0356` → eight-siblings pattern). It fits inside
`VISION.md` (frot is not a browser; principle 5, honest signals; principle 7,
testability) and beside `docs/design/identity.md`, whose oracle discipline —
normalized captures, declared residuals, a falsification rule — this design
reuses on the view axis. When code and this doc disagree, fix whichever is
wrong and say so here.

Every claim below marked *measured* comes from the dated session in §12, run
against Chrome 139.0.7258.138 (`--headless=new`, 1280×720) and the worktree
build at `75b2d42` while this document was being written — the design was in
contact with the instrument it specifies.

## The one-sentence shape

Both instruments read the **same bytes**; a browser's answers are **recorded**
(CDP, pinned by version and date) into checked-in captures; frot's views are
compared to the captures through **per-view projections onto frot's declared
model**, where the predicate is **equality** — every divergence is a filed
defect or a ledger-declared residual, nothing in between — and the comparison
runs offline in `cargo test`, so Chrome is needed only to re-record, never in
CI and never in the shipped artifact.

## 1. Why inspection does not converge

The 2026-08-12 backlog was twenty balls and one sentence: frot's impression
disagrees with what a browser paints. Both directions — emitted-but-unpainted
(hidden fallback `bl-eeb4`, closed `<details>` `bl-74a6`, media fallback
`bl-0f83`) and painted-but-dropped (SVG text `bl-c0a4`, name-from-contents
`bl-3d2e`, pseudo-content geometry `bl-6fcb`). Three facts from that burn-down
justify building infrastructure rather than filing ball twenty-one:

1. **One claim, falsified twice, in the same arena.** `bl-0f83` landed "the
   video rule holds in every recipe"; `bl-0aaf` falsified it within hours (the
   accname walk didn't read the rule); `bl-d470` found the same defect a third
   time in a third walk (`views::text` missed the `<details>` half). Three
   hand-inspections of one seam, three misses. Inspection samples the space;
   it does not cover it.
2. **Measurement ruled out the assumed design.** `bl-e79a` measured Chrome via
   CDP and the answer contradicted what every prior agent had assumed: canvas
   fallback is *not* `display:none` — Chrome computes `display:block;
   visibility:visible`, paints no box, and still exposes `StaticText` to AX.
   That measurement now lives in `dom/conceal.rs` as the `unpainted` /
   `concealed` split (`layout.md` §2.1). No amount of spec-reading produced
   it; ten minutes of CDP did.
3. **The instrument keeps being rebuilt and thrown away.** `bl-ae88` diffed a
   fixture byte-for-byte against Chrome; `bl-e79a` built an
   innerText/getClientRects/getFullAXTree probe; `bl-273b` was told to do the
   same. Each in a scratchpad, each discarded. The highest-yield tool the
   project has is the only one with no home in the repo.

Prior art in-repo: `examples/ab_harness.rs` (`bl-46f5`) is this shape on the
identity/transport axis — a deterministic offline half for CI plus a live
dated half. §4 confirms the split with one correction the measurement forced.

## 2. What is compared — the channel map

frot's views and Chrome's observables are not 1:1. Pretending they are
manufactures false deltas, so the map is explicit, and each channel names the
projection (§3) that makes its two sides commensurable.

### 2.1 The recipe rule

A browser has no recipe knob: it always runs full CSS and JS. So exactly one
frot recipe per fixture is browser-comparable — **`--css`** for a static
fixture, **`--css --js`** for a fixture whose scripts are deterministic.
Lower recipes (`--out text` bare, `ax` without `--css`) are *not* compared to
a browser; they are covered by frot's own contract tests, because their
divergence from a browser is frot's documented recipe semantics
(`layout.md` §2.3), not a fidelity fact. The one prior confusion here —
which rules hold in which recipe — is exactly what `bl-0aaf`/`bl-d470` were,
and the fix was accessors in frot, not comparisons against Chrome.

### 2.2 The channels

| frot view | Chrome observable | projection (§3) | expected divergence |
|---|---|---|---|
| `text` | `document.body.innerText` | whitespace-collapsed token sequence | **zero** (measured: 521/521 and 776/776 tokens on two real pages) |
| `ax` | `Accessibility.getFullAXTree` | roled-node tree in frot's model: prune `ignored`/`InlineTextBox`/`StaticText`/`ListMarker`/`generic`/UA-widget subtrees; map Blink role names; trim names | zero after projection (measured on the §12 fixture set) |
| `links` | `document.links` | filter frot's `links` to anchor kind (frot's view also carries `<link>`/`<area>`/forms of reference `document.links` lacks — measured 195 vs 163 on one page); compare resolved `href` + text in document order | zero |
| `forms` | `document.forms` + `form.elements` | field list per form: tag/type/name/action/method | zero (unmeasured this session; verify in the sibling) |
| `bboxes` | `getClientRects().length > 0` per element | **paintedness only** — box vs no box; never a coordinate | zero for flow content; declared non-goals excluded (§3.1) |

### 2.3 Channels deliberately absent

- **`dom`** — comparing serialized DOMs measures html5ever against Blink, two
  spec-converged parsers, and under `--js` measures script scheduling. The
  defect class this ball exists for lives in *views over* the DOM, not the
  parse. (`bl-ae88`-style byte cross-checks stay what they were: ad-hoc
  evidence inside a bug ball.)
- **`meta`** — trivial extraction; no browser disagreement surface worth a
  capture.
- **Coordinates** — see §3.1. This is not a deferral; it is a refusal.

## 3. The predicate — projection, then equality

**This is the section the design lives or dies on.** `layout.md` is explicit
that layout is a structural estimate, not pixel truth (`LINE_HEIGHT = 20`,
`GLYPH_ADVANCE = 8`, no margins, no real fonts). Measured (§12): on a
13-element fixture, **every one of frot's 13 rects differs from Chrome's on
every axis** — the 8px default body margin alone shifts the entire page. A
bbox-equality harness is red on all pages forever; a tolerance-threshold
harness is the same harness with the alarm taped over. Both get muted, and a
muted oracle is worse than none.

### 3.1 The projections are the acceptance policy

The move that dissolves the problem: **acceptable divergence is not a
tolerance on a comparison — it is exclusion from the comparison domain.** A
fact enters a channel only if it is invariant under frot's published
approximation contract and inside frot's declared view model. Everything else
is out *by construction*, so there is no epsilon anywhere, no similarity
score, and no per-fixture tuning:

- **Coordinates are out; paintedness is in.** Every absolute px value is
  downstream of the estimated metrics, so no coordinate is compared, ever.
  Whether an element generates a box at all is *not* downstream of metrics —
  it is the visibility model, and it is exactly the axis the whole twenty-ball
  backlog lived on. The `bboxes` channel compares only that. (Ordinal facts —
  reading order, x/y ordering — measured as surviving for flow and flex, and
  breaking exactly at the declared non-goals, `position:absolute` and floats.
  They are *not* in the initial predicate: the text channel already carries
  order, and an ordinal-geometry comparator would need the non-goal coercion
  table as an exclusion list — mechanism the first corpus hasn't earned.
  OQ-2.)
- **Formatting is out; token sequence is in.** Chrome's `innerText` emits
  `\n\n` at paragraph margins and `\t` between table cells; frot's `text`
  emits one `\n` per block. Both are formatting policy over the same painted
  words, so the projection collapses all whitespace runs and compares the
  token sequence. Measured: this projection takes two real pages to
  **zero deltas** while still catching real defects — it is what surfaced the
  `<option>` fusion (§12.3), because a fused word is a token no browser
  produced, not a whitespace difference.
- **Text-node granularity is out of `ax`; roled structure is in.** frot's
  `ax` view is "roles, accessible names, levels" (README) — it has no
  `StaticText`, no `InlineTextBox`, no `ListMarker`, no UA-shadow widgetry.
  Measured: Chrome's raw tree for the §12 fixture is ~4× frot's node count,
  and includes a play button, a mute button, a time scrubber, and a link
  Chrome names `"Unable to play media."` — none of which exist in the page's
  markup. The projection prunes Chrome's tree to frot's model (drop
  `ignored`, layout/text-granularity nodes, nameless `generic` wrappers, and
  UA-widget subtrees under media/input elements; map Blink's role vocabulary
  — `DisclosureTriangle`→`button`, `SvgRoot`→`graphics-document`, … — one
  finite table in the comparator; trim name whitespace). What survives is
  exactly what frot claims to produce, and on that, equality.

The direction matters: the projection maps **the browser's channel onto
frot's declared model**, never frot toward the browser. A harness that
pressured frot to emit `StaticText` nodes or media-control buttons would be
pushing frot across the not-a-browser line VISION holds on purpose.

### 3.2 Measured: expected divergence on the projection is zero

The reason equality is the right predicate and a "ranked delta report" is the
wrong emission: on the text channel, a saved w3.org front page and a saved
Wikipedia article — real pages, full CSS, ~50–100 KB — compare
**token-for-token identical** (521/521, 776/776; `difflib` ratio 1.0) between
`frot --css --out text` and Chrome 139 `innerText`. The channel's noise floor
is not "low"; it is zero. Every delta is therefore signal: a defect on one
side, or a residual worth declaring. A harness whose expected-pass state is
exact equality cannot cry wolf, and a harness that cannot cry wolf cannot
justify being muted.

### 3.3 The residual ledger

Some divergences are *correct* — frot is right and the channel is blind, or
frot's model deliberately excludes the fact. These are handled exactly as
`identity.md` §11/§12 handles ClientHello residuals: **asserted as residuals,
never silently excluded.** The ledger is a per-fixture, per-channel list of
expected deltas, in the capture's own terms, each with a reason and a source
citation. The comparator applies the ledger *after* projection and fails in
both directions — an undeclared delta fails, and a declared delta that stops
appearing fails too (the residual's assertion is that it still differs in the
declared way). A residual can therefore never drift silently, and the ledger
is reviewed like code because it is code.

The founding entries, all measured (§12):

| residual | reason |
|---|---|
| frot `text` includes string `::before`/`::after` content; `innerText` never does | The pseudo-content **is painted** (Chrome computes the content and generates the box) — the *channel* under-reports paint; frot's inclusion is the `--css` contract (`VISION.md`, `css.md`). frot is right; innerText is the blind ruler. |
| `<select>` options: painted UI shows the selected option; `innerText` shows all options; frot shows all options | Both rulers differ from paint; comparing them to each other is stable. The *fusion* of option texts is a defect, not a residual (§12.3). |
| canvas bare-text fallback appears in no frot view; Chrome AX has `StaticText` under `Canvas` | frot's `ax` model has no text nodes, so bare text under `<canvas>` surfaces nowhere — a model consequence, uniform with `<p>` prose. Note: `layout.md` §2.1's "kept in ax" is thereby overstated for *bare text* — what survives is element fallback (measured: a link inside canvas appears; its sibling bare text does not). Fix the wording there when this lands. |
| `open-quote`/`close-quote` and PUA icon glyphs | frot emits the raw `` codepoint and no quote marks; innerText emits neither. Sub-token noise the token projection mostly absorbs; declared for the icon-font case. |

## 4. Corpus — same bytes, pinned captures

- **Fixtures are checked in, small, and network-free.** Two kinds: authored
  minimal fixtures, one per defect class (the §12 set: fallback, disclosure,
  hidden-attr, aria-hidden, generated content, tables/lists/selects, flex
  order, SVG text — i.e. the regression arena of `bl-eeb4`/`bl-74a6`/
  `bl-0f83`/`bl-0aaf`/`bl-d470`/`bl-c0a4`, pinned green forever); and saved
  real pages with subresources localized at save time, so both instruments
  read identical bytes with zero network. Fixtures and captures are excluded
  from the published crate exactly as the existing 656K of fixtures are
  (`bl-aa7d`); they cost the repo, not the artifact.
- **Captures are raw, projection is code.** The recorder stores Chrome's raw
  channel output (innerText string, `getFullAXTree` JSON, …) stamped with
  Chrome version, flags, viewport, and date. Projection happens at compare
  time, applied to both sides — so improving a projection never requires
  re-recording, and the capture stays a fact about Chrome, not about the
  comparator (single source of truth).
- **Staleness is a re-record, not a rot.** A capture does not go stale by
  sitting still — it is a true statement about a named Chrome version on a
  named date. It goes stale when Chrome's *behavior* changes, which is
  observed by deliberately re-recording (§7), and the diff between two
  captures of two Chrome versions is itself evidence.

### 4.1 The live half cannot compare content — measured

`bl-46f5`'s offline/live split is confirmed, with a correction its own axis
never hit: **the site's challenge layer serves the two probes different
pages.** Measured (§12.5): live `https://www.w3.org/` gives headless
Chrome a Cloudflare interstitial ("Performing security verification", Ray ID)
while frot receives the real page. A live content diff would report the
entire page as divergent, and the divergence would be about the *gate*, not
the views. So the live mode of this instrument records **outcomes and
captures for later fixture-ization** — it is the dated-evidence half, like
`ab_harness`'s field corpus — and content comparison happens only on the
same-bytes corpus. (The finding generalizes: any differential harness whose
two probes present different personas must not compare content fetched
separately.)

## 5. What it emits — a gate, and where the report went

The design question posed was gate vs. triaged report; the measured zero
noise floor (§3.2) dissolves the dilemma:

- **In CI it is a gate**: `cargo test` compares frot against the checked-in
  captures modulo the ledger. Green means "frot matches Chrome-as-recorded in
  every declared way and differs in every declared way." It cannot cry wolf
  (equality is the expected state), and it cannot be ignored (it is red CI).
- **The report exists only at record time.** Re-recording against a new
  Chrome, or recording a new fixture, surfaces every new delta as a failing
  comparison, and each one must be dispositioned before the capture can land:
  a fix (a filed ball), or a ledger row (a reviewed residual claim). There is
  no third bucket, no severity score, and no standing report to go unread —
  the "report" is a diff in a commit that cannot merge undispositioned.

This is `identity.md` §12's discipline transplanted: *a field is either an
exact match or a residual asserted against stable output; neither may drift
silently.*

## 6. Where it lives

- **Comparator: `tests/fidelity.rs`** (+ a small support module), driving the
  compiled binary over `file://` fixtures — the `tests/binary.rs` precedent.
  It is test code: nothing ships, no binary-size cost, no new `src/`
  surface, and the ≤300-line cap applies per file as usual. It runs in the
  default `cargo test` path with **zero network and zero Chrome**; a missing
  or unreadable capture is a **failure, never a skip** — the
  `h2_preface.rs` discipline: an oracle must not pass vacuously.
- **Recorder: `scripts/fidelity-record`** + `make fidelity-record`. It needs
  Chrome and CDP; Node ≥ 22's built-in WebSocket makes it a zero-dependency
  script (the §12 probe is the prototype), which beats adding a Rust
  websocket dev-dependency for a tool that runs at record time only. It
  refuses to run without Chrome rather than degrading.
- **Nothing in the shipped artifact, no cargo feature, no frot flag.** The
  harness consumes frot's public surface exactly as any caller does. If a
  comparison ever seems to need a debug hook or a new view, that is evidence
  about the view's design, not a license for a flag (I8).

## 7. The oracle is a version

Chrome 139 is not "the truth"; it is a named instrument. Every capture embeds
`{chrome, flags, viewport, date}`; the comparator asserts against the
capture, so **Chrome moving changes nothing until someone re-records** — a
deliberate, dated act, done when a defect investigation wants a fresher
oracle or on a cadence Mark chooses, not a treadmill CI imposes. When two
Chrome versions disagree, that diff is recorded evidence about Chrome (as the
persona pin in `identity.md` §2 treats Firefox releases), and the ledger says
which answer frot follows and why.

### 7.1 No second oracle

**Firefox is not added.** It would double capture, projection, and ledger
cost, and when the engines disagree the harness has no adjudicator — a
two-oracle harness converts every engine disagreement into a standing
three-way delta. The facts frot's views express (visibility, fallback
content-model, roles/names, token text) are the spec-convergent stratum where
engine disagreement is rare and newsworthy; when a specific capture is
suspected of pinning a Blink-ism, the right move is what `bl-e79a` did —
measure Firefox ad hoc, decide, record the decision in the ledger row. (That
frot *impersonates* Firefox is a transport fact, not a view fact; it creates
no obligation here.)

## 8. Invariants

- **F1 — same bytes.** Content is compared only when both instruments read
  identical bytes. No live URL's content is ever diffed (§4.1).
- **F2 — projection over tolerance.** A fact outside frot's declared view
  model or not invariant under the published approximation contract is
  excluded from the domain by the projection. There is no epsilon, threshold,
  score, or rank anywhere in the harness.
- **F3 — equality on the projection.** The expected divergence on every
  channel is zero; §3.2 is the measured basis.
- **F4 — total disposition.** Every delta becomes a filed defect or a ledger
  residual; the ledger asserts residuals in both directions, so none can
  drift silently.
- **F5 — no vacuous pass.** The comparator needs no network and no Chrome;
  a missing capture fails.
- **F6 — nothing ships.** Recorder, fixtures, captures, ledger: dev-tree
  only, excluded from the published crate.
- **F7 — the oracle is named.** A capture without its Chrome
  version/flags/viewport/date stamp is invalid.
- **F8 — public surface only.** The harness runs the frot binary as any
  caller would; it never grows frot a flag, view, or hook.

## 9. What is deliberately not built

Attacked before committing, per the house rule — each of these was considered
and is refused, not deferred:

- **Coordinate comparison, with or without tolerance.** §3.1. A tolerance is
  a muted alarm with extra steps.
- **Pixel/screenshot comparison.** VISION: pixels are not the output.
- **A similarity score or ranked report.** A score is a delta nobody has to
  disposition; F4 forbids the bucket it would live in.
- **A second browser oracle.** §7.1.
- **A live-site content-comparison mode.** §4.1 — measured unsound.
- **Chrome in CI, or as any build/test dependency.** §6.
- **A crawler/corpus-expander.** The corpus grows by defect class and by
  deliberate page saves, reviewed like code — sampling breadth is the *live*
  instrument's job (`ab_harness`), not this one's.
- **A `dom`-channel diff.** §2.3.

## 10. What this design does not solve

1. **Corpus blindness.** It sees nothing outside the corpus. What it adds
   over inspection is permanence — a class, once captured, is pinned forever
   — not coverage of the unbounded space.
2. **Adjudication.** A delta says "the sides differ", never who is right.
   §3.3's founding entries show both directions occur (frot right/channel
   blind; frot wrong/defect). Disposition stays human.
3. **Process-boundary defects.** The `bl-ae88` SIGABRT class belongs to the
   POSIX suite (`docs/design/posix.md`), which already owns
   one-envelope-always. This harness assumes an envelope exists.
4. **JS nondeterminism.** Only fixtures with deterministic scripts get a
   `--css --js` capture; pages whose DOM depends on `Date`/random/network
   timing are compared under `--css` only (the `ab_harness` parity rule).
5. **The channel's own blind spots.** innerText cannot see generated
   content; the AX channel cannot see what Blink chooses not to expose. The
   ledger names these; it cannot repair them.
6. **Fixture realism.** Localized saved pages drift from their origins the
   day they are saved. That is F1's price, paid knowingly; the live recorder
   exists to notice when reality has moved far enough to warrant a new save.

## 11. Falsification rules

> **Rule 1 — the zero floor.** If, as the corpus grows past the founding set,
> re-records keep producing deltas that are neither defects nor stable
> residuals — churn that equality cannot absorb and the ledger only buries —
> then §3's central claim is wrong for that channel, and the channel must be
> **narrowed or deleted, never thresholded**. A fidelity harness that needs a
> tolerance knob to stay green has already failed; muting it with math is the
> outcome this design exists to avoid.

> **Rule 2 — the ledger ratio.** If residual rows grow in proportion to
> fixtures (each new page needing bespoke exceptions), the projection is
> misdrawn — the model boundary belongs in the projection, not in per-fixture
> rows. Redraw the projection; a ledger that scales with the corpus is a
> tolerance knob wearing a paper trail.

> **Rule 3 — the instrument must catch its own founding class.** The first
> implementation sibling must demonstrate each founding fixture red against
> the pre-fix commit of its defect class (e.g. `<option>` fusion red before
> its fix, `bl-d470`'s `<details>` red at `4f68c48`). If the harness cannot
> re-detect the defects that motivated it, it is not the instrument this doc
> claims, and does not merge.

A null result is publishable here as everywhere in this repo (`identity.md`
§14): if the harness runs for a quarter and catches nothing the backlog
didn't already know, that is evidence about where defects come from, recorded
— not a reason to widen the predicate until it "finds" something.

## 12. Measurement session — 2026-08-12, Chrome 139.0.7258.138

Instrument: `--headless=new --no-sandbox --window-size=1280,720`, CDP over
Node 22's WebSocket (`Runtime.evaluate`, `Accessibility.getFullAXTree`);
frot built at `75b2d42` (post-`bl-d470`). Fixtures `file://`, real pages
saved and read as the same bytes by both sides.

1. **Generated content** (`.new::before{content:"NEW: "}`): frot `--css
   text` → `NEW: Fresh item`; innerText → `Fresh item`; Chrome computes the
   `::before` and paints it. → founding ledger row (channel blind, frot
   right). Icon-font PUA `\e609` emitted by frot, absent from innerText;
   `open-quote` pairs painted by Chrome, in neither text output.
2. **Formatting**: innerText emits `r1c1\tr1c2` per table row and `\n\n` at
   paragraph margins; frot one `\n` per block. Token projection: identical.
3. **Defect found by the instrument**: `<select><option>first</option>
   <option selected>second</option></select>` → frot text `firstsecond` —
   two words fused into a token no browser produces (Chrome: `first`,
   `second`). Surfaced by the token projection on the first fixture that
   contained a select. *Awaiting triage as a bug ball on sign-off — not filed
   from inside this design ball.*
4. **Geometry**: 13-element fixture (headings, wrap, flex `order`, float,
   `position:absolute`, sized broken `<img>`): all 13 frot rects differ from
   Chrome's on every axis (body margin 8, real font metrics 37px vs 20px
   h1). Ordinal x-order of flex items *matches* (visual b-then-a both
   sides); text token order identical on both sides including float and abs
   (both emit DOM order); abs breaks y-ordinal (declared non-goal). frot
   gives the attr-sized broken `<img>` a 0×0 box where Chrome gives 300×150
   — a paintedness-adjacent fact for the sibling to examine, noted not
   filed.
5. **The oracle is gated**: live `www.w3.org` → Chrome headless receives
   a Cloudflare interstitial; frot receives the page. Basis of F1/§4.1.
6. **Zero floor on real pages** (same bytes, `file://`): w3.org front page
   **521/521 tokens identical**; Wikipedia *Frottage (art)* **776/776**;
   `difflib` ratio 1.0 on both.
7. **AX raw vs projected**: fixture with details/canvas/video/nav/form/svg —
   Chrome raw ≈4× frot's node count, including UA-widget subtrees (play/
   mute/fullscreen buttons, `slider "video time scrubber"`, a link named
   `"Unable to play media."`) and `InlineTextBox`/`ListMarker`/`StaticText`
   layers. After projection: frot's tree matches, including the whole
   recently-fixed arena (closed `<details>` body absent, video fallback
   absent, `[hidden]` absent, `aria-hidden` pruned from ax while present in
   text, summary→button vs `DisclosureTriangle`). Canvas bare-text fallback:
   Chrome `Canvas → StaticText "canvas fallback words"`; frot: no node in
   any view (element fallback *is* kept: a link inside canvas appears) —
   founding ledger row + `layout.md` §2.1 wording fix.
8. **Links channel shape**: saved Wikipedia page — frot `links` 195 entries
   (anchors + `<link rel=stylesheet>` + alternates…), `document.links` 163
   (anchors/areas only). Mapping = filter by kind, then compare.

## 13. Decomposition — proposed siblings (NOT filed; after sign-off only)

1. **Recorder + capture format** — `scripts/fidelity-record`, raw-channel
   captures with the F7 stamp; founding fixture set from §12. *Dep: none.*
2. **Text-channel comparator** — token projection, ledger format, the
   founding ledger rows; Rule 3 demonstration against `4f68c48`. *Dep: 1.*
3. **AX-channel comparator** — the projection of §3.1 (prune/map/trim), role
   table. *Dep: 1.*
4. **links/forms comparators + paintedness channel** — the narrow remainder
   of §2.2. *Dep: 1.*
5. **Docs fidelity pass** — `layout.md` §2.1 bare-text wording; README dev
   docs for `make fidelity-record`; ARCHITECTURE test-oracle note. *Dep:
   2–4.*

Defects already in hand for triage at sign-off (not filed, per the ball's
non-goal): the `<option>` token fusion (§12.3); the attr-sized `<img>` 0×0
box question (§12.4).

## 14. Open questions (with recommended defaults)

- **OQ-1 — pin the recorder to a Chrome major?** Recommended: **no pin
  enforcement in the recorder** — it stamps whatever it ran (F7) and the
  captures are the pin. An enforced version check adds mechanism for a
  problem the stamp already makes visible in review.
- **OQ-2 — ordinal geometry (reading-order/containment) as a later
  channel?** Recommended: **not until a defect class demands it.** The text
  channel already carries order; the measured ordinal breaks sit exactly on
  declared non-goals, so the comparator would start life needing an
  exclusion table — mechanism ahead of evidence.
- **OQ-3 — should the live recorder auto-save gated/changed pages as new
  fixtures?** Recommended: **no** — saving is a reviewed, deliberate act
  (§9, no crawler). The live half reports; a human saves.
