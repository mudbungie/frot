# Challenge — the declared-challenge round trip

**Status: AS BUILT (`bl-15ff`, 2026-11-17).** Signed off by Mark on
2026-11-17 — *"build the round trips."* Designed in `bl-017a`, 2026-07-22, as a
proposal; the text below is that proposal, kept, with a dated **As built** note
wherever the code differs from it. §11 is the as-built summary and the measured
capstone: the round trip works offline and, on first measurement, did not get
Amazon's page — a null, recorded in §11.3. §11.4 is the run-by-run history of
closing the gaps that null named, and it ends in a pass: **measured
2026-11-17, frot gets Amazon's page through the declared challenge** — every
declared challenge of 11 was passed (`rounds:1, passed:true`); 4 further runs
of 15 got a different, undeclared response (`bl-eed9`). **Since 2026-11-20
(`bl-a7c5`) a vendor-declared challenge enters the round trip at any status** —
Cloudflare's is a 403 — and the first Cloudflare measurement is a null with a
named wall (§11.5). The undeclared response is Akamai's interstitial, captured
2026-11-20 (§11.6). Living authority for this path; `identity.md` §10/§14,
`js.md` §6/§11, `needs.md` §1/§3/§5 and `VISION.md` carry dated in-place notes
pointing here.

Trigger: Mark, 2026-07-22 — *"File the boundary movement ball. It's time for that
scope creep. The value of this project is in letting an agent quietly ask for 'the
content at this url, as a human browser would get it'. It's okay to do multiple
round trips to get there."* Evidence: `identity.md` §3.9 (`bl-7e34`) — Amazon's
202 is an AWS WAF **declared challenge** (`x-amzn-waf-action: challenge`,
`challenge.js` + `window.gokuProps` → token → `aws-waf-token` cookie), and passing
it needs three things frot does not do today: execute a challenge, send a
non-GET, re-issue the request.

---

## 1. The principle

### 1.1 What actually stops frot today

**frot executes challenge scripts today.** `--js` runs whatever the page ships,
and `bl-bd4e` learned which surfaces Amazon's script probes **by running it**. Nothing in frot
stops at "a challenge is executing"; what stops is a header check bolted in front
of parse (`src/run.rs`, `needs::challenge` → `needs:["human"]` pre-parse) that
fires on a **declaration** — reddit's `retry-after`, Amazon's `x-amzn-waf-action`
— not on anything intrinsic to the work.

### 1.2 The proposed scope

> **frot executes the page's own challenge script and re-issues the navigation
> the resulting token earns, as far as delivering the requested URL's impression
> requires.**

Two clauses, deliberately:

- **Ceiling — automatable.** A browser at `amazon.com` passes the WAF challenge
  with no involvement from the person driving it, and that is the class of gate
  this design covers — what *"as a human browser would get it"* names.
- **Floor — the frottage rule survives.** frot is still an impression of **one
  URL**. Everything it does must be in service of delivering that impression;
  frot originates nothing else.

### 1.3 The corollary — `needs:["human"]` is a false signal today

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

> **As built 2026-11-17 (`bl-15ff`) — the seam is real, in a different place
> than sketched.** Rust's `pub(in path)` can only name an *ancestor* module, so
> a type defined in `crate::fetch` cannot have a constructor visible "only to
> `crate::js::subfetch`". The enforcement is therefore the reverse arrangement:
> the type lives **where its only constructor may be called**.
>
> - `ScriptBody` (`src/js/subfetch/post.rs`) holds the bytes and their
>   `Content-Type`. Its one constructor, `ScriptBody::from_script`, is
>   `pub(super)` — callable from `crate::js::subfetch` and nowhere else — and
>   refuses a body over `REQ_BODY_BYTES`, so an over-cap body cannot exist.
> - The fetch layer only *consumes* it: `Transport::send(…, body:
>   Option<&ScriptBody>, …)` sends `POST` iff it is `Some`
>   (`src/fetch/transport.rs`), `fetch::Request` carries `body:
>   Option<&ScriptBody>` (`src/fetch/dispatch.rs`), and the one session method
>   that passes one is `FetchSession::submit(url, initiator, &ScriptBody,
>   timeout)` — `Intent::FetchXhr` fixed, never cached. `navigation()`/`load()`/
>   `follow()`/`subresource()` still cannot express a verb. There is no `enum
>   ReqBody`; "no body" is `None`.
> - Net effect is the one promised: **no code outside `src/js/subfetch/` can
>   construct a non-GET request** — it has no way to obtain a `ScriptBody`.
>   No Rust code names a verb except the two literals in `Transport::exchange`.
> - The prelude (`src/js/prelude/net.js`) is the only place a method is chosen:
>   `GET` → `__frot_subfetch`, `POST` → the new `__frot_subpost(url, body,
>   contentType)` syscall, anything else rejects/throws as before.

Non-goals that must stay pinned by test, because they are exactly where the
design would drift: `form.submit()` / `requestSubmit()` remain no-ops, no input event is
ever dispatched, no syscall exists that a host-side caller can use to originate a
request.

> **As built 2026-11-17 (`bl-15ff`).** Pinned in
> `src/run/roundtrip_guard_tests.rs` (`frot_originates_no_submission`): a
> challenge page holding a `<form method=post>` gets no POST and no GET of its
> action from `submit()`, `requestSubmit()`, a dispatched `submit` or a
> dispatched `click`. One correction to the wording above: frot's `<form>` has
> **no** `submit`/`requestSubmit` at all — calling one throws rather than
> silently doing nothing. Either way nothing is submitted, which is the pin.

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

> **As built 2026-11-17 (`bl-15ff`) — the table holds; four things it did not
> say.**
>
> 1. **Text bodies only.** A POST body is a string or a `URLSearchParams`
>    (sent as UTF-8). An `ArrayBuffer`/typed array/`Blob`/`FormData` body
>    **fails the request** (`fetch` rejects, XHR errors) rather than go out as
>    something else. No measured mechanism posts bytes; widening it is a ball.
>    *(2026-11-17, `bl-6eb9`: a `FormData` is now sent, as the
>    `multipart/form-data` **text** Firefox serializes for it — AWS WAF's token
>    POST is one, measured, §11.4. Its entries are strings, so it is still a
>    text body under the same 64 KiB cap; bytes are still refused.)*
> 2. **Request headers.** The POST carries the script's own `Content-Type`
>    (from `fetch` `init.headers` in any of its three shapes, or XHR
>    `setRequestHeader`), else the browser default for the body's type
>    (`text/plain;charset=UTF-8`, or `application/x-www-form-urlencoded;charset=UTF-8`
>    for `URLSearchParams`), plus the `Origin` a browser attaches to every POST;
>    `Content-Length` is the serializer's. **No other script-set request header
>    is sent** — that was already true of GETs and is unchanged.
> 3. **Redirects.** A redirected POST follows the browser rule: `307`/`308`
>    re-send the body to the new target; `301`/`302`/`303` continue as a GET
>    with no body.
> 4. **Target.** Only an `http(s)` URL can receive a POST (`data:`, `file:` are
>    refused), on top of the existing remote→local block.
>
> The cache row is satisfied by construction, not by a grown test: a POST goes
> through `FetchSession::submit`, which never touches the invocation cache, and
> `Subfetch::post` never touches the once-then-frozen one — a later GET of the
> same URL is its own request.

**Why no vendor-origin allowlist.** The obvious constraint is "same-origin or a
known challenge-vendor origin (`token.awswaf.com`, `challenges.cloudflare.com`)".
**Recommend against it.** It is a maintained list of third parties that fails on
the next vendor, it is the same species as the body-copy classifier `needs.md` §5
already declines (*"copy changes per site and per locale; declarations and
structure do not"*). It also buys little: a page script can already GET any origin today, so the delta
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

That precondition is checkable, and one invariant bounds the whole class of
second attempts: an identical request is never re-sent, because "identical" fails
the precondition. The jar is the only mutation the round trip reads, and only the
server writes it — so there is no timer, and no second attempt at an unchanged
request.

> **As built 2026-11-17 (`bl-15ff`) — the test is on the request, not the jar.**
> "The jar differs from its state at the first navigation" was too loose in two
> directions, so the code tests the sentence above it instead — *changed the
> request frot would now send*. `fetch::Navigation::request()` returns the
> header set a `load()` of the caller's URL would send right now (jar cookies
> applicable to that URL, caller `-H` layered on, order-normalised); `run.rs`
> reads it **after** the challenge response has arrived and **before** its
> scripts run, then again after. A second navigation is issued iff the two
> readings differ. Consequences, each pinned:
>
> - a cookie the **server** set on the challenge response itself is not a
>   script's doing and earns nothing (`run::cookie_tests`);
> - a cookie the script set that would **not ride** the re-issue (another
>   path or domain) earns nothing;
> - a caller's own `-H Cookie` replaces the jar's line, so the request cannot
>   change and no round is taken — the caller has taken over the cookies.
>
> "An identical request is never re-sent" is thereby literal: the comparison
> *is* the request.

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
pre-parse. The `>= 400` error flip beside it is untouched. *(2026-11-20,
`bl-a7c5`: no longer untouched — a vendor-declared challenge is consulted
first, at any status; see the note below and §11.5.)*

> **As built 2026-11-17 (`bl-15ff`).** The sequence is built as drawn
> (`src/run.rs`, `navigate`), with these additions the drawing left open:
>
> - **Below 400 only.** A response with status ≥ 400 takes the error flip even
>   when it carries a challenge declaration. Cloudflare serves its managed
>   challenge at **403** (measured, §11.3), so `cf-mitigated: challenge` never
>   enters this sequence on that target. Whether it should is Mark's call —
>   `bl-a7c5`.
>   **Superseded 2026-11-20 (`bl-a7c5`, Mark: yes).** A browser runs a
>   challenge page whatever its status, so a **vendor declaration**
>   (`cf-mitigated: challenge`, `x-amzn-waf-action: challenge`) enters this
>   sequence at any status, and the sequence is otherwise unchanged. A status
>   ≥ 400 that declares nothing is exactly the `http.<code>` error it was.
>   `retry-after` still declares only below 400: beside a 429 or a 503 it is
>   the header's ordinary meaning (a rate limit, an outage), not a stand-in
>   page. The whole rule and its corners: §11.5.
> - **The `js` block.** On an honest stop it is the challenge document's run
>   (so `stopped:"budget"` is visible); when the round is spent and the answer
>   is still a challenge, the second challenge document is **not** run and the
>   block is the first run's; on a pass it is the delivered page's run.
> - **Hops.** The re-issue is `Navigation::load()` on the same navigation: same
>   session, pool, jar and the one ten-request hop budget, no referrer, exactly
>   as the first. A challenge reached through a redirect or a refresh is
>   answered the same way, and the re-issue walks the same hops again.
> - **What the re-issue returns is reported as it is** — see §5.1's note.

> **Code names updated 2026-11-17 (`bl-08d8`).** `FetchSession::navigate()` and
> the private `FetchSession::run` this document cites (§2, §3) no longer exist.
> The top-level GET is `fetch::Navigation` — `session.navigation(url)`, then
> `load()`; **`navigate(url) again` in the sequence above is a second `load()`
> on the same `Navigation`**, which re-issues the caller's URL on the same
> session, jar and hop budget. `run.rs`'s `build_envelope` is already a loop
> that restarts at the top with a new response (the meta-refresh hop uses it via
> `Navigation::follow`); the round trip is a second way to `continue` it, not a
> second path. The invocation cache now lives in `FetchSession::subresource`
> alone — a navigation never reaches it — so §3's "`cacheable =
> !matches!(intent, Navigation)`" test is gone, and the verb rule it asks for
> belongs in `subresource`.

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

> **As built 2026-11-17 (`bl-15ff`).** `ChallengeInfo` in
> `src/envelope/challenge.rs`; the last key of the envelope. `declared` is one
> of `retry-after`, `cf-mitigated`, `x-amzn-waf-action` — whichever
> `needs::challenge` matched first, in that order — and a test pins that each
> is in `SURFACED`. The block rides **every** envelope shape, an `error`
> included. `passed` is exactly the definition above, which has two corners
> worth stating: a re-issue answered with **status ≥ 400** is `passed: true`
> beside `status:"error"` (`http.403`) — the challenge was passed, then the
> server refused — and a re-issue that got **no response** (timeout, redirect
> loop) is `passed: false` beside the fetch error.

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

## 6. Q5 — what stays out

**Out, and why each is *out under the principle*, not out by enumeration:**

- **CAPTCHA and every interactive gate.** Out of scope — no input-dispatch path
  exists. (Invisible/score-based gates need none, so they are in.)
- **Any synthesized input event, click, scroll, or form submission.** No dispatch
  path exists (`js.md` §11), and none is added.
- **Re-sending an unchanged request.** Excluded by §4.1's precondition, by
  construction rather than by prohibition.
- **Script-chosen navigation.** frot re-issues the caller's URL; it never follows
  a page elsewhere. `location`/`history`/meta-refresh stay no-ops.
  > **Superseded in part 2026-11-17 (`bl-c262`, `bl-08d8`).** One document-chosen
  > navigation is now followed: a **zero-delay `<meta http-equiv=refresh>` in
  > the markup as fetched**, treated as a redirect (`js.md` §11's note of the
  > same date; rule in `src/refresh.rs`). It is not *script*-chosen — it is read
  > before any script runs, and a script-inserted refresh, `location` and
  > `history` stay no-ops — so this bullet's subject is unchanged. The round
  > trip itself still re-issues **the caller's URL**: both ride the one
  > in-invocation re-navigation path, `fetch::Navigation` (`load()` re-issues
  > the caller's URL, `follow()` takes a document-directed hop), on one session
  > and one hop budget. *(2026-11-17, `bl-e52a`: the `Refresh:` response header
  > is the same rule and the same hop.)*
- **Cross-invocation persistence** of tokens, cookies, or anything else.
- **Body-copy classification** of challenge pages (`needs.md` §5) — a challenge is
  recognized only by a server's own declaration, unchanged.
- **Vendor-specific challenge code.** frot runs *the page's* script. It ships no
  knowledge of AWS WAF, Cloudflare, or DataDome beyond the header names already
  allowlisted; needing a vendor's algorithm would mean computing the token
  itself, which this design does not do.

The discriminator throughout is **origination**: the page's script initiates the
request; frot does not. That is a structural property, not a judgement call —
§2's compile-time seam is what enforces it.

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

> **As built 2026-11-17 (`bl-15ff`) — one budget per invocation, made
> literal.** The arithmetic above counts one JS run, but a passed challenge
> means **two** documents run: the challenge, then the page behind it. Giving
> each its own `EXEC_CPU_MS`/`NET_BUDGET_MS` would have doubled the invocation's
> ceiling without anyone raising a constant. So both runs draw on **one**
> `js::Bounds`: the network meter is shared, and `Deadline::bank()` closes the
> first run's compute window keeping only what it left, which is all the second
> run opens. The sentence above is therefore true of the whole invocation —
> up to 1 s CPU plus up to 1 s of wire, across both runs. The cost is honest
> and visible: a proof-of-work that spends 700 ms leaves the real page 300 ms,
> and that page then reports `js:{settled:false, stopped:"budget"}` beside
> `challenge:{passed:true}`. Pinned in `roundtrip_guard_tests.rs`. If the
> page deserves a fresh allowance, that is a decision to make on evidence, not
> a default to slip in.

**(iii) Rate.** Recommendation: **no rate mechanism.** A single invocation is
bounded at two navigations, and frot has no rate to govern — the call rate lives
in the harness, and an inter-round delay would be a flag, which is a smell.

---

## 8. Q7 — does it actually work? (measure; record a null as loudly as a win)

**Primary case — Amazon** (`identity.md` §3.9, the prompt for this ball):
`frot https://www.amazon.com/ --js --out text`, ×3 medians, from the **same
reference egress** used by §3.1/§3.8/§3.9 (`identity.md` §3) so the
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

> **Measured 2026-11-17 — see §11.3.** In short: Amazon **null** (the challenge
> script cannot finish inside frot's JS environment; no round was taken), reddit
> honest stop, g2 untouched, zero `challenge` blocks on the ordinary corpus. The
> §10.1 cookie-flow trace is **still owed**: the script never reached a write.

## 9. Proposed decomposition (proposed, NOT filed)

> **As built 2026-11-17.** Not decomposed: one ball, `bl-15ff`, took all six
> rows through in one delivery.

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
- **Interactive gates** — out of scope; frot synthesizes no input.
- **A challenge needing more than one round** — a measurement away from a constant
  change, not designed for speculatively.
- **A challenge whose PoW exceeds one second** — an honest stop, deliberately.
- **The C-interpreter exposure** (§7 i) — bounded for termination, not a security
  boundary, and this design does not make it one.
- **Whether this ships at all** — that is the sign-off this document is asking
  for, not something it assumes.

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

---

## 11. As built, and measured (`bl-15ff`, 2026-11-17)

### 11.1 What happens, in order

1. frot GETs the caller's URL, following redirects and zero-delay refreshes as
   always.
2. If the response carries `cf-mitigated: challenge` or `x-amzn-waf-action:
   challenge` (at any status — `bl-a7c5`, §11.5), or is below 400 and carries
   `retry-after`, a challenge was declared, and
   the envelope will carry a `challenge` block whatever happens next. Otherwise
   nothing below applies and the invocation is exactly what it was before —
   including the `http.<code>` error for a status of 400 or above.
3. Without `--js`: stop, `needs:["js"]`.
4. With `--js`: note the request a re-issue would send, parse the challenge
   document, and run its scripts in the ordinary sandbox under the ordinary
   bounds. The scripts may `fetch`/XHR — including a POST with a text body, the
   one new thing they can do. Whatever cookies those responses or
   `document.cookie` set land in the invocation's jar.
5. Note the request a re-issue would send *now*. Unchanged: stop,
   `needs:["human"]`, `rounds:0`, with the `js` block of that run.
6. Changed: GET the caller's URL again (`rounds:1`). That is the only extra
   navigation there will ever be.
7. Still a declared challenge: stop, `needs:["human"]`, `rounds:1`,
   `passed:false`. Anything else: `passed:true`, and the response goes through
   the ordinary pipeline — its scripts run on what is left of the invocation's
   budget — to whatever envelope it earns.

### 11.2 The envelope

```json
{"frot":"0","url":{"requested":"https://shop.example/","final":"https://shop.example/"},
 "view":"text","status":"ok","out":"the real page",
 "http":{"status":200,"headers":[]},
 "js":{"scripts":0,"errors":0,"settled":true},
 "challenge":{"declared":"x-amzn-waf-action","rounds":1,"passed":true}}
```

`challenge` is the only addition. `needs:["js"]` for a declared challenge
without `--js` (it was `needs:["human"]`) is the one changed verdict.

### 11.3 The capstone — a null

Measured 2026-11-17, release build of the `bl-15ff` worktree. **Egress: the
machine this session ran on. It was not verified to be the §3 reference egress
(`identity.md` §3 deliberately records no address to compare against), so these
rows are not controlled against the earlier baselines for IP reputation.**

| target | recipe | result |
|---|---|---|
| `www.amazon.com/` ×3 | `--js --out text` | **not passed.** `needs:["human"]`, `http.status 202`, `challenge:{declared:"x-amzn-waf-action", rounds:0, passed:false}`, `js:{scripts:3, errors:2, settled:true}` — identical all three runs |
| `www.amazon.com/` | `--out text` | `needs:["js"]`, same `challenge` block, no `js` block |
| `www.reddit.com/` | `--js --out text` | honest stop. `needs:["human"]`, `http.status 200`, `challenge:{declared:"retry-after", rounds:0, passed:false}`, `js:{scripts:1, errors:1, settled:true}` |
| `www.g2.com/` | `--js --out text` | untouched: `error{http.403}`, no `challenge` block |
| `stackoverflow.com/questions` | `--js --out text` | `error{http.403}`, `cf-mitigated: challenge`, `server: cloudflare`, **no `challenge` block** — the Cloudflare challenge arrives at 403, on the error path (`bl-a7c5`) |
| `en.wikipedia.org/wiki/Main_Page`, `lobste.rs/`, `news.ycombinator.com/`, `react.dev/` | `--js --out text` | no `challenge` block on any (`ok`, `ok`, `ok`, `needs:["js"]`) — zero false positives |

**Amazon: failed, and exactly where.** The round trip was never reached. AWS
WAF's `challenge.js` (702 kB) loads and runs, GETs
`…token.awswaf.com/…/inputs?client=browser` (200), and then needs Web APIs
frot's JS environment does not have, so it never produces a token, never
POSTs, never writes a cookie. The request is unchanged, so frot stops after one
navigation — the honest stop, working as designed. Traced with a throwaway
harness that logged reads of absent globals (and, in a second pass, stubbed them
to see what came next):

| absent surface | evidence | ball |
|---|---|---|
| `crypto.subtle` *(closed 2026-11-17, `bl-0647` — §11.4)* | **the hard stop.** With everything else stubbed the script calls `crypto.subtle.importKey("raw", <32 bytes>, {name:"AES-GCM"}, …, ["encrypt"])` — the key and IV are the page's `window.gokuProps` | `bl-0647` |
| `TextEncoder` / `TextDecoder` *(closed 2026-11-17, `bl-53b7`)* | read before and after the inputs fetch | `bl-53b7` |
| `requestIdleCallback` *(closed 2026-11-17, `bl-b584`)* | read right after the inputs fetch returns | `bl-b584` |
| `Blob` (+ `URL.createObjectURL`) *(closed 2026-11-17, `bl-78c5`; the `Worker` worry did not materialise — §11.4)* | read; the script also names `Worker` — if the proof-of-work runs in a Blob-URL worker, frot's non-executing `Worker` is the next wall (unmeasured) | `bl-78c5` |
| `FormData` *(closed 2026-11-17, `bl-6eb9` — it is the token POST's body, §11.4)* | read | `bl-6eb9` |

None was built here: `crypto.subtle` is a large declared residual
(`identity.md` §11) with an open decision attached, and the small ones do not
move the outcome without it.

**What this null does and does not say.** It is **not** the falsifier §8
stated in advance (a challenge still posed *after* a passed round — IP/ASN
reputation); that stays untested, because no round was passed. It says the
blocker for Amazon moved from *"frot will not run the challenge"* to *"frot's
JS environment cannot run this challenge"* — a capability gap with a named
first item, which is the kind of thing frot can close. **§10.1's two residuals
are still unmeasured**: which write mints `aws-waf-token`, and whether it is in
scope for the re-issue, cannot be traced until the script gets that far.

**The mechanism is proven only offline.** `src/run/roundtrip_tests.rs` and
`roundtrip_guard_tests.rs` drive a synthetic challenge (compute a token; set it
by `document.cookie`, by a same-origin POST whose response sets an HttpOnly
cookie, or by a cross-origin POST) to the real page, and every stop short of it.
No live target has yet been passed. That is the honest state of this feature.

### 11.4 The capstone, run by run (from `bl-0647`, 2026-11-17)

§11.3 named gaps; this is what happened as each closed. Every row is
`frot https://www.amazon.com/ --js --js-errors --out text`, release build, a
handful of runs per stage, **from the machine the session ran on — not
verified to be the §3 reference egress**. "Traced" means a throwaway build,
never shipped, that logs reads of absent globals, `crypto.subtle` calls, the
script's requests and unhandled rejections. A null is recorded as loudly as a
win.

| run | frot had | envelope | where the script stopped |
|---|---|---|---|
| 1 (`bl-15ff`, ×3) | the round trip | `needs:["human"]`, 202, `rounds:0`, `js:{scripts:3, errors:2}` | `crypto.subtle` absent (§11.3) |
| 2 (`bl-0647`, ×2 + 1 traced) | + a real `crypto.subtle` | **unchanged — still a null**: `needs:["human"]`, 202, `rounds:0`, `js:{scripts:3, errors:2}` | **past the crypto.** `importKey("raw", 32 bytes, {name:"AES-GCM"}, false, ["encrypt"])` resolves; `encrypt({name:"AES-GCM", iv: 12 bytes, tagLength: 128}, key, 3655 bytes)` resolves to 3671 bytes. Then `new FormData()` — absent, `TypeError: not a function` — while building the POST body (`bl-6eb9`), and separately `document.scripts.length` — absent (`bl-c296`, new). No POST, no cookie |
| 3 (`bl-6eb9`, ×4 + 1 traced) | + `FormData`, sent as multipart text | **PASSED, 4 of 5**: `status:"ok"`, `http.status 200`, `challenge:{declared:"x-amzn-waf-action", rounds:1, passed:true}`, 2.4–4.7 kB of the real home page's text, `js:{scripts:~100, errors:6–8}` on the delivered page. **1 of 5**: 200 with *no* declaration and no text → `needs:["js"]`, no `challenge` block (`bl-eed9`) | nowhere — it finished. Still absent and still read: `document.scripts` (one rejected side path, `bl-c296`), `TextEncoder`, `requestIdleCallback`, `Blob` (all feature-tested) |
| 4 (`bl-53b7`, ×2) | + `TextEncoder`/`TextDecoder` | passed, 2 of 2: `ok`, 200, `rounds:1, passed:true`, 2.3–3.1 kB, `js:{scripts:~102, errors:5}` | finished. On the delivered page, two of the five errors are `Image is not defined` (`bl-1404`, new) |
| 5 (`bl-b584`, ×2) | + `requestIdleCallback` | **1 of 2 passed** (`ok`, 200, `rounds:1, passed:true`, 4.7 kB, `js:{scripts:101, errors:5}`); **1 of 2** the undeclared 200 again — `needs:["js"]`, no `challenge` block, `js:{scripts:2, errors:0}` (`bl-eed9`, second sighting; both were the first request after a pause) | the passing run finished |
| 6 (`bl-c296`, ×2) | + `document.scripts` | passed, 2 of 2: `ok`, 200, `rounds:1, passed:true`, 3.1 kB, `js:{scripts:102, errors:5}` | finished. Not traced, so that the challenge page's `document.scripts` walk now completes is shown by the offline test, not by this run |
| 7 (`bl-78c5`, ×3 + 1 traced; the final tree) | + `Blob`, `URL.createObjectURL` | **2 of 4 passed** (`ok`, 200, `rounds:1, passed:true`, 2.3 kB, `js:{scripts:~102, errors:5}`); **2 of 4** the undeclared 200 (`bl-eed9`). A fifth request without `--js` got the declared 202 and `needs:["js"]`, as designed | the passing runs finished. With `Blob` present the script now tries `new FileReader()` before its `btoa` fallback (`bl-8c46`, new) |

**What run 2 measured that §11.3 could only guess** (from the trace and from
reading `challenge.js`):

- **The crypto the script needs is exactly what was built**: one AES-256-GCM
  key, a 96-bit IV, a 128-bit tag. Nothing else of `SubtleCrypto` is called.
- **`FormData` is the POST body** on the path taken, so `bl-6eb9` and §3's
  "text bodies only" limit do move together: the script appends string fields
  and hands the `FormData` to `fetch`.
- **`TextEncoder`/`TextDecoder` and `requestIdleCallback` are feature-tested**
  (`'function' == typeof TextEncoder ? … : <its own UTF-8 code>`), so their
  absence is not what stops it. `Blob` is used inside a `try` whose `catch`
  falls back to `btoa`.
- **`Worker` is not on the path.** The script's `Worker`/`Blob`/
  `createObjectURL` text is a bundled library's CPU-core estimator, not the
  proof-of-work. `bl-78c5`'s worry is not borne out so far.
- **One of the script's three challenge kinds uploads padding**: it builds an
  `ArrayBuffer` of 1 KiB, 10 KiB, 100 KiB, 1 MiB or 10 MiB by difficulty and
  POSTs it base64-encoded. Above difficulty 2 that exceeds `REQ_BODY_BYTES`
  (64 KiB, §3). *(Measured in run 7: **this is the kind Amazon serves**, at
  its smallest size — the trace shows `new Blob([<1024-byte ArrayBuffer>])`.
  The whole token POST is 11.4 kB.)*

The offline proof now includes this shape: `src/run/roundtrip_subtle_tests.rs`
is a synthetic challenge that fetches inputs, imports the page's 32-byte key,
AES-GCM-encrypts its answer and POSTs it, against a server that mints the
token only for the correct ciphertext (a published test vector).

**Run 3 is the first live pass.** What happened, in order, traced:

1. GET `www.amazon.com/` → 202, `x-amzn-waf-action: challenge`. frot notes
   the request a re-issue would send, and runs the page's three scripts.
2. `challenge.js` GETs `…token.awswaf.com/…/inputs?client=browser` (200).
3. It imports the page's 32-byte key and AES-256-GCM-encrypts 3,655 bytes of
   collected signals (§11.4 run 2).
4. It base64-encodes 1,024 bytes of padding (the "challenge": an upload, not a
   computation), builds a `FormData` (`solution_metadata`, plus the encrypted
   signals) and POSTs it — 11.4 kB of multipart text — to
   `…token.awswaf.com/…/mp_verify`.
5. The answer is 200, a small JSON object carrying a `token`. **No
   `Set-Cookie`.**
6. The script writes `document.cookie = "aws-waf-token=…"` itself.
7. The request frot would now send has a new cookie, so it GETs the caller's
   URL once more → 200, the home page. Its ~100 scripts run on what is left of
   the invocation's budget (one traced run ended `stopped:"network"`).

**§10.1's two residuals are answered by measurement.** *Which write mints the
token:* the script's own `document.cookie` write, from a token returned in the
**body** of a cross-origin POST — not a `Set-Cookie` on that response, and not
a same-origin POST. *Is it in scope for the re-issue:* yes — the script sets it
on the page's own site, so it rides the second navigation. The trigger
(§4.1, as built: "the request frot would now send changed") held with no
change.

**What this pass does and does not say.**

- **§8's falsifier did not fire.** A passed round was *not* followed by another
  challenge, so from this egress Amazon's gate is the challenge itself, not
  IP/ASN reputation (`identity.md` §14 item 2). Other egresses are unmeasured.
- **It is not 5 of 5.** One run got a 200 that declared nothing and had no
  text. The round trip never saw it — no declaration, no round — and frot said
  `needs:["js"]`. What that response is, is unmeasured (`bl-eed9`). *(Measured 2026-11-20, §11.6.)*
- **The delivered page is a real but partial impression**: `js.errors` is 6–8
  (Amazon's own scripts hit APIs frot lacks, and its telemetry hits the
  network budget), and the text is 2.4–4.7 kB.
- **Nothing was raised to get here.** `EXEC_CPU_MS`, `NET_BUDGET_MS`,
  `REQ_BODY_BYTES` and `CHALLENGE_ROUNDS` are untouched; frot clicked nothing,
  solved nothing, and sent only the requests the page's own script chose.
- **Proof-of-work was not the obstacle — none was asked for.** *(Corrected
  after run 7; this bullet first said the padding-upload kind "was not
  served", which nobody had measured.)* The challenge Amazon serves is the
  padding upload at 1 KiB: no hashing loop, well inside both budgets and the
  64 KiB body cap. A harder difficulty (100 KiB and up) would exceed that cap
  and be an honest stop; that has not been observed.

**Where it stands (2026-11-17, after run 7).** Since the challenge first
passed: **15 `--js` runs, 11 passed, 4 got the undeclared 200.** Put the other
way: **every one of the 11 declared challenges was passed** (`rounds:1,
passed:true`, real home-page text), and the 4 misses are a response that
declares nothing, runs two scripts cleanly and has no text — frot answers
`needs:["js"]` and takes no round, which is the design for an undeclared page
(§6). What that page is, and whether a browser gets it too, is `bl-eed9`. *(Answered 2026-11-20: Akamai's interstitial — §11.6.)*

**§8's controls, re-run once each on the final tree** — unchanged from §11.3:
reddit `needs:["human"]`, `challenge:{declared:"retry-after", rounds:0,
passed:false}` (its script still changes nothing); g2 `error{http.403}`, no
block; stackoverflow `error{http.403}` with `cf-mitigated: challenge`, no block
(`bl-a7c5`); wikipedia, lobste.rs, HN `ok` and react.dev `needs:["js"]`, none
with a `challenge` block — **zero false positives.**

**What the delivered Amazon page still trips on** (its own scripts, after the
challenge; 5 `js.errors` per run): `Image is not defined` twice (`bl-1404`);
`crypto.subtle.importKey("raw", …, {name:"AES-CBC"}, …)` rejected
`NotSupportedError` three times (`bl-db8f` — the first unimplemented algorithm
with a measured caller). Its two `crypto.subtle.digest("SHA-256", …)` calls
succeed.

> **2026-11-20 (`bl-db8f`):** AES-CBC is built, with the rest of the
> secret-key half of WebCrypto (identity.md §3.19), so those three rejections
> are gone.
>
> **2026-11-20 (`bl-1404`):** `Image` exists (identity.md §3.20); the page's
> beacons construct and are not sent.

### 11.5 A challenge declared at 400 or above (`bl-a7c5`, 2026-11-20)

Decided by Mark, 2026-11-20: **yes** — a declared challenge enters the round
trip whatever its status. A browser does not read the status before running
the page; Cloudflare's managed challenge is a **403** + `cf-mitigated:
challenge`, and until this ball that was `error{http.403}` with no script run.

**The rule, in the order it happens.**

1. frot GETs the caller's URL.
2. It asks whether the response declares a challenge (`needs::challenge`, the
   one place the rule lives): `cf-mitigated: challenge` or `x-amzn-waf-action:
   challenge` declare **at any status**; `retry-after` declares **only below
   400**. Beside a 429 or a 503, `retry-after` is the header's ordinary meaning
   — a rate limit, an outage — and such a response is the error it says it is.
3. Declares nothing and the status is 400 or above: `error{http.<code>}`,
   before parsing, no script run, no `challenge` block — byte for byte what it
   was before this ball.
4. Declares a challenge: §11.1 from step 3 on, unchanged. The `http` block
   reports the response as it was, so the status is never lost.

**Every verdict corner** (each pinned in `src/run/roundtrip_status_tests.rs`;
the examples are a 403 + `cf-mitigated: challenge`):

| what happened | envelope | exit |
|---|---|---|
| no `--js` | `needs:["js"]`, `http.status 403` with the declaring header, `challenge:{declared:"cf-mitigated", rounds:0, passed:false}`, no `js` block, one request | 0 |
| `--js`, the scripts changed nothing | `needs:["human"]`, `http.status 403`, `challenge:{rounds:0, passed:false}`, the `js` block of that run, one request | 0 |
| `--js`, re-issue is still a declared challenge | `needs:["human"]`, `http.status 403`, `challenge:{rounds:1, passed:false}`, the `js` block of the first run (the second challenge is not run), two requests | 0 |
| `--js`, re-issue is the page | `ok`, `http.status 200`, `challenge:{rounds:1, passed:true}` | 0 |
| `--js`, re-issue is a 403 that declares nothing | `error{http.403}`, `challenge:{rounds:1, passed:true}` — the challenge was passed, then the server refused (§5.1) | 1 |
| a 403 / 429 / 503 that declares nothing, `--js` or not — `retry-after` beside it, or `cf-mitigated: block` | `error{http.<code>}`, no `challenge` block, no script run, one request — **byte-identical** to before | 1 |

**What changed for a caller.** One thing: a response at 400 or above that
carries a vendor's challenge declaration was `status:"error"` (exit 1) and is
now `status:"needs"` (exit 0) or, when the round is passed, `ok`. The honest
http fact rides in `http.status`. No field was added, renamed or removed.

**Measured 2026-11-20**, release build of the `bl-a7c5` tree, from the machine
the session ran on (not verified to be the §3 reference egress).
`frot https://stackoverflow.com/questions/1732348 --js --js-errors --out text`:

| run | result |
|---|---|
| ×1, before the fix below | **not passed.** `needs:["human"]`, `http.status 403`, `challenge:{declared:"cf-mitigated", rounds:1, passed:false}`, `js:{scripts:2, errors:2}` — the script threw on its first act |
| ×2, the final tree | **not passed.** Same envelope, `js:{scripts:2, errors:1, settled:true}` — identical both runs |
| ×1, no `--js` | `needs:["js"]`, `http.status 403`, `challenge:{declared:"cf-mitigated", rounds:0, passed:false}` |
| g2, reddit (controls, ×1 each) | unchanged: g2 `error{http.403}`, no `challenge` block; reddit `needs:["human"]`, `challenge:{declared:"retry-after", rounds:0, passed:false}` |

No other `cf-mitigated` target is named in these documents, so none was run.

**Cloudflare: failed, and exactly where.** The round trip now runs — the 403
is parsed, its inline script and the 650 kB challenge-platform script it loads
both execute — and the script cannot finish in frot's JS environment. It is a
virtualised interpreter, so it was traced (seven runs of a throwaway build
that logs absent-property reads, requests, cookie writes and caught throws),
not read. In order:

| step | what the script did | frot | ball |
|---|---|---|---|
| 1 | called `addEventListener(…)` with no receiver (strict code, `this === undefined`) | **threw** — fixed here: a receiverless call on the EventTarget trio is a call on the window, as in a browser (`src/js/prelude/events.js`) | this one |
| 2 | created an `<iframe>`, read its `contentWindow` and `contentDocument` | both absent | `bl-54ac` |
| 3 | `new PerformanceObserver(…)` | absent → `TypeError: not a function`, the run's one counted error | `bl-b129` |
| 4 | *(traced build only, with step 3 stubbed)* `observe({entryTypes:["resource","navigation"]})`; read `navigator.gpu` and `RTCPeerConnection` (absent); created a second iframe and called its `contentWindow.eval` | **the hard stop**: `cannot read property 'eval' of undefined` — it wants a second global to evaluate code in | `bl-54ac` |
| 5 | built an interstitial UI (headings, an image, a link) | which UI — an error notice or a widget wanting a click — was not determined | — |

It made **no `fetch`, no XHR, no POST and no `document.cookie` write.** So
whether Cloudflare's challenge ends in an unattended pass or in an interactive
widget is **unmeasured** — frot does not get far enough to be asked. If it is
a widget, that is the refused side (§6) and `needs:["human"]` stays the answer.

**Why `rounds` is 1 when nothing was solved.** The second navigation was
earned by a cookie the *server* set on the response to the page's `<script
src>` — it reached the jar, so "the request frot would now send" (§4.1)
differed. The rule held to its letter; a browser would not have re-navigated.
The cost is one extra GET, answered by the same challenge, and the verdict is
still honest. Whether a subresource response's cookie should count is a
decision, not a fix: `bl-cc4c`.

**Nothing was raised, clicked or solved**, and no vendor-specific code was
added: the one code change beyond the status rule is a general EventTarget
correction.

### 11.6 The undeclared 200 on Amazon, captured (`bl-eed9`, 2026-11-20)

Captured with a throwaway build that saves each navigation's response, from
the machine the session ran on. Runs: 8 plain GETs of `www.amazon.com/` (all 8
the declared 202), then 6 × `frot https://www.amazon.com/ --js --js-errors
--out text` — **5 passed** the declared challenge (`ok`, 200, `rounds:1,
passed:true`, 2.4–2.9 kB of text, `js:{scripts:~105, errors:5}`) and **1 got
the undeclared 200**, which is the one captured.

**What it is: Akamai Bot Manager's interstitial.** Amazon answers from two
CDNs. Most requests reach CloudFront and get AWS WAF's declared 202; some
reach an Akamai edge and get this:

- **Headers:** `200`, `server: AkamaiGHost`, `content-type: text/html`,
  `cache-control: no-cache, no-store, must-revalidate`; it sets one cookie
  (`ak_bmsc`). No `retry-after`, no vendor action header — **nothing declares
  it.**
- **Body** (2,159 bytes), in document order:
  1. `<meta http-equiv="refresh" content="5; URL='/?bm-verify=<token>'">` — a
     five-second refresh, the no-script fallback.
  2. An inline script: `var i = <a number>; var j = i + Number("6540" +
     "16078");`. That one addition is the whole "proof of work".
  3. An `<iframe>` of a tracking gif.
  4. An inline script that sends a **synchronous XHR**, `POST
     /_sec/verify?provider=interstitial`, `Content-Type: application/json`,
     body `{"bm-verify": <token>, "pow": j}`, and on `loadend` reads the JSON
     answer: `reload: true` → `window.location.replace(<its own URL with any
     bm-verify parameter removed>)`; a `location` field →
     `location.replace(that)`; anything else → `location.reload()`.

**What a browser does with it, unattended:** runs the script, POSTs, is
answered `{"reload": true}`, and replaces the location with its own URL — a
reload — which returns the real page. No person is involved.

**What frot does with it (traced), in order:**

1. GET → the 200 above. Nothing declares a challenge, so §11.1 does not apply.
2. The five-second refresh is not followed — only a zero-delay refresh is a
   redirect (§6).
3. Both scripts run cleanly (`js:{scripts:2, errors:0, settled:true}`). The
   POST goes out on the existing page-initiated POST path and is answered
   `200`, `{"reload": true}`.
4. The script calls `location.replace` with the caller's own URL. That is a
   script-chosen navigation, a no-op by design (§6).
5. The document has no text: `needs:["js"]`, no `challenge` block, one
   navigation.

**Not measured:** whether the verify response sets a cookie — that is, whether
the request frot would re-send had changed (§4.1's test).

**Verdict on the two candidate fixes.** It is *not* a second spelling of a
declaration: no header says anything, and recognising it by `server:
AkamaiGHost` or by its body would be the vendor/body-copy classifier §6 and
`needs.md` §5 refuse — so nothing was added to `needs::challenge`. What would
resolve it is honoring the script's own request to reload the caller's URL.
That moves the boundary in §6 and is Mark's call, filed with this evidence as
`bl-99b7`. Until then frot's answer for this response is unchanged and stated
plainly: one Amazon run in four or so ends `needs:["js"]` under `--js`.
