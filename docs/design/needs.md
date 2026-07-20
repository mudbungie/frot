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
list — the page demands interactive verification that no frot recipe will
ever provide, so the honest advice is "escalate out of frot".

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

## 4. Document: content starvation → `needs:["js"]` (bl-e22e)

Supersedes bl-6bb5's `no text AND ≤ 3 elements` guard. The element count
was a proxy for the real fact and the proxy leaked: a lone mount `<div>`
and telegram's 49-element scaffold are the *same fact* — nothing
renderable — differing only in how much furniture the shell ships. Test
the fact directly:

One walk over the body with chrome (`header`/`footer`/`nav`/`aside`) and
never-rendered subtrees (`script`/`style`/`noscript`/`template`) set
aside collects two content signals:

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
- No text-length thresholds. A 37-character page and a 37-kilobyte page
  are both either declared stand-ins or they are not.
- No new flags. Both signals ride the existing `status`/`needs` shape;
  the envelope grows one enum member (`human`), nothing else.
