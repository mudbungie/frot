# Challenge — the third movement of the scope boundary

**Status: PROPOSAL, awaiting Mark's sign-off. No code has changed.** Design
`bl-017a`, 2026-07-22. Living authority for the boundary `identity.md` §10 draws;
that section, `js.md` §11, `needs.md` §3/§5 and `VISION.md` carry dated in-place
supersessions pointing here.

Trigger: Mark, 2026-07-22 — *"File the boundary movement ball. It's time for that
scope creep. The value of this project is in letting an agent quietly ask for 'the
content at this url, as a human browser would get it'. It's okay to do multiple
round trips to get there."* Evidence: `identity.md` §3.9 (`bl-7e34`) — Amazon's
202 is an AWS WAF **declared challenge** (`x-amzn-waf-action: challenge`,
`challenge.js` + `window.gokuProps` → token → `aws-waf-token` cookie), and passing
it needs the three things `identity.md` §10 refuses simultaneously: execute a
challenge, send a non-GET, re-issue the request.

---

## 1. The principle — and an attack on it before adopting it

### 1.1 Not "challenge execution is now allowed"

That framing is the mechanism, not the line, and it licenses far more than Mark
asked for. It is also *already incoherent with the shipped tool*: **frot executes
challenge scripts today.** `--js` runs whatever the page ships, and `bl-bd4e`
learned which surfaces Amazon's script probes **by running it**. Nothing in frot
stops at "a challenge is executing"; what stops is a header check bolted in front
of parse (`src/run.rs`, `needs::challenge` → `needs:["human"]` pre-parse) that
fires on a **declaration** — reddit's `retry-after`, Amazon's `x-amzn-waf-action`
— not on anything intrinsic to the work. The old line was drawn on *what the
server admitted*, not on *what frot does*.

### 1.2 The proposed line

> **frot does what the browser does unattended, and only as far as delivering the
> impression of the requested URL requires. It never does what the human does.**

Two clauses, deliberately:

- **Ceiling — automatable, not human-requiring.** A human browsing to
  `amazon.com` passes the WAF challenge with *zero* involvement; they never learn
  it happened. A CAPTCHA is the opposite — the browser cannot proceed without a
  person. That is a real, testable seam, and it is what *"as a human browser would
  get it"* names.
- **Floor — the frottage rule survives.** frot is still an impression of **one
  URL**. Everything it does must be in service of delivering that impression;
  frot originates nothing else.

### 1.3 Attacking it (`~/AGENTS.md`: *"Attack a design before committing it"*)

**A1 — "unattended browser" over-licenses.** An unattended browser also POSTs
analytics beacons, runs service workers, speculatively prefetches, and navigates
wherever script tells it. Under clause one alone, all of that is in. **This attack
lands**, and it is why the principle is *two* clauses: the second confines frot to
the requested URL's impression. A beacon serves the site, not the impression, so
it is out — not because beacons are forbidden, but because nothing licenses it.
The one-clause version of this principle is wrong and must not be quoted alone.

**A2 — "unattended" is not the same as "invisible to a person".** reCAPTCHA v3
and other score-based gates require no interaction at all. Under this line they
are **in**, and the doc says so rather than letting it be discovered later: a
human never sees them either, so the principle is consistent. What is out is not
"anti-bot vendors" but *interaction*.

**A3 — the ball's suggested defence of the behavioural-interstitial case.** Does
*"a human browser does not move the mouse by itself; the human does"* hold? **It
holds.** No browser dispatches `mousemove`/`pointerdown` absent a device; those
events exist only because a person acted. Synthesizing one is doing the human's
work, precisely. And it is already enforced: `js.md` §11's *"No interaction"* is
unchanged by this design — the only events frot dispatches remain the
`DOMContentLoaded`/`load` lifecycle pair. Note the correct handling of the
pass-through case: if a behavioural script passes on timers and passive telemetry
alone, frot passes it, and that is fine — frot synthesized nothing.

**A4 — does this make frot a browser?** VISION: *"Not a browser."* Clause two is
the answer: one requested URL, one impression, no script-chosen navigation
(`js.md` §11's *"No navigation"* is likewise unchanged — `location` writes stay
no-ops). A round trip re-issues **the same** URL; it never follows a page to a
different one.

**Verdict: adopt, as two clauses.** The reframe survives; the one-clause version
does not.

### 1.4 The corollary — `needs:["human"]` is a false signal today

`needs.md` §1: *"`human` is the capability past the end of the flag list — the
page demands interactive verification that no frot recipe will ever provide."*
For Amazon that sentence is **false in both halves**: no interactive verification
is demanded (a browser clears it silently), and a recipe change *does* help (this
design). Telling a caller "escalate to a person" about a gate a browser passes
unattended is itself a false capability signal — VISION principle 5, one layer up
from the defect `bl-7e34` just fixed. **Confirmed, with one correction to the
ball's framing:** the *status* was never a lie (`this recipe cannot render this
page` was true); the **taxonomy member** was. §5 below adopts it.

---

## 2. Q1 — what may execute and submit, and what enforces it

**Recommendation: (a)-only.**

- **(a) In:** the page's own challenge script, run in the existing sandbox,
  issuing the request **it** chose to issue. That is literally what a browser
  does.
- **(b) Out:** frot *synthesizing* a submission, a click, a form post, or a
  navigation. This is VISION's first temptation — *"the moment frot needs to
  express 'click X if Y is present', it becomes a browser-automation DSL, badly"*
  — and Mark's sentence does not ask for it.

**"We won't write a click helper" is not an enforcement.** The structural seam,
grounded in the code as it stands:

- Today the HTTP verb is **not a parameter anywhere in frot**. `Transport::exchange`
  (`src/fetch/transport.rs`) hardcodes `.method("GET")` with `Empty::<Bytes>::new()`;
  `FetchSession::navigate`/`subresource` (`src/fetch/session.rs`) take no method;
  `Intent` names five destinations (`Navigation`, `Style`, `ClassicScript`,
  `Module`, `FetchXhr`) and no verbs. GET-only is a property of the type
  signatures, not a check.
- **Keep it that way for every Rust-side caller.** The proposal threads a request
  body/verb through `request_once` as a new type — sketch:
  `enum ReqBody { None, FromScript(Vec<u8>) }` — whose `FromScript` constructor is
  visible **only to `crate::js::subfetch`** (`pub(in crate::js)`). `navigate()`
  and `subresource()` keep signatures that cannot express a verb.
- The consequence is compile-time and severable: **no code outside the JS subfetch
  path can construct a non-GET request.** A future "submit this form" helper does
  not merely violate a rule — it does not compile. That is the guarantee; a policy
  comment is not.
- Two layers already exist and both stay: the prelude
  (`src/js/prelude/net.js`) is where a page's chosen method arrives
  (`fetch` rejects non-GET with a `TypeError`; XHR `send` throws), and it becomes
  the *only* place a verb is ever named.

Non-goals that must stay pinned by test, because they are exactly where this line
would slide: `form.submit()` / `requestSubmit()` remain no-ops, no input event is
ever dispatched, no syscall exists that a host-side caller can use to originate a
request.

---

## 3. Q2 — GET-only → page-initiated non-GET

`src/js/subfetch.rs` is GET-only *by construction*, not by flag: its doc comment
records *"GET only (implicit — the cache only ever calls `FetchSession::subresource`,
which GETs)"*. This is a real interface change.

**Recommended constraints:**

| constraint | recommendation | why |
|---|---|---|
| intent | `Intent::FetchXhr` only | the navigation stays GET **permanently** — that is the frottage rule at the top. Styles/scripts/modules have no verb in any browser either. |
| verb | **POST only**; every other method keeps its current rejection | the minimal delta that answers the measured mechanism (every observed vendor mints a token by POST). A constant, not a flag; widening it is a doc edit plus a ball, not a config change. |
| request body | new constant **`REQ_BODY_BYTES = 64 KiB`**, per request | a token payload is single-digit KiB; 64 KiB is generous by an order of magnitude and far below anything that is a useful upload. **This is what keeps "submitting anything" from becoming true: frot cannot post a file.** |
| destination | **no new origin restriction** — deliberately | see below. |
| response caching | a POST **must not** enter the once-then-frozen cache | it is not idempotent, and `FetchSession::run` keys the invocation cache by URL alone — a frozen POST response would be served to a later GET of the same URL. Today `cacheable = !matches!(intent, Navigation)`; that test must grow the verb. |
| budget | rides the existing bounds; **no new time budget** | see below. |

**Why no vendor-origin allowlist.** The obvious constraint is "same-origin or a
known challenge-vendor origin (`token.awswaf.com`, `challenges.cloudflare.com`)".
**Recommend against it.** It is a maintained list of third parties that fails on
the next vendor, it is the same species as the body-copy classifier `needs.md` §5
already refuses (*"copy changes per site and per locale; declarations and
structure do not"*), and it is the first step toward vendor-specific evasion code.
It also buys little: a page script can already GET any origin today, so the delta
this change introduces is **the body**, and the body is what gets capped. The
existing credential rules are untouched and do the real work — caller `-H` never
leaves same-origin, and SameSite gates the jar's cookies per context
(`identity.md` §9).

**Budget — and a correction to the filing.** The ball asks whether this rides
`SUBFETCH_MAX = 16`. **That constant does not exist**: it was deleted by `bl-c7e9`
(`js.md` §6 — *"a count measures no resource"*, and it starved code-split apps).
The live bounds are `EXEC_CPU_MS = 1_000` (*CPU* time, enforced at the engine
interrupt hook) and `NET_BUDGET_MS = 1_000` (*wall* time, enforced at the
subfetch dispatch seam) — two bounds in two units since `bl-8dc0` split the one
wall-clock `EXEC_BUDGET_MS`, because compute and network are different resources
— plus `JS_MEM_LIMIT = 64 MiB`,
`SUBFETCH_BYTES = 64 MiB` (pooled **response** bytes), `MAX_BODY_BYTES = 16 MiB`
per response, `VIRTUAL_HORIZON_MS = 10_000`. A POST costs time (already bounded)
and response bytes (already pooled); the only unbounded new quantity is the
**request** body, which `REQ_BODY_BYTES` bounds. **No existing constant changes.**

---

## 4. Q3 — what a round trip is, and why it is not a retry loop

### 4.1 The distinction, stated sharply

> **A round trip answers a challenge the server posed. A retry loop re-asks an
> unchanged question.**

Made structural rather than rhetorical:

> **A second navigation is warranted iff the page's own challenge script ran and
> changed the request frot would now send — specifically, iff the per-invocation
> cookie jar differs from its state at the first navigation.**

That precondition is checkable, and it does the work of an entire prohibition:

- **Retry-until-allowed is impossible by construction.** An identical request is
  never re-sent, because "identical" fails the precondition.
- **UA/IP/proxy rotation is excluded by the same test**, without naming it: a
  rotated persona changes the request, but not by state *the server minted*. The
  jar is the only mutation the round trip reads, and only the server writes it.
- **Backoff-and-try-again has nothing to trigger it** — there is no timer, no
  second attempt at an unchanged request, and nothing to back off from.

One invariant dissolves the whole class, which is the shape this repo prefers to
an answer apiece.

### 4.2 The sequence

```
navigate(url)                       # GET, as today
  └─ declared challenge? (needs::challenge, unchanged)
       └─ --js off  → needs:["js"]         (§5 — the false-signal fix)
       └─ --js on   → parse, run the page's scripts under the §5 bounds
                       (EXEC_CPU_MS compute + NET_BUDGET_MS network)
            └─ jar changed?  no  → needs:["human"], challenge{passed:false}
                             yes → navigate(url) again   [1 round]
                                    └─ still declared? → needs:["human"], passed:false
                                    └─ otherwise       → the normal pipeline, ok
```

Note the challenge page is now **parsed and run**, where today `run.rs` returns
pre-parse. The `>= 400` error flip beside it is untouched.

### 4.3 The hard constant

**`CHALLENGE_ROUNDS = 1`.** A constant, not a flag — the `EXEC_CPU_MS` /
`NET_BUDGET_MS` / `VIRTUAL_HORIZON_MS` / `SUBFETCH_BYTES` tradition, same severability posture as
the 1280px viewport. Recommend **1, not 2**: every measured vendor mechanism is
one challenge → one token → one retry, so a ceiling of 1 means **at most two
navigations per invocation, ever**. If a real target is measured to need two, that
changes one constant in one ball, on evidence — not a config knob and not a loop.

### 4.4 Statelessness survives

VISION principle 1 is *"stateless per process… State **within** a call (redirects,
JS event loop, etc.) is fine; nothing persists across calls."* Two navigations in
one call is the same category as the redirect chain principle 1 names explicitly.
Everything involved is already per-invocation and already dropped at exit: one
`FetchSession`, one `CookieJar` (`bl-6dad`, born empty, `identity.md` §9), one
`Subfetch` cache. **No new persistence surface is introduced** — no disk, no flag,
no `--cookie-jar`, nothing observable by a second invocation. VISION's *"Don't
grow a session model"* stands; it gets a dated clarification that per-invocation
multi-hop is not a session model, not an amendment.

---

## 5. Q4 — the envelope, and the `needs` taxonomy

### 5.1 The additive block

Non-negotiable, and the reason this is not a retreat from principle 5: a caller
must be able to tell *"content, straight"* from *"content, after we answered a
challenge"*. Mark's own resolution (`identity.md` §10, 2026-07-20) — *"be honest
to the user about what happened. How that got done is fine"* — **requires** that
what happened be reported. **"Quietly" in Mark's sentence governs the wire, not
the envelope.**

```json
"challenge": { "declared": "x-amzn-waf-action", "rounds": 1, "passed": true }
```

- `declared` — the header that declared it. Constrained to `envelope::http::SURFACED`
  (single source: it is the same capture `needs::challenge` decides on and
  `http.headers` surfaces — they cannot disagree, `bl-acec`).
- `rounds` — navigations **after** the first. `0` when frot stopped without one.
- `passed` — whether the re-navigation returned a non-challenge response.

Present only when a challenge was declared; absent otherwise
(`skip_serializing_if = "Option::is_none"`, exactly as `js` and `http` do in
`src/envelope.rs`). **The envelope has never renamed a field** — `js`, `http`,
`http.headers` all arrived additively, and so does this. Division of labour:
`http` reports the **delivered** response; `challenge` reports **what it took**,
so a caller seeing a clean 200 does not lose the fact that a 202 preceded it.
Per repo `AGENTS.md`, an output-schema change is escalated to Mark — as `bl-acec`
was.

### 5.2 The taxonomy needs no new member — again

The honest verdict now depends on where the sequence stopped, and the existing two
members cover it exactly:

| situation | verdict | why it is true |
|---|---|---|
| declared, `--js` **off** | `needs:["js"]` | a recipe change genuinely helps. Today this returns `human`, which is the false signal in its purest form. |
| declared, `--js` on, passed | `ok` + `challenge{passed:true}` | it is the page. |
| declared, `--js` on, script failed / budget exhausted / no cookie minted | `needs:["human"]` + `challenge{passed:false}` | no recipe frot offers renders this, **and frot has already tried the automatable route** — the escalation advice is now earned. |
| declared with a vendor action other than `challenge` (`captcha`, `block`) | unchanged | `needs.md` §3 already refuses to invent a verdict for an unmeasured value. A `captcha` action is the human clause, truthfully. |

Same result as `bl-7e34`: **no new `needs` kind, no third envelope shape.** What
changes is that `human` stops being asserted before frot has tried.

---

## 6. Q5 — what stays out, and whether the line slides

**Out, and why each is *out under the principle*, not out by enumeration:**

- **CAPTCHA and every interactive gate.** Human-requiring — the whole point of the
  line. (Invisible/score-based gates are **in**; see §1.3 A2.)
- **Any synthesized input event, click, scroll, or form submission.** The human
  does these through the browser; the browser does not do them.
- **UA / IP / proxy rotation, retry-until-allowed, backoff-and-retry.** Excluded by
  §4.1's precondition, by construction rather than by prohibition.
- **Script-chosen navigation.** frot re-issues the caller's URL; it never follows
  a page elsewhere. `location`/`history`/meta-refresh stay no-ops.
- **Cross-invocation persistence** of tokens, cookies, or anything else.
- **Body-copy classification** of challenge pages (`needs.md` §5) — a challenge is
  recognized only by a server's own declaration, unchanged.
- **Vendor-specific challenge code.** frot runs *the page's* script. It ships no
  knowledge of AWS WAF, Cloudflare, or DataDome beyond the header names already
  allowlisted. If frot ever needs to know a vendor's algorithm, the line has been
  crossed — that is solving the challenge, not executing it.

**Does the line slide?** Two probes:

- *"If executing the page's challenge is in, why is a behavioural interstitial's
  mouse-move out?"* — §1.3 A3: because no browser produces that event; the human
  does. Verified against the code: no input-dispatch path exists, and none is added.
- *"If POST is in, why not submit the login form?"* — because frot synthesizes no
  submission (§2's compile-time seam) and holds no credentials, and the navigation
  is permanently a GET of the caller's URL. A page script that POSTs on its own is
  the browser acting; frot filling a form is frot acting.

The principled difference in both cases is the same one: **origination**. The line
is *who initiated it*, and that is a structural property, not a judgement call.

---

## 7. Q6 — risk posture, named now rather than discovered later

**(i) Sandbox exposure against deliberately-adversarial JS.** Running a hostile
script *to completion* is a longer exposure than bailing at the header. Audited
against the live bounds: `EXEC_CPU_MS` (CPU time) is enforced by the engine
interrupt, and `NET_BUDGET_MS` (wall time) at the subfetch dispatch seam — the
interrupt fires only between JS instructions and spends CPU besides, so a
blocking host-fetch chain would otherwise outrun it; overshoot is bounded to one
in-flight request; `JS_MEM_LIMIT` is engine-enforced;
`VIRTUAL_HORIZON_MS` drops far-future timers; `SUBFETCH_BYTES` pools response
bytes. **Recommendation: change none of them.** But state plainly what they are:
these are *termination* bounds, not a security sandbox. quickjs-ng is a C
interpreter, and a memory-safety bug in it is reachable from page input. **That
risk is not new in kind** — frot already runs arbitrary page JS under `--js`; what
changes is the *intent* of the input. The one genuinely new exposure is the
**request body**: a hostile script now chooses bytes frot sends. It is bounded by
`REQ_BODY_BYTES`, it cannot carry the caller's `-H` cross-origin (session policy,
unchanged), and it cannot carry HttpOnly cookies into JS (`identity.md` §9).

**(ii) Proof-of-work vs VISION's sub-second promise.** PoW is *designed* to cost
CPU. Under `CHALLENGE_ROUNDS = 1` the worst case is one navigation +
one bounded JS run + one navigation ≈ up to 1 s of **CPU** (`EXEC_CPU_MS`) plus
up to 1 s of **network wall** (`NET_BUDGET_MS`) plus two RTTs — two bounds in two
units, never one shared second. VISION's *"What success looks like"* needs a
stated carve-out (drafted in that file), and the CPU unit makes it a *stronger*
claim: the verdict no longer depends on how busy the host is. **Neither bound is
negotiable to make a gate pass**: a PoW is pure compute, so a PoW that outruns
`EXEC_CPU_MS` produces an honest stop (`settled: false, stopped: "budget"`), not a
bigger number. If a target is measured to need more, that is a
finding to record, not a constant to raise quietly.

**(iii) Disclosure — frot becomes quieter on the wire.** An operator who reads
today's docs believes frot stops at every declared gate. After this it does not:
a site's telemetry sees frot as a client that *passed*, and frot no longer emits
the `needs:["human"]` marker on those targets. `VISION.md` says this plainly so
nobody has to infer it.

**(iv) Politeness and rate — flagged as Mark's conscious call, not inherited.**
Recommendation: **no rate mechanism.** A single invocation is bounded at two
navigations, and frot has no rate to govern — the call rate lives in the harness,
and an inter-round delay would be a flag, which is a smell. But two facts are
recorded rather than absorbed: frot is now a tool that *passes* bot defences, and
its posture toward `robots.txt` is still *none* — unchanged by this design, and a
separate decision if Mark wants one.

---

## 8. Q7 — does it actually work? (measure; record a null as loudly as a win)

**Primary case — Amazon** (`identity.md` §3.9, the prompt for this ball):
`frot https://www.amazon.com/ --js --out text`, ×3 medians, from the **same egress
IP** used by §3.1/§3.8/§3.9 (`[redacted-egress-ip]`, [redacted-egress-network]) so the
comparison is controlled against the existing baselines. Success is
`status:"ok"` with real Amazon text **and** `challenge:{declared:"x-amzn-waf-action",
rounds:1, passed:true}`.

**Controls, all required:**

- **reddit** (`retry-after` on a 200) — a different vendor and a different
  mechanism; record pass or honest stop either way.
- **g2** (`x-datadome`, 403) — must be untouched; it is an error path, not a
  challenge path.
- **The full §3.6 field corpus** — the regression that matters is a **false
  positive**: no genuine page may ever trigger a challenge round. Zero `challenge`
  blocks on the corpus is a pass condition.
- **Cost on the ordinary path** — non-challenge output byte-identical, timing
  unchanged. This change must impose nothing on the 99%.

**Measure the cookie flow before assuming the trigger (§10.1).** The jar-delta
precondition is only sound if the token actually reaches the jar and the
re-navigation; trace which write mints `aws-waf-token` (page-initiated POST
response `Set-Cookie` vs. `document.cookie`, same- vs. cross-origin) as the first
capstone step, not after the loop is built.

**Falsifier, stated in advance.** If Amazon still answers a challenge *after* a
passed round, the cause is IP/ASN reputation (`identity.md` §14 item 2), which no
client-side change fixes. That result is written into `identity.md` as a new §3.10
and §14 item 1 is amended accordingly — **it is not a reason to add a second
round, a proxy, or a vendor-specific path.** The instrument is
`examples/ab_harness.rs` (`bl-46f5`), the standing practice is §14 item 1 /
`bl-d66b`, and the measurement capstone may **not** close on a single successful
run.

---

## 9. Proposed decomposition (proposed, NOT filed)

No balls created by this ball. Recommended siblings once Mark signs off:

| # | sibling | delivers | dep |
|---|---|---|---|
| 1 | transport verb seam | `ReqBody` with a `pub(in crate::js)` constructor; `request_once`/`exchange` carry method+body; `navigate`/`subresource` stay verb-less; POST excluded from the invocation cache | — |
| 2 | prelude + subfetch POST | `fetch`/XHR POST path, `REQ_BODY_BYTES`, unchanged resource timing | 1 |
| 3 | the round loop | `run.rs` restructure (challenge → parse+run → jar-delta test → re-navigate), `CHALLENGE_ROUNDS` | 2 |
| 4 | envelope `challenge` block | §5.1; output-schema escalation per repo `AGENTS.md` | 3 |
| 5 | non-goal pins | tests that no host-side caller can originate a request, `form.submit()` is a no-op, no input event is dispatched, no challenge round on a clean page | 1 (parallel to 3–4) |
| 6 | field measurement capstone | §8, including the null **and the §10.1 cookie-flow trace, which precedes sibling 3's loop assumptions** | 4, 5 |

Sibling 6 is the only one that can answer whether any of this was worth doing.

---

## 10. What this design does not solve

- **IP/ASN reputation** (`identity.md` §14 item 2) — untouched, and plausibly
  dominant. If Amazon scores the egress IP, everything here changes nothing.
- **Interactive gates** — permanently out, by the principle.
- **A challenge needing more than one round** — a measurement away from a constant
  change, not designed for speculatively.
- **A challenge whose PoW exceeds one second** — an honest stop, deliberately.
- **The C-interpreter exposure** (§7 i) — bounded for termination, not a security
  boundary, and this design does not make it one.
- **Whether Mark wants frot passing defences at all** — that is the sign-off this
  document is asking for, not something it assumes.

### 10.1 Two jar-delta residuals surfaced in review (2026-07-22, `bl-017a`)

§4.1 treats a changed cookie jar as *the* definition of "the script passed". That
is one sufficient signal, not a proven-complete one, and two gaps must be measured
by the sibling-6 capstone **before** the sibling-3 loop assumes them:

- **Which write reaches the jar is unmeasured.** The trigger fires only if the
  token lands in the per-invocation jar (`bl-6dad`) *and* is visible on the
  re-navigation. Amazon's `aws-waf-token` is typically minted by a POST to a
  **vendor origin** (`token.awswaf.com`), not `www.amazon.com`, so two things must
  both hold and neither is confirmed: (a) the jar must capture `Set-Cookie` **from
  a page-initiated subfetch POST response**, not only from navigations; (b) that
  cookie must be in scope for the `www.amazon.com` re-navigation despite being set
  cross-origin (SameSite/domain, `identity.md` §9). A third path — the script
  writing `document.cookie` directly — is a different flow again. **Sibling 6 must
  trace the actual cookie flow first; sibling 3 must not hardcode an assumed one.**
- **A pass signalled without a cookie is reported as a failure — the safe
  direction, but name it.** Some challenges signal completion by rewriting the DOM
  or setting a JS variable, minting no cookie. The jar-delta test then yields
  `needs:["human"]` + `challenge:{passed:false}` for a page that may be ready. This
  is an honest **false-negative** (never a false-`ok`), so it is fine to ship — but
  it is a residual, not a surprise: jar-delta is *a* sufficient pass signal, not
  the definition of "passed".
