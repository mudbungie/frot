# Needs — capability-gap detection and honest reporting

Design landed 2026-07-19 (bl-6d75, bl-e22e; supersedes the bl-6bb5 element
guard). This is the working-out of VISION principle 5:

> When a page can't be rendered faithfully under the current capability
> recipe, return a clear, structured `needs-X` signal. Never silently
> produce a degraded result that looks complete.

## 1. The contract

`status:"needs"` with `needs:[...]` means: **this recipe cannot render this
page.** Each member names the missing capability. `js` and `css` are recipe
flags the caller can add; `human` is the capability past the end of the flag
list — no frot recipe will render this, so the honest advice is "escalate out
of frot".

> **`human` redefined 2026-07-22 (`bl-017a`) — the old wording was itself a false
> signal.** This paragraph previously read: *"`human` is the capability past the
> end of the flag list — **the page demands interactive verification that no frot
> recipe will ever provide**, so the honest advice is 'escalate out of frot'."*
> **The ball's argument is confirmed, with one correction.** For Amazon's AWS WAF
> challenge (§3, `identity.md` §3.9) *both halves of that sentence are false*: no
> interactive verification is demanded — a human browsing there passes it with
> zero involvement and never learns it happened — and a recipe change *does* help.
> Telling a caller "escalate to a person" about a gate a browser clears silently
> is exactly the class of defect VISION principle 5 forbids, one layer above the
> one `bl-7e34` just fixed.
>
> The correction to the framing: the *status* was never a lie — "this recipe
> cannot render this page" was true — the **taxonomy member** was. So `human` is
> not deleted or replaced; it is narrowed to what it can honestly carry:
>
> **`human` = no recipe frot offers renders this, *and frot has already tried the
> automatable route*.** It no longer asserts that a person is required before
> anything was attempted. Concretely (`challenge.md` §5.2): a declared challenge
> with `--js` **off** is `needs:["js"]`, because adding the flag genuinely helps;
> with `--js` on and the challenge unpassed it is `needs:["human"]` — the same
> spelling, now **earned**, with `challenge:{passed:false}` as the evidence. No
> new `needs` kind and no new envelope shape, the same result `bl-7e34` reached.
> *(Proposal awaiting Mark's sign-off; the shipped behaviour is the superseded
> wording until the challenge design lands.)*

`needs` is not an error. The transport succeeded and frot did its job; what
it refuses to do is present a placeholder as the page. Exit code stays 0,
the `http` block still reports the raw transport outcome.

## 2. Two detectors, one taxonomy

The 2026-07-19 field trial surfaced two false-`ok` classes with the same
shape — a body that is structurally present but carries no impression of
the real page:

- reddit's bot-challenge interstitial served at HTTP **200** (bl-6d75), and
- telegram's 49-empty-element SPA scaffold with zero text (bl-e22e).

They resolve to the same *reporting* (`status:"needs"`) but rest on facts
different in kind, and neither detector can substitute for the other:

| | fact | detector | signal |
|---|---|---|---|
| bl-6d75 | **transport**: the server declared the response a stand-in | response headers, pre-parse | `needs:["human"]` |
| bl-e22e | **document**: the post-recipe DOM carries nothing renderable | DOM walk, post-JS | `needs:["js"]` |

The reddit interstitial *has* text ("Reddit - Please wait for
verification"), so no content test catches it without a magic length
threshold — exactly the fragile string-adjacent classifier this design
refuses. The telegram scaffold sends no deferral header, so no transport
test catches it. One mechanism for both would have to be a body-shape
classifier of challenge pages, which is both fragile and the first step
down the evasion road. Two detectors, one honest spelling family.

## 3. Transport: declared deferral → `needs:["human"]` (bl-6d75)

A challenge interstitial is not sniffed from body copy. It is recognized
only when **the server itself declares the response is not the resource**:

- **`Retry-After` on a success (< 400) response.** RFC 9110 §10.2.3
  defines `Retry-After` for 503 and 3xx responses; on a 2xx it is the
  server saying "this body is a stand-in, come back". reddit's challenge
  answers `200` + `retry-after: 0` (observed 2026-07-19). Any value
  counts; the declaration is the header's presence.
- **`cf-mitigated: challenge`** — Cloudflare's documented marker for a
  challenge response, honored at any status < 400 (at ≥ 400 the existing
  error flip already reports honestly).
- **`x-amzn-waf-action: challenge`** (added `bl-7e34`, 2026-07-22) — AWS
  WAF's documented marker, the same declaration in a different vendor's
  spelling. Measured on `www.amazon.com/`: HTTP **202** +
  `x-amzn-waf-action: challenge` + `server: CloudFront`, body an AWS WAF
  `challenge.js` loader (`window.gokuProps`) carrying no rendered text.
  Both vendors reuse their header for non-deferral actions (`block`,
  `count`, `captcha`), so **only the literal `challenge` value declares**;
  an unmeasured value is not given an invented verdict.

  This is the fix for the "Amazon 202 is mislabelled `needs:["js"]`" defect
  (`identity.md` §15 item 2). It needed **no new `needs` kind and no new
  envelope shape**: the premise that the 202 was an *undiagnosable* withheld
  body was simply wrong — the server declares the reason in a header, in
  exactly the shape §3 already recognizes. Before the fix the starvation
  detector (§4) saw a text-free body with scripts and said `needs:["js"]`,
  which is a lie — no recipe change renders a page the origin refused to
  serve. `human` subsumes, per the precedence rule below.

Semantics:

- **Fires pre-parse and view-independent**, mirroring the `status >= 400`
  flip beside it in `run.rs`: a `--out dom` of an interstitial is an
  impression of the placeholder presented as the page, so every view
  flips. No `out`, no `js` block — nothing ran.
- **frot never executes a declared challenge's scripts.** Stopping at the
  declaration is the refused boundary (VISION "What this is not",
  `js.md` §11) enforced in code: detection is refusal to pretend, not a
  step toward evasion. If that boundary ever moves, it moves in its own
  ball — not by this detector growing capabilities.

  > **Superseded 2026-07-22 (`bl-017a`) — that ball was filed, and this is it.**
  > The bullet above is kept verbatim as the shipped behaviour and as the
  > superseded line. Under `docs/design/challenge.md` (**proposal, awaiting
  > sign-off**) the sequence becomes: a declared challenge with `--js` off →
  > `needs:["js"]`; with `--js` on → the page's *own* challenge script runs in the
  > existing sandbox, and **iff it changed the request frot would now send** (the
  > per-invocation cookie jar differs — `bl-6dad`) the original navigation is
  > re-issued, at most `CHALLENGE_ROUNDS = 1` time. Success is `ok` plus an
  > additive `challenge` block; failure is `needs:["human"]` plus
  > `challenge:{passed:false}` — earned rather than assumed.
  >
  > **This detector still does not grow capabilities**, and that constraint is
  > respected exactly as written: §3's job is unchanged — recognize a *server's
  > own declaration*, from headers, pre-parse. What changes is only what `run.rs`
  > does *with* that verdict. The declaration remains the sole recognition
  > mechanism, and no body-copy or vendor-algorithm knowledge enters here or
  > anywhere else.
  >
  > **Why this is not the retry loop `identity.md` §10 refuses:** a round trip
  > answers a challenge the server posed; a retry loop re-asks an unchanged
  > question. The jar-delta precondition makes that structural — an identical
  > request is never re-sent, which excludes retry-until-allowed *and* UA/IP
  > rotation by construction (`challenge.md` §4.1).
- **Precedence over starvation**: a challenge page may also be starved,
  but `needs:["js"]` would be a lie there — no recipe change helps a page
  the origin refused to serve. `human` subsumes.

Why `needs:["human"]` and not `error.kind:"http.challenge"`:

- Nothing failed. The transport succeeded and a well-formed document
  arrived; an `error` envelope would misstate the operation.
- `error.kind` HTTP kinds mirror real status codes (`http.404`);
  `http.challenge` would break that convention for a response whose code
  was 200.
- The honest statement is "this recipe cannot render this page" — the
  `needs` contract verbatim. A challenge arriving at ≥ 400 keeps its
  existing honest spelling, `error.kind:"http.<code>"`.

Accepted residuals, stated rather than papered over:

- A server that sends `Retry-After` with a genuine 2xx page is out of
  spec usage and gets flagged; false `needs` is the cheap direction
  (bl-6bb5's policy: false `ok` is worse, the primary consumer can always
  escalate and look).
- An undeclared 200-challenge with no deferral header falls through to
  the starvation detector if its body is starved, and otherwise passes as
  `ok`. That residual is the price of refusing a body-copy classifier.

**One capture, surfaced (`bl-acec`).** `challenge` reads
`FetchResult::headers`; the envelope's `http.headers`
(`src/envelope/http.rs`) exposes an allowlisted slice of *that same*
capture, so the evidence behind this signal is observable and the two
cannot disagree — the declaring header (`retry-after` / `cf-mitigated`) is
inside the allowlist, so it is always surfaced. The allowlist is bounded to
the headers a caller or `needs` uses to tell a bot-defence refusal from a
genuine response: `retry-after`, `cf-mitigated`, `server`, `x-datadome`,
`x-amzn-waf-action`, `content-type` (provenance: `identity.md` §3.7 —
reddit's `server: snooserv`, g2's `x-datadome: protected` + `server:
cloudflare`, Amazon's `x-amzn-waf-action: challenge`).
`set-cookie` (cookie jar's, `bl-6dad`) and volatile per-request headers
(`date`, request/trace ids) are excluded so goldens stay deterministic
(`identity.md` §12). This surfaces the evidence; it does not change the
`needs` verdict. The reclassification it unblocked landed separately
(`bl-7e34`): Amazon's 202 is now `needs:["human"]`, keyed on the
`x-amzn-waf-action` declaration above, not on `server: CloudFront` — a CDN
name is not a refusal, and keying off one would flag every genuine page
CloudFront serves.

## 4. Document: content starvation → `needs:["js"]` (bl-e22e)

Supersedes bl-6bb5's `no text AND ≤ 3 elements` guard. The element count
was a proxy for the real fact and the proxy leaked: a lone mount `<div>`
and telegram's 49-element scaffold are the *same fact* — nothing
renderable — differing only in how much furniture the shell ships. Test
the fact directly:

One walk over the body with chrome (`header`/`footer`/`nav`/`aside` — see
the scope rule below) and never-rendered subtrees
(`script`/`style`/`noscript`/`template`, plus anything this recipe does not
render) set aside collects two content signals:

- **text** — any non-whitespace text node;
- **label** — any element carrying a non-empty `alt` or `aria-label`:
  content a non-text view can express without text.

Starvation is **view-sensitive**, because views differ in what they can
carry:

- `text` is starved without **text** (`alt`/`aria-label` never reach the
  text view);
- `ax` / `links` / `forms` are starved without **text or label**.

Preconditions unchanged from bl-6bb5: scripts must be present, and the
check re-runs on the post-settle DOM under `--js`, so `needs:["js"]`
means "needs more JS than the shim gives" (VISION Phase 4). There is no
element count.

### 4.1 Non-rendered text is not content (`bl-eeb4`, 2026-08-11)

Two live shells returned `ok` on an empty mount because fallback copy no
browser paints was counted as body content: a deploy console's `<div hidden>`
holding "Opens in a new tab", and a task app's CSS-hidden "could not load the
required files" panel. Both are the false-`ok` direction §1 refuses, from one
missing invariant — the walk had no notion of *rendered*.

The invariant has one home and every consumer reads it there. The HTML UA
rule `[hidden] { display: none }` (author-overridable) now lives in the
cascade's UA-implicit `display` step (`layout.md` §2), which is what already
feeds `--css` text, the AX tree and geometry — so those three drop hidden
subtrees without knowing this rule exists. The starvation walk asks the same
question, against whatever oracle the **recipe** provides: under `--css` the
cascade (author rules, inline `style=`, and the UA rule folded into one
`display`); without it, the document's own `hidden` attribute, the only
visibility fact a recipe with no CSS has. The raw no-`css` views are
unchanged: `--css` still means "apply author CSS", and a `text` dump without
it stays a source-order dump.

**Chrome is scoped, not tag-named.** Excluding every `<header>`/`<footer>`
swallowed rendered apps: TodoMVC live renders `<section id=root><header><h1>
todos</h1>…</header></section>` and parks its `<main>`/`<footer>` behind
`hidden` until a todo exists, so with hidden text no longer propping it up the
page read as starved — a *false* `needs`, the very regression `bl-3a36`
fixed. HTML-AAM already draws the line: `<header>` is the page `banner` and
`<footer>` the `contentinfo` **only when no sectioning element**
(`article`/`aside`/`main`/`nav`/`section`) scopes it; a scoped one is that
section's own heading or byline, i.e. content. So `header`/`footer` are chrome
at page scope only, while `nav`/`aside` (landmarks wherever they sit) are
chrome unconditionally. The canvas-shell fixture — a body-level `<header>`
masthead over an empty `#root` — still flags, unchanged.

### 4.2 Concealed by a parent, not by itself (`bl-74a6`, 2026-08-11)

§4.1's oracle answers "is this node rendered", and `[hidden]` was only the
half of that question a node answers about *itself*. The other half is
structural: a `<details>` without `open` renders its first `<summary>` and
nothing else, so its disclosure body is copy no browser paints until someone
clicks — and frot never clicks (`js.md` §11, non-goals). Live repro
2026-08-11: the GOV.UK Design System's `/components/details/` page emitted
both closed disclosure bodies under `--css --out text`, and `--out bboxes`
emitted their paragraphs as zero-sized entries. A zero-sized entry is not an
omission: `bboxes` is defined as one entry per *rendered* element.

Same shape as §4.1, so the same two seams and no third: the fact has one home
(`dom::Document::concealed`), the cascade folds it into `display`
(`layout.md` §2) and every `--css` consumer inherits it, and this walk asks the
same question of whichever oracle the recipe has. Two differences from
`[hidden]` are load-bearing and both are properties of *where the skipped box
is*, not of this detector: it is **not author-overridable** (the box belongs to
the `<details>`), and it reaches **text nodes**, since `<details><summary>Q
</summary>A</details>` gives the `A` no element to hang the rule on. The walk's
render check therefore sits above the element/text split rather than inside the
element arm.

**A second parent rule, and the one line that separates them (`bl-0f83`).**
A `<video>`/`<audio>` renders *none* of its children: HTML defines its contents
as **fallback** for a user agent that does not implement the element, not as
copy shown when playback is idle or a source fails. Live repro 2026-08-11:
`https://www.w3.org/` under `--css --out text` emitted "Sorry, your browser
does not support embedded videos", and the AX tree exposed it too, while Chrome
at 1280×720 paints a 500×281 replaced box and exposes neither — unchanged with
JavaScript off. frot implements the element and presents a Firefox persona, so
emitting its fallback describes a different page.

It rides `dom::Document::concealed` beside the `<details>` rule, but the fact
itself lives in `tags::renders_children`, and that placement is the difference:
media fallback is a **content model** fact, not a UA *stylesheet* one, so no
cascade is needed to see it and `--css` is not what makes it true. It therefore
also holds in the raw views, which read it in the recipe-independent skips they
already keep for `<script>`/`<iframe>`/`<svg>`. `[hidden]` and a closed
`<details>` are document *state* and stay cascade-gated; this is what the
markup means.

`<object>`/`<canvas>` fallback is deliberately **not** in that set: the same
argument looks like it should extend, but the ball asked for evidence rather
than assumption, and none was measured here (`bl-e79a`).

Accepted residuals:

- A zero-text image gallery (labeled images, scripts present) flags
  `needs-js` on the `text` view even though JS would not add text —
  persistent false `needs`, cheap direction, and the `ax` view reads it
  honestly via labels.
- A body of only unlabeled graphics flags on every content view. Such a
  page is equally invisible to the AX tree, so "no impression under this
  recipe" remains the honest report.

## 5. What this deliberately does not do

- No body string-matching ("Please wait…", vendor page titles). Copy
  changes per site and per locale; declarations and structure do not.
- No challenge solving, token computation, or form submission — the
  detector's whole job is to stop *earlier* than that.
  > **Amended 2026-07-22 (`bl-017a`), proposal awaiting sign-off.** Still true
  > **of this detector**, and that scoping is the whole point: §3 recognizes a
  > declaration and reports it; it computes nothing. What moved is elsewhere —
  > `run.rs` may now hand a declared challenge to the *page's own* script rather
  > than exiting pre-parse (`challenge.md` §4.2). frot still performs **no token
  > computation of its own** and ships **no vendor algorithm**: the page computes
  > its token, as it does in a browser. "No form submission" is likewise unmoved —
  > frot originates nothing; only a page-initiated `fetch`/XHR POST is permitted,
  > and the top-level navigation stays GET permanently (`challenge.md` §2–§3).
- No text-length thresholds. A 37-character page and a 37-kilobyte page
  are both either declared stand-ins or they are not.
- No new flags. Both signals ride the existing `status`/`needs` shape;
  the envelope grows one enum member (`human`), nothing else.
