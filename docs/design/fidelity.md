# View fidelity — a differential oracle against a browser

Living design for `bl-55cd`. **Status: BUILT (`bl-2e08`, signed off by Mark
2026-11-17).** The recorder is `scripts/fidelity-record`, the comparator
`tests/fidelity.rs`, the ledger `tests/fidelity/ledger.rs`, the corpus
`tests/fixtures/fidelity/`; §15 is the as-built record — what the first
recording found, and where building it corrected this document. It fits inside
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
| `text` | `document.body.innerText`, read after the recorder has put the two things Chrome paints and `innerText` omits — string `::before`/`::after` content and `<textarea>` values — into the tree as text where they paint (§15.1) | whitespace-collapsed token sequence | **zero** (measured: 521/521 and 776/776 tokens on two real pages) |
| `ax` | `Accessibility.getFullAXTree` | roled-node tree in frot's model: prune `ignored`/`InlineTextBox`/`StaticText`/`ListMarker`/`generic`/UA-widget subtrees; map Blink role names; trim names | zero after projection (measured on the §12 fixture set) |
| `links` | `document.links` | filter frot's `links` to anchor kind (frot's view also carries `<link>`/`<area>`/forms of reference `document.links` lacks — measured 195 vs 163 on one page); compare resolved `href` + text in document order | zero |
| `forms` | `document.forms` + `form.elements` | field list per form: tag/type/name/action/method, over the four control tags the view declares (`input`/`textarea`/`select`/`button`), `type` as the attribute is written | zero (measured, §15) |
| `bboxes` | `checkVisibility() && getClientRects().length > 0` per element (§15.1) | **box vs no box only**; never a coordinate | zero for flow content; a collapsed select's options, once the one ledger residual here, have no box on either side since `bl-2932` (`layout.md` §2.4) |

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
  text-granularity nodes with their subtrees; *splice out* — remove the node,
  keep its children — every `ignored` node, every `generic` wrapper, and
  every node inside a user-agent shadow root; map Blink's role vocabulary
  — `DisclosureTriangle`→`button`, `SvgRoot`→`graphics-document`, … — one
  finite table in the comparator; trim name whitespace). What survives is
  exactly what frot claims to produce, and on that, equality. (As built the
  UA widgetry is spliced, not pruned by subtree as first written here: a
  collapsed select's `MenuListPopup` is a UA-shadow node whose children are
  the page's own `<option>`s. Which nodes are UA-shadow is itself recorded —
  the capture's `ua_shadow` list, from `DOM.getDocument` — not guessed from
  roles.)

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
citation. A row is one of exactly two kinds: a **residual** (the reason), or
an open **defect** (the ball that will fix it) — the second kind is how a
divergence found at record time is pinned while its fix is still a filed
ball, so the gate is green on "frot differs here, and it is filed" and goes
red the moment the fix lands, until the row is deleted. The comparator applies the ledger *after* projection and fails in
both directions — an undeclared delta fails, and a declared delta that stops
appearing fails too (the residual's assertion is that it still differs in the
declared way). A residual can therefore never drift silently, and the ledger
is reviewed like code because it is code.

The founding entries, all measured (§12). As built (§15), the first, third
and fifth were ledger rows; the second and fourth produce no delta at all —
both rulers list every option, and Chrome's `StaticText` under `Canvas` is
text granularity the `ax` projection already drops — so they are facts about
the projection and need no row. The first and fifth were a blind *ruler*, not
a fact about frot, and a blind ruler is repaired, not declared: since
`bl-7966` (§15.1) the recorder reads generated strings from Chrome, both rows
are gone, and those tokens are compared like any other. The third went with
`bl-2932` (2026-11-20), which made it true of frot instead of declaring it:

| residual | reason |
|---|---|
| frot `text` includes string `::before`/`::after` content; `innerText` never does | The pseudo-content **is painted** (Chrome computes the content and generates the box) — the *channel* under-reported paint; frot's inclusion is the `--css` contract (`VISION.md`, `css.md`). **No longer a row** (§15.1): the recorder now reads the computed string from Chrome, and the two sides are equal. |
| `<select>` options: painted UI shows the selected option; `innerText` shows all options; frot shows all options | Both rulers differ from paint; comparing them to each other is stable. The *fusion* of option texts was a defect, not a residual (§12.3), and is fixed — `<option>`/`<optgroup>` are `BLOCK_TAGS` on their measured `display:block` (`bl-66ed`, `layout.md` §2.4). |
| ~~`bboxes`: a **collapsed** (`size=1`) select's options get a frot box; Chrome gives them zero client rects~~ | No longer a residual (`bl-2932`, 2026-11-20): frot models select modes, and a menu list's contents get no box (`layout.md` §2.4). Its 4 ledger rows are deleted. |
| canvas bare-text fallback appears in no frot view; Chrome AX has `StaticText` under `Canvas` | frot's `ax` model has no text nodes, so bare text under `<canvas>` surfaces nowhere — a model consequence, uniform with `<p>` prose. Note: `layout.md` §2.1's "kept in ax" is thereby overstated for *bare text* — what survives is element fallback (measured: a link inside canvas appears; its sibling bare text does not). Fix the wording there when this lands. |
| `open-quote`/`close-quote` and PUA icon glyphs | frot emits the raw `` codepoint and no quote marks; innerText emits neither. **The icon row is gone** (§15.1): Chrome's computed `content` *is* that codepoint and Chrome paints it, so frot emitting it is an equality, not a choice. Quote marks are still unseen by both sides — Chrome paints them, the recorder reads only strings, frot emits none — so they produce no delta and no row (§10.5). |

## 4. Corpus — same bytes, pinned captures

- **Fixtures are checked in, small, and network-free.** (As built: the
  authored pages at the top of `tests/fixtures/fidelity/`, and since `bl-5334`
  three saved real pages in `saved/`, each a licensing decision recorded in
  `tests/fixtures/NOTICE.md` §4 — see §15.2.) Two kinds: authored
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
  Chrome version, flags, viewport, and date, plus the sha256 of every file
  Chrome was served — the comparator re-hashes them, so F1 is checked, not
  assumed. Chrome does not read the fixture from `file://` as the §12 probe
  did: the recorder intercepts every request (CDP `Fetch`) and answers it
  from the corpus directory at a neutral base URL. A `file://` capture embeds
  the recording machine's paths in every resolved href and AX `url`; served
  this way a capture carries no local identity, and a page that asks for any
  byte outside the corpus is refused outright. `links`/`forms` therefore
  compare the *reference* — the URL relative to the corpus root, else to
  its origin (the root, not the page's own directory, since `bl-8682`: a page
  in `saved/` may link above itself) — which is the same on both sides. Projection happens at compare
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
  a filed ball with its defect row, or a reviewed residual row (§3.3). There is
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
5. **The channel's own blind spots.** The AX channel cannot see what Blink
   chooses not to expose. `innerText` could not see generated content; the
   recorder now reads the string part of it from Chrome (§15.1), and what is
   left unseen is named here because it produces no delta to hang a row on:
   `counter()`/`counters()`, `open-quote`/`close-quote`, images in `content`,
   `::marker`, and the painted value or placeholder of an `<input>`. Chrome
   paints all of these; frot's `text` emits none of them; the channel reports
   neither, so it says nothing either way.
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
> its fix, `bl-74a6`'s closed `<details>` red before its). If the harness cannot
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
   contained a select. Filed as `bl-66ed` and fixed there: `<option>` and
   `<optgroup>` compute `display: block` in Chrome in every select mode and
   outside a select entirely, so they joined `BLOCK_TAGS`. The same session
   measured two more facts in that subtree — `<datalist>` is a UA declaration
   (`layout.md` §2.3) and the collapsed select's absent option boxes are a
   residual (§3.3; modelled instead since `bl-2932`) — and left the `<optgroup label>` accessible name filed
   separately as `bl-4093`, since `innerText` carries neither attribute and the
   text channel already agreed. Fixed there: the `label` content attribute is a
   name-from-author source on both tags, two rows in `ax::name`'s existing
   native-name table (`layout.md` §2.4). No `text` output changed. Measuring
   *that* exposed a sixth fact one layer down and again only on the AX channel:
   `aria-labelledby` pointing straight at an element read its contents and never
   its own alternative, so a reference to an `<optgroup label>` named the
   referrer after the option inside it and a reference to an `<img alt>` named
   it nothing. Filed as `bl-0482` and fixed there — the cycle guard was seeded
   with the target and so doubled as "do not read this node's own alternative",
   two rules on one mechanism; splitting them leaves three doors on one
   recursion (`layout.md` §2.4). 46 reference/label/legend cases were measured
   for it and frot then matched Chrome on 40; the six that remained were all
   one fact — `title` as a name source inside those traversals — filed as
   `bl-d8ff` rather than folded in, and fixed there against 33 further cases.
   That seventh fact is the one the spec reads wrong: accname puts §2I at every
   node, and Chrome does not. `title` is read where the node's *own* name is at
   stake (the element being named, and a `<label>`/`<legend>` standing in that
   place) and everywhere inside a reference, never at a descendant read only
   for its contribution — so `<div role=button>A<span title=T></span>B</div>`
   is "AB". Three positional rules, no two agreeing, now share one four-state
   axis (`layout.md` §2.4). All 79 cases match; the AX channel has no measured
   name divergence left.
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

## 13. Decomposition — as proposed (built as one ball, `bl-2e08`)

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

## 15. As built — 2026-11-18, Chrome 156.0.8078.73 (`bl-2e08`)

**The instrument.** `scripts/fidelity-record` (Node ≥ 22, no dependencies;
`make fidelity-record`) launches a local headless Chrome, serves each page of
`tests/fixtures/fidelity/` to it over CDP `Fetch` at a neutral base URL,
advances Chrome's virtual clock by frot's own 10 s `--js` horizon, and writes
`<page>.json` beside `<page>.html`: the stamp (`chrome`, `flags`, `viewport`,
`date`, `base`, `recipe`, `bytes` — the sha256 of every file served) and the
raw channels (`text`, `links`, `forms`, `boxes`, `ax`, `ua_shadow`). The
recipe is derived, not chosen: a page with a `<script` is compared under
`--css --js`, any other under `--css` (§2.1). `tests/fidelity.rs` runs the
compiled binary over the same files, projects both sides to a sequence of
lines per channel, and requires the difference to be exactly the ledger's
rows. A page with no capture, a capture with no page, an incomplete stamp, a
viewport other than frot's 1280×720, or bytes that no longer hash to the
stamp each fail.

**The corpus.** Ten authored pages, one per defect class: `fallback`,
`disclosure`, `hidden`, `generated`, `structure` (tables, lists, selects),
`flex`, `svg`, `forms` (forms and links), `article` (landmarks, an external
stylesheet), `script` (a deterministic DOM build with a timer; `--css --js`).
Added since (2026-11-20, Chrome 158): `select` (select modes, `bl-2932`) and
`names` (accessible-name and role edge cases, one section per ball:
`bl-fb69` separators, `bl-07bd` SVG, `bl-2b09` table summary, `bl-fe88` empty
caption, `bl-c5a6` unnamed form/region roles, `bl-0794` titled term).

**What the first recording found.** 51 ledger rows over 10 pages × 5 channels.
`links` and `forms`: zero deltas. `text`: zero deltas outside the two
blind-channel residuals. `bboxes`: zero outside two residuals. `ax`: seven
divergences, each filed and pinned as a defect row (a ball marked *fixed*
has since landed and taken its rows out of the ledger) —

| ball | divergence (frot `--css --out ax` vs Chrome 156) |
|---|---|
| `bl-fd85` (fixed) | a `<table>` is not named from its `<caption>` |
| `bl-e760` (fixed) | `<tbody>` emits a `rowgroup`; Chrome ignores tbody and keeps only `<thead>` |
| `bl-3072` (fixed) | a `row` is named from its cells, fused (`"Q110"`); Chrome names it `""` |
| `bl-8ad7` (fixed) | `<dt>` (`term`) has no name; Chrome names it from contents |
| `bl-0a0a` (fixed) | an inline `<svg>` is not named from its `<title>` |
| `bl-2a3c` (fixed) | an unnamed `<form>` is omitted; Chrome exposes `form ""` |
| `bl-7430` (fixed) | flex `order`/`row-reverse` reorder AX children (`layout.md` §5); Chrome's AX tree stays in DOM order |

Two residuals joined the founding ones, both the channel's blindness rather
than frot's: `innerText` omits a `<textarea>`'s painted default value, and
`getClientRects()` still answers for the body of a closed `<details>` —
Chrome hides it with `content-visibility: hidden`, which skips paint but
keeps the boxes. The second means §2.2's observable is not paintedness
everywhere; if rows of that kind grow with the corpus, Rule 2 says redraw the
observable (`checkVisibility()` is the candidate), not the ledger.

**Rule 3, demonstrated.** Today's comparator, corpus and ledger were run
against frot rebuilt at the commit before each founding fix. All red, each on
its own class:

| frot at | before the fix for | red on (among others) |
|---|---|---|
| `f7281e3` | `bl-66ed` option fusion | `structure [text]` `- first` `- second` `+ firstsecond` |
| `4cd5dd7` | `bl-c0a4` SVG text | `svg [text]` `- svg` `- label` `- words` |
| `1d0e74e` | `bl-0f83` video fallback | `fallback [text]` `+ video` `+ fallback` `+ words`; `fallback [ax]` `+ link "download the clip"` |
| `b1d3673` | `bl-74a6` closed `<details>` | `disclosure [text]` `+ Closed` `+ body` `+ text` …; `disclosure [ax]` `+ link "closed link"` |
| `b10171c` | `bl-eeb4` `hidden` | `hidden [text]` `+ Hidden` `+ attribute` `+ text` …; `hidden [bboxes]` `+ 8 div` |

It also re-detected classes nobody asked it about: every one of those builds
put the `<title>` into `text`, and the first named an `<optgroup>` `""`
(`bl-4093`). And it corrected Rule 3's own example: `4f68c48` is **not** red
on closed `<details>` — `bl-d470` was a defect of the recipe *without* `--css`,
which §2.1 already puts outside the comparable domain. The `--css` class is
`bl-74a6`, and the rule now names it.

### 15.1 The blind channels made to see — 2026-11-20, Chrome 158.0.8127.36 (`bl-7966`)

Nine of the residual rows above said "frot is right, the ruler is blind". A
row like that asserts only that frot still emits *something* there; it cannot
catch frot emitting the wrong thing. So the ruler was repaired and the rows
deleted. All ten pages were re-recorded on Chrome 158; no new delta appeared.

**`text`.** After every other channel has been read, the recorder's text probe
(`scripts/fidelity-probes.js`) asks Chrome for the computed `content` of every
element's `::before` and `::after` and for every `<textarea>`'s value, inserts
each as a real element where it paints (first child, last child, just before
the control) carrying the computed `display`, `visibility` and
`text-transform` of what it stands for, and reads `innerText` once. Measured
on a scratch page: `attr()` arrives already resolved to a string; a
`display:block` pseudo breaks the line; content under `display:none` or
`visibility:hidden` stays out; `url()`, `counter()` and the alternative text
after `/` are not taken. The capture's `text` is therefore no longer the bare
`innerText` — it is `innerText` of the page with its generated strings made
real. The insertion can in principle disturb a structural selector
(`:empty`, `:first-child`) on the element it lands in; nothing in the corpus
does, and a page where it did would show up as a delta, not hide one.

The seven rows that became equalities: `article` `By`; `generated` `NEW:`
(twice), `(done)` and the private-use icon glyph U+E609; `forms` `draft`
`text` (the textarea). The icon glyph needed no decision: Chrome's computed
`content` is that codepoint and Chrome paints it, so both sides carry it.

**`bboxes`.** Measured, per element, `getClientRects().length` against
`checkVisibility()` (default options):

| case | client rects | `checkVisibility()` |
|---|---|---|
| body of a closed `<details>`; content under `hidden=until-found` | 1 | false |
| `display:none`, `[hidden]`, `display:contents`, media/canvas fallback, a collapsed select's options | 0 | false |
| `visibility:hidden` | 1 | true (false only with `visibilityProperty: true`) |
| zero-size box, empty block, `opacity:0`, off-screen | 1 | true |
| SVG `<defs>` and its children, `<wbr>` | 0 | true |

Neither answer alone is "has a box"; their conjunction is, and that is what
the recorder now stores in `boxes[].rects` (the rect count where
`checkVisibility()` holds, else 0). `visibilityProperty` is deliberately off:
frot's `bboxes` lists a `visibility:hidden` element because it occupies space
(`layout.md` §7), and so does Chrome. The two closed-`<details>` rows became
equalities; the channel's name for what it compares is now "box", not
"paintedness".

**What is left.** After `bl-7966`: four ledger rows, all one residual, the
collapsed select's options. `bl-2932` (2026-11-20) modelled select modes and
deleted them; its new `select` page pinned one open defect row instead
(`bl-240d`: Chrome's `innerText` lists a `hidden` option, frot's `text` does
not). No row is a blind ruler any more.

### 15.2 Saved real pages — 2026-11-21, Chrome 158.0.8127.36 (`bl-5334`)

**Layout.** A saved page is `tests/fixtures/fidelity/saved/<name>.html`, its
stylesheets in `saved/<name>/`, its capture `saved/<name>.json`. The recorder
and the comparator list the corpus as the top directory plus `saved/`; a
fixture's name in the ledger is its path below the corpus (`saved/<name>`).
Nothing else changed: same recorder, same five channels, same equality.

**What "saved" means here.** Scripts are removed, stylesheet links point at
the saved copies, and every image, icon and font reference is replaced by one
empty inline `data:` SVG — so Chrome asks for nothing outside the corpus and
no image is redistributed, while every element, `alt` and size stays. The
edits are listed per page in `tests/fixtures/NOTICE.md` §4. There is no save
tool in the tree (§9, no crawler): a page is saved by hand, once, and reviewed.

**The pages, and what each channel found** (rows = ledger rows, all defects):

| page | licence | size seen by Chrome | text | ax | links | forms | bboxes |
|---|---|---|---|---|---|---|---|
| `saved/go-getting-started` (go.dev tutorial) | CC BY 4.0; stylesheet BSD-3-Clause | 1022 tokens, 399 elements, 105 links | 0 | 0 | 0 | 0 | 0 |
| `saved/bootstrap-forms` (Bootstrap 5.3 docs, Forms overview) | CC BY 3.0; `bootstrap.min.css` MIT | 1005 tokens, 1291 elements, 172 links, 3 forms | 6 | 2 | 0 | 0 | 1 |
| `saved/wikipedia-brass-rubbing` (English Wikipedia article body) | CC BY-SA 4.0 | 483 tokens, 157 elements, 39 links | 3 | 13 | 0 | 0 | 0 |

`links` and `forms` are exact on all three. The Go page is exact on every
channel — 1022 of 1022 tokens, 288 of 288 boxes — with a 114 KB real
stylesheet applied.

**Every divergence is a ball.** Ten root causes, each with a minimal repro in
an authored page (`reach.html` for the stylesheet ones, `roles.html` for the
`ax` ones) so the class stays pinned when a real page changes:

| ball | divergence (frot `--css` vs Chrome 158) | seen on |
|---|---|---|
| `bl-3aef` | `@import` is skipped: rules in an imported sheet never apply | Python docs (left out) |
| `bl-011e` | `:not()` / `:is()` unsupported: a rule that *reveals* content never applies, so the content stays hidden | Bootstrap (`Search docs…` label and its box) |
| `bl-6363` | sibling combinators `+` / `~` unsupported | W3C spec (left out) |
| `bl-e987` | `:first-child` / `:last-child` unsupported: list separators come out wrong | Wikipedia (trailing `·`) |
| `bl-e48d` | `text-transform` not applied to `text` or accessible names | Bootstrap (`HTML`) |
| `bl-346e` | an absolutely positioned box fuses with its neighbours in `text` | W3C spec (left out) |
| `bl-fc83` (fixed) | DPUB-ARIA roles (`doc-noteref` …) ignored | Python docs (left out) |
| `bl-2ea9` (fixed) | a `role=presentation` table keeps its rows and cells | Wikipedia (message box) |
| `bl-b124` | a `<style>` in the body is read into an accessible name | a larger Wikipedia article (left out) |
| `bl-3bd4` (fixed) | a no-break space in a name becomes a plain space | Bootstrap, Wikipedia |

`bl-011e` contradicts a claim in `css.md` ("an unsupported rule can only
cause under-application, which leaves content visible"): under-applying a
rule that *reveals* hides.

**One projection fix, no residual.** Chrome's `Abbr` — its internal role for
`<abbr title>` — joined the roles the `ax` projection splices: HTML-AAM gives
`<abbr>` no role, so frot's vocabulary has no word for it. No residual row
was added: all 68 new rows are defect rows, 25 of them on the two real
pages that differ and 43 on the two authored repro pages.

**Two pages were tried and left out** (the rule of `bl-5334`: a page whose
rows come in hundreds from features frot's stylesheet subset does not have
drowns the ledger). A Python docs page differed on about 100 lines, about 90
of them one cause (`bl-3aef`: Sphinx's `classic.css` is a single `@import`);
with the imported sheet linked directly it differed on 10. A W3C
Recommendation (Web Share) differed on about 270: `bl-011e` with `bl-6363`
(every section's `§`), `bl-346e` (a type tooltip on every `<var>`) and
`bl-e48d`. Both have clean licences and are filed to return when those balls
land (`bl-8682`). This is Rule 2 read honestly: the count scales with how
often a page repeats one unsupported feature, not with the number of pages,
so the remedy is the feature (or a decision that it is outside the model),
not rows.

### 15.3 The four stylesheet gaps closed, and one page back — 2026-11-21, Chrome 158.0.8127.36 (`bl-8682`)

`bl-3aef` (`@import`), `bl-011e` (`:is()` / `:where()` / `:not()`), `bl-6363`
(`+` / `~`) and `bl-e987` (`:first-child` / `:last-child`) landed, each
deleting its own rows. The two pages §15.2 left out were measured again.

**The Python docs page is in** (`saved/python-webbrowser`, the `webbrowser`
module page with its six Sphinx sheets, one reached only through `@import`).
It matches Chrome on every channel with no ledger row: 1263 text tokens, 776
boxes, 71 links, 3 forms, and the `ax` outline. The 10 DPUB-role lines §15.2
predicted went with `bl-fc83`, which landed in between. One harness change
came with it: `links` and `forms` compare references relative to the corpus
root rather than the page's directory (§4), because this page points above
itself (`../genindex.html`) and the two sides resolved that to different
absolute paths — a difference of the harness's making, not frot's.

**The W3C page is left out again**, by the same rule. It now differs on 223
lines (text 191, ax 32; links, forms and bboxes exact): about 150 are `bl-346e`
(the type tooltip on every `<var>`), about 60 are `bl-e48d` (`text-transform`),
and with both declarations stripped as a probe 12 remain — 9 `bl-fe2c` (a
`display:block` span) and 3 a new one, `bl-788d` (`<details>` / `<summary>` do
not break the line in `text`; pinned by three lines added to
`disclosure.html` and nine defect rows). It is filed as `bl-3248`, blocked on
the first two.

**Also learned.** English Wikipedia's current parser stamps every page with a
four-part version number that the repo's leak scanner reads as an address;
the article was saved through the legacy parser instead (`NOTICE.md` §4)
rather than touching the rule.

