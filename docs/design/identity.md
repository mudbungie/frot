# Network identity — one profile, derived everywhere

Living design for `bl-abe5`. Authority for the eight implementation siblings
(`bl-5191`, `bl-20ec`, `bl-abca`, `bl-6dad`, `bl-3972`, `bl-e707`, `bl-08f6`,
`bl-d66b`). Where this doc and the code disagree, fix whichever is wrong and say
so here — `~/AGENTS.md`: *never assume any doc is correct*.

## The one-sentence shape

Every fact a server can observe about who frot is — TLS ClientHello, ALPN, HTTP
version, header names/order/values, cookies, `navigator`, clocks — is **derived
from one `BrowserProfile` constant**, and frot **genuinely negotiates everything
it advertises**. It is not, and under frot's hard constraints cannot be,
byte-identical to Firefox.

---

## 1. The falsification rule — and that it has fired

> **Rule.** If an exact, attainable browser profile cannot fit frot's hard
> constraints (static musl, security maintenance, the 5–15 MiB envelope,
> representative testability), report the ceiling rather than ship another
> contradictory costume.

**It fired.** The measured spike (`bl-0356` transport spike, 2026-07-19) proves
no route delivers a byte-exact Firefox inside those constraints.

- **The ceiling is the static-musl constraint** — specifically **the absence of a
  musl-targeting C++ toolchain**. Every stack that reproduces the measured
  Firefox profile today (`wreq`, `warpsock`, `curl-impersonate`) is built on
  **BoringSSL, which is C++**. `musl-tools` ships `musl-gcc` only; `boring-sys2`
  fails with `ToolNotFound: x86_64-linux-musl-g++` (`dpkg -L musl-tools | grep
  bin/` → `musl-gcc`, `musl-ldd`, no C++ compiler).
- **Size was never the binding constraint.** The spike put the selected route at
  +284 KiB stripped musl; the 2026-07-20 whole-binary rebuild puts it **32 KiB
  *below* today** (§6.1 config (b)), and even `wreq` would land at ≈8.46 MiB,
  inside the envelope. The constraint that binds is the toolchain.
- **The pure-Rust ceiling is real and independent.** `rustls` emits **at most two
  key shares** where Firefox emits **three** (4588, 29, 23). The `h2` crate's
  `Pseudo` struct is hardcoded `m,s,a,p` and exposes no client API for the
  HEADERS `PRIORITY` flag, so Firefox's `m,p,a,s` and weight-42 priority need a
  fork. Byte-exact Firefox in pure Rust means owning **two** forks indefinitely —
  which is exactly how frot acquired `craftls`.
- **The ESR escape hatch is closed.** 140esr postdates both the PQ key share
  (Firefox 132, Nov 2024) and zstd `Accept-Encoding` (Firefox 126). Measured:
  140.12.0esr sends X25519MLKEM768 **first** in both `supported_groups` and
  `key_share`, and sends `zstd`. The only PQ-free Firefox is ≤131, i.e. EOL
  115esr. Pinning ESR does not avoid X25519MLKEM768 and never could.

**Therefore the goal of this design is stated as a first-class commitment, not an
apology:**

> frot's network identity is **coherent, current, and maintained** — a client
> that genuinely negotiates what it advertises, with no dead dependencies and no
> internal contradictions. It is **explicitly not byte-identical to Firefox.**

The rationale is that coherence is the property that is actually *achievable* and
*testable*, and incoherence is what is actually *cheap to detect*. A defence that
sees a Firefox-140esr ClientHello followed by a Chrome-shaped `Accept` header, or
by a refusal to speak h2, does not need a hash database — it needs one `if`.
Removing every such `if` is a finite, verifiable job. Matching a hash is not.

**Honest caveat.** This is a **host-configuration ceiling, not a law**. A musl C++
cross-toolchain (musl-cross-make, or an Alpine build container) would likely let
`wreq` link statically; that was not proven, so `wreq`'s static-musl status is
**"not verified", not "impossible"**. Proving it is the single highest-value
follow-up and would reopen §6. If the project ever decides fingerprint exactness
outranks static musl and blocking I/O, `wreq` is the correct answer — that is a
VISION-level trade, not an implementation detail.

---

## 2. The persona

**Pinned: Mozilla Firefox `140.12.0esr`, the official Mozilla `linux-x86_64`
en-US tarball build.**

Captured 2026-07-19 from a real binary driven through Gecko's own network stack
(necko/NSS) via Marionette — not curl, not a library. `application.ini` `Version`
`140.12.0`, `BuildID` `20260609153453`, `SourceStamp`
`7df86525c2c876c7c92320e49c3e0771f7a605c0`.

**Why ESR 140 over rapid-release 152:**

1. **Stability.** 140.x is the sole ESR line (`FIREFOX_ESR_NEXT` empty), so the
   fingerprint holds still for the support window. 152's moves every ~4 weeks,
   and every move invalidates every golden capture.
2. **Migration cost is near zero on the cipher axis.** frot's current JA4 cipher
   hash is **`5b57614c22b0`**; 140esr's is **`5b57614c22b0`**; 152's is
   `86a278354501`. frot's inherited `FIREFOX_105` cipher list is *already the
   140esr list* — 17 suites, same set, including `0xc009`
   (`ECDHE_ECDSA_AES_128_CBC_SHA`) which 152 dropped. Pinning 152 would mean
   removing a cipher and regenerating everything; pinning 140esr means changing
   nothing on that axis. *(Verified: the JA4 sorted cipher lists in
   `fingerprints.md` and `firefox-esr140.md` are character-identical. Wire
   **order** — `ja4_ro` — was not captured for frot and must be confirmed against
   140esr's `ja4_ro` during `bl-abca`.)*
3. **`accept-language` already matches.** 140esr emits `en-US,en;q=0.5`; frot
   emits `en-US,en;q=0.5`. 152 emits `q=0.9`.
4. **The feared drift did not happen.** `zstd`, X25519MLKEM768-first, and ECH-last
   were all flagged as likely 140esr regressions and all three are measured
   non-issues. The pin costs no modernity.

**Mozilla tarball, not the Ubuntu snap — deliberately.** The 152 capture showed
`Mozilla/5.0 (X11; **Ubuntu**; Linux x86_64; rv:152.0) …`; the `Ubuntu` token is
a *distro artifact* (the snap patches `general.useragent.vendor`), not a version
difference. The 140esr tarball omits it. frot pins the tarball persona because
the TLS/h2 bytes it reproduces are the tarball's: copying a snap UA onto
tarball-shaped everything-else is precisely the incoherence this phase exists to
delete.

**Update cadence.** The profile is pinned to an ESR *line*, re-captured on each
new ESR major (Mozilla ships roughly one per year). The profile constant carries
its own `PINNED` capture date and its line's `EOL` date, and **a test fails once
`EOL` passes** (see I8, `tests/hygiene/persona.rs`) — silent staleness becomes a
red test run, with no config knob and no calendar to remember. *(The 140esr EOL
date is taken from Mozilla's published ESR calendar at implementation time; it
is not measured here.)*

---

## 3. Evidence matrix

All rows measured 2026-07-19 from **the reference egress**, so client identity is
separated from IP reputation. frot binary: worktree `bl-0356` @ `4aac8b2`,
`cargo build --release`.

> **The reference egress** is one fixed residential IPv4 on a US consumer ISP,
> used unchanged for every measurement in this document and in `challenge.md`.
> The literal address and its ISP/city are deliberately **not recorded here**:
> they identify a private home network, and they buy no reproducibility, since
> nobody else can measure from that line anyway. What the evidence rests on is
> not *which* address it was but that it was the **same** one across every row —
> each later section states that it re-verified sameness against §3.1.

### 3.1 Same-egress fingerprint comparison

| field | frot (today) | **Firefox 140.12.0esr** *(the pin)* | Firefox 152.0.5 | curl 8.18.0 |
|---|---|---|---|---|
| negotiated version | HTTP/1.1 | **h2** | h2 | h2 |
| ALPN offered | `["http/1.1"]` | **`["h2","http/1.1"]`** | `["h2","http/1.1"]` | `["h2","http/1.1"]` |
| JA4 | `t13d1714h1_5b57614c22b0_abe81bfac2ff` | **`t13d1717h2_5b57614c22b0_3cbfd9057e0d`** | `t13d1617h2_86a278354501_3cbfd9057e0d` | `t13d3013h2_1d37bd780c83_8537cf56674e` |
| JA3 hash | `4b88cf1ba40252bd8c9ef14e9a4e2614` | `6f7889b9fb1a62a9577e685c1fcfa919` | `6447ab086255d194909d4013b1a89e87` | `32e4b8812cda0c0d50783b438492a769` |
| peetprint hash | `95696730db99c3472a5d74e2dce36e28` | `89d89662b21018947a9a46658c4f5ede` | `fd4547eeb41f073156b7bc8125a79a3c` | `02df6b5136d60208cb9c138793f899ba` |
| akamai (h2) hash | **none — no h2** | `6ea73faa8fc5aac76bded7bd238f6433` | `6ea73faa8fc5aac76bded7bd238f6433` | `55cb2bd667724e6d46cc3c8dd15d50de` |
| akamai text | — | `1:65536;2:0;4:131072;5:16384\|12517377\|0\|m,p,a,s` | *identical to 140esr* | `3:100;4:65536;2:0\|1048510465\|0\|m,s,a,p` |
| cipher count / set | 17 — **already 140esr's set** | 17 | 16 (no `0xc009`) | 30 |
| TLS extensions | 14 | **17** | 17 (same list+order as 140esr) | 13 |
| key shares | 1 | **3** (4588, 29, 23) | 3 | — |
| X25519MLKEM768 (4588) | absent | **present, first** | present, first | absent |
| ECH (0xfe0d) | absent | **present, last** | present, last | absent |
| GREASE | — | **absent** | absent | — |
| advertised UA | Firefox **121.0** | Firefox **140.0** | Firefox 152.0 (`Ubuntu` token) | `curl/8.18.0` |

**Cross-validated**: peet.ws and browserleaks.com independently reported the same
`ja3_hash`/`ja4`/`ja4_r` for 140esr on separate connections; a second full run
reproduced every hash and the entire header list exactly.

**Normalized out of every comparison above** (per-connection random, not
identity): `client_random`, `session_id`, `key_share` payload bytes, ECH payload.

### 3.2 The measured gap, frot → 140esr

| layer | delta | cost |
|---|---|---|
| ciphers | **none** — set already matches (wire order unverified) | 0 |
| TLS extensions | missing `session_ticket(35)`, `signed_certificate_timestamp(18)`, `compress_certificate(27)`, `encrypted_client_hello(65037)`; sends extra `padding(21)` | craft-layer port |
| extension **order** | frot's 14 are a **correct prefix** of Firefox's 17 — same relative order | 0 |
| `supported_groups` | frot lacks 4588; needs `4588,29,23,24,25,256,257` | provider change |
| `key_share` | 1 share vs 3 | **partially blocked** — rustls emits ≤2 (§6) |
| signature algorithms | **identical, in order** | 0 |
| ALPN / h2 | h1-only vs `h2,http/1.1` + full h2 frame profile | §7 |
| headers | 9 measured mismatches (§3.3) | `bl-20ec` |

### 3.3 Header mismatches (measured, HTTP/1.1, directly comparable)

| # | aspect | frot | Firefox 140esr |
|---|---|---|---|
| 1 | first header | `Accept-Encoding` | `Host` |
| 2 | `Host`/`User-Agent` order | UA before Host | Host before UA |
| 3 | `Accept-Encoding` position | 1st | 5th |
| 4 | `Accept-Encoding` value | `gzip, br` | `gzip, deflate, br, zstd` |
| 5 | `Accept` value | contains **`image/avif,image/webp`** | `…xml;q=0.9,*/*;q=0.8` |
| 6 | `Accept-Language` | `en-US,en;q=0.5` | `en-US,en;q=0.5` — **matches** |
| 7 | `Connection` | absent | `keep-alive` |
| 8 | `Priority` | absent | `u=0, i` |
| 9 | `TE` (h2 only) | n/a | `trailers`, last |

Row 5 is the sharpest: **`image/avif,image/webp` is Chrome-shaped**, on a
client claiming to be Firefox. That is an internal contradiction independent of
any version question, and it is exactly the class of tell §1 says is cheap to
detect and finite to fix.

### 3.4 Header casing is scheme-dependent — measured

frot presents **two different header identities depending on URL scheme**:

| scheme | casing | path |
|---|---|---|
| `https://` | **Title-Case** (Firefox-like) | through `FirefoxTlsConnector::transmit_output` |
| `http://` | **lowercase** (bot tell) | bypasses the connector entirely |

Confirmed by a controlled same-run A/B against a local TLS server with a
throwaway CA (`frot-https-head.md`): both heads are byte-identical after
case-folding and port normalization — *the only difference is casing*.

This is not cosmetic. It is the design failure this phase exists to remove:
**identity is applied by the transport seam instead of derived from one
profile**, so it silently varies with an unrelated axis. `bl-20ec` dissolves it
by construction — one profile owns the ordered request description and hands it
to both the h1 and h2 serializers, so there is no second place for casing to be
decided. (Note the irony: the *correct* case is the one WAFs see, which is why
the defect could sit undetected.)

### 3.5 Connection behaviour

`strace -f -e trace=connect`, counting port-443 connects:

| run | requests | connections | same IP? |
|---|---|---|---|
| `react.dev --css` | 2 | 2 | no (2 A records) |
| `en.wikipedia.org --css` | 3 | **3** | **yes** — conclusive |

**frot opens one TCP connection per HTTP request and reuses none.** A real
browser multiplexes these onto one h2 connection. Root cause: `fetch_within()`
builds a fresh `ureq::Agent` per URL (`bl-5191`). Consequence beyond latency:
**the TLS fingerprint is presented N+1 times per page**, and a client that never
resumes a session is itself anomalous. `connections.md` calls this "the single
most browser-unlike behaviour measured."

**Resolved, with one bounded remainder (`bl-5191`, then `bl-fa12` 2026-07-24).**
The per-invocation `FetchSession` pool made this **1** connection for
`wiki --css` (§3.8), and h2 multiplexes a whole wave onto that one. The
remainder is h1-only: `hyper_util` checks an HTTP/1.1 connection back in from a
spawned task, so a saturated host can make a sequential same-origin pair open
two sockets (§11, measured `bl-df88`). §6.5 decides to declare that rather than
own the pool — including why the *"presented N+1 times"* clause above no longer
bites on it: rustls resumption is on by default and the `ClientConfig` is
per-invocation, so the re-dial is an abbreviated handshake, as a browser's
second connection is.

### 3.6 Field corpus (`--out text`, no `-H`, median of 3 runs)

| site | frot status | protocol | verdict | curl status |
|---|---|---|---|---|
| stackoverflow.com/questions | **200** | HTTP/1.1 | **real content, 11783 B** | 200 |
| www.amazon.com/ | **202** | HTTP/1.1 | **empty — soft block, no `out` key** | 503 block page |
| en.wikipedia.org/wiki/Main_Page | 200 | HTTP/1.1 | real content | 200 |
| lobste.rs/ | 200 | HTTP/1.1 | real content | 200 |
| news.ycombinator.com/ | 200 | HTTP/1.1 | real content | 200 |
| react.dev/ | 200 | HTTP/1.1 | real content | 200 |

**Two corrections to the premises this epic was filed on:**

1. **`stackoverflow.com/questions` no longer reproduces.** It returned HTTP 200
   with real content on all 3 runs. The TLS/header-casing gate that motivated
   `firefox_tls.rs` — and which `ARCHITECTURE.md` still cites as proven — **is
   not firing from this egress IP today.** It may have been fixed, may be
   IP/reputation-scoped, or may never have been reproducible. Recorded as *not
   currently reproducing*. **No part of this design may be justified by it
   without re-isolating it first.**
2. **Amazon's 202 soft block does still reproduce**, and frot **mislabels it
   `needs:["js"]`**. Verbatim: `{…"status":"needs","needs":["js"],"http":{"status":202}}`
   with no `out` key at all. The body is empty because Amazon *withheld* it, not
   because the page needs JS. This is a `needs.md` defect, not an identity one —
   see §15.
   > **Corrected 2026-07-22 (`bl-7e34`, §3.9).** "Withheld, cause unknown" was
   > wrong. The 202 carries **`x-amzn-waf-action: challenge`** — AWS WAF
   > *declaring* a challenge, the same transport fact as reddit's `retry-after`
   > in a different vendor's spelling. It is a **declared challenge (§3.7), not
   > a soft block**, and is now reported `needs:["human"]` pre-parse.

### 3.7 Negative controls — declared challenges (one request each, never executed)

| site | frot envelope | HTTP | declaring header |
|---|---|---|---|
| reddit.com | `needs:["human"]` — **correct** | 200 | `retry-after: 0`, `server: snooserv` |
| g2.com | `error{kind:"http.403"}` | 403 | `x-datadome: protected`, `server: cloudflare` |
| www.amazon.com/ (added `bl-7e34`, 2026-07-22) | `needs:["human"]` — **correct since `bl-7e34`** | 202 | `x-amzn-waf-action: challenge`, `server: CloudFront` |

Both are the same real-world condition — *a bot defence refused us* — and a
consumer must handle **two envelope shapes** to detect it. A 403 challenge was
**indistinguishable from a genuine 403**, because the distinguishing signal
lives in response headers and frot's envelope discarded them. **Resolved
(`bl-acec`, 2026-07-20):** the `http` block is now `{status, headers}`, and the
allowlist surfaces exactly these markers (`server`, `x-datadome`,
`retry-after`, `cf-mitigated`), so g2's bot-defence 403 carries
`x-datadome: protected` / `server: cloudflare` where a genuine origin 403 does
not — the two are now distinguishable in one envelope shape. The *reporting*
still differs (one `needs`, one `error`); only the evidence gap is closed. See
§15.

### 3.8 Post-transport capstone re-measurement (`bl-d66b`, 2026-07-21)

Measured after **every** identity sibling landed — `bl-abca` (Option C: stock
`rustls 0.23` + `aws-lc-rs` + real h2), `bl-20ec` (one request serializer),
`bl-5191` (per-invocation session), `bl-3972`/capability balls (JS persona,
incl. `bl-8733` audio), `bl-e707` (clocks) — from the **same reference egress**
as §3.1 (§3, verified identical), release binary from the
`bl-d66b` worktree, `cargo build --release`. Wire persona read black-box through
`tls.peet.ws/api/all` (the §3.1 tool). This is the §14 falsifier run.

**Transport, frot then → now (same oracle, same egress):**

| field | frot 2026-07-19 (pre-transport) | **frot 2026-07-21 (as-built)** | Firefox 140.12.0esr (the pin) |
|---|---|---|---|
| negotiated version | HTTP/1.1 | **h2** | h2 |
| ALPN | `["http/1.1"]` | **`["h2","http/1.1"]`** (h2 selected) | `["h2","http/1.1"]` |
| JA4 | `t13d1714h1_5b57614c22b0_abe81bfac2ff` | **`t13d1011h2_61a7ad8aa9b6_f9531d972513`** | `t13d1717h2_5b57614c22b0_3cbfd9057e0d` |
| cipher count | 17 | **10** | 17 |
| TLS extension count | 14 | **11** | 17 |
| key shares | 1 | **≤2** | 3 |
| advertised UA | Firefox 121.0 | **Firefox 140.0** | Firefox 140.0 |
| `Accept` value | `…image/avif,image/webp…` (Chrome-shaped) | **`text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8`** | *identical* |
| `Accept-Encoding` | `gzip, br` | **`gzip, br`** (declared residual, §14 item 6) | `gzip, deflate, br, zstd` |
| header casing | scheme-dependent (§3.4) | **one serializer, scheme-independent** | — |
| `sec-fetch-*` / `priority` / `te` | absent | **present** (`u=0, i` / `trailers`) | present |
| h2 SETTINGS | none (no h2) | `2:0;4:131072;5:16384;6:16384` (declared residual, §11 — see note below) | `1:65536;2:0;4:131072;5:16384` |
| h2 pseudo-order | none | **`m,s,a,p`** (declared residual, §7 stage C) | `m,p,a,s` |
| connections / page (`wiki --css`) | **3** (§3.5) | **1** (session pool reuse, `bl-5191`) | 1 (h2 mux) |
| stripped static-musl size | 7.25 MiB | **7.63 MiB** (hyper+tokio+h2 net +~0.4 MiB) | — |

**h2 SETTINGS follow-up (`bl-f312`, 2026-07-22).** The SETTINGS row above was
recorded honestly but left *undeclared* — §14 named only pseudo-order/PRIORITY,
so the set mismatch was a §12 drift by this doc's own rule. It is now confirmed,
partly fixed and fully declared. Re-measured offline through the production
transport (`src/fetch/transport/h2_preface.rs`, an ALPN-`h2` throwaway-CA origin
reading frot's raw preface — no network, so it runs in CI): the wire is exactly
`2:0; 4:131072; 5:16384; 6:16384`, reproducing the live `tls.peet.ws` capture.
**Fixed:** the connection WINDOW_UPDATE increment was 12451842, not the 12517377
the profile declared and Firefox sends — hyper's
`http2_initial_connection_window_size` is a *target* window, from which `h2`
subtracts the RFC 9113 default 65535 to get the increment, so frot was off by
that much in the akamai-h2 fingerprint's second field. `transport.rs` now targets
increment + 65535 and the wire increment matches Firefox exactly. **Declared:**
ids 1 and 6 are blocked by `hyper-util` and are now §11 residuals asserted in the
oracle. **Reconciled:** `profile.rs` no longer claims the whole SETTINGS set is
enforced — each `H2Profile` field's doc now says which of enforced /
oracle-enforced / declared-residual it is.

**JS persona coherence (live, `--js`, `js.errors:0 settled:true`):**
`navigator.userAgent` = the *same* `…rv:140.0…Firefox/140.0` string as the wire
UA header (no version contradiction, the §3.3/§3.5-era tell), `platform`
`Linux x86_64`, `language` `en-US` (matches `Accept-Language`),
`hardwareConcurrency` 8; `canvas.toDataURL()` a real PNG, `WebGLRenderingContext`
and `OfflineAudioContext` both `function`, `webkitAudioContext` `undefined`
(coherent — Firefox has no alias). The deterministic half of this is asserted in
CI (`src/js/tests/persona_gold.rs`, cross-invocation stable).

**The §14 verdict — measured, not dressed up.**

1. **Coherence: achieved.** The pre-phase capture's dominant tells are gone —
   the h1-under-a-Firefox-UA contradiction (h2 now), the Chrome-shaped `Accept`,
   the scheme-dependent header casing, the Firefox-121/140-cipher mismatch, and
   the one-connection-per-request anomaly (3→1). frot now presents *one*
   internally consistent Firefox-140esr identity across TLS, ALPN, HTTP/2,
   headers, and `navigator`. This is the win the epic was actually premised on
   (§6.3/§14) and it is real.
2. **Byte-exact fingerprint: not achieved, by design (Option C).** JA4 is
   `t13d1011h2_…`, **not** the pin's `t13d1717h2_…`: stock rustls emits 10
   ciphers (not 17), 11 extensions (not 17), ≤2 key shares (not 3); the `h2`
   crate emits `m,s,a,p` (not `m,p,a,s`); `Accept-Encoding` stays `gzip, br`.
   Every one of these is a **declared residual** (§6.4/§7/§11/§14), asserted in
   the §12 oracle — never a silent miss. Consistent with §1's "coherent, current,
   maintained — explicitly **not** byte-identical."
3. **Access outcomes: NULL — no gate changed.** The field corpus is
   **byte-for-outcome identical** to the pre-transport §3.6/§3.7 baseline:
   controls (wikipedia/lobste.rs/HN/react.dev) still `ok` with real content;
   StackOverflow still `200` real content (its gate **still does not reproduce**
   from this IP — do not cite it); Amazon still `202` soft-block (still
   mislabelled `needs:["js"]` — the `needs.md`/§15 defect, unchanged and now
   carrying `server: CloudFront`); reddit still `needs:["human"]`, g2 still
   `error{http.403}` carrying `x-datadome: protected`. **No target that failed
   before now succeeds, and none that succeeded now fails.** This is the
   publishable null §14 item 1 predicted. The justification standing on measured
   ground is **coherence + security maintenance (§6.3)**, not access improvement.
4. **Controls and gates intact.** All 757 tests pass, 100% line+region coverage,
   clippy clean, static-musl links (`static-pie`, `ldd` = not a dynamic
   executable) and fetches live over h2; size 7.63 MiB, inside the 5–15 MiB
   envelope. No artificial delays, no bypass, no per-site shim was added to force
   a result — the harness reports the null honestly (`examples/ab_harness.rs`).

**Residual failure classification (by evidence, per the stop rule).** The
soft-2xx and challenge cases that remain are not client-profile failures: Amazon
202 is a *withheld body* the server chose (a `needs`-labelling defect, §15
item 2, owned elsewhere), reddit/g2 are *declared challenges* frot correctly
stops before (§3.7, `needs:["human"]`/`error` honesty preserved). The residuals
*inside* the fingerprint a serious defence hashes (cipher breadth, extension
set, key shares, pseudo-order) are the Option-C ceiling (§6.4) — a rustls-fork
cost §6.3 declines to pay for an unproven access gain. IP/ASN reputation (§14
item 2) is untested here and out of scope.

### 3.9 Amazon re-measured under the full persona (`bl-7e34`, 2026-07-22)

**Question asked.** Every capability ball `bl-bd4e` found Amazon's 202 page
probing has since landed (canvas `bl-05e6`, WebGL `bl-f624`, audio `bl-8733`,
crypto `bl-cf3a`, screen `bl-1cb7`, Worker `bl-342a`, `navigator.webdriver =
false`). §3.8's capstone measured the 202 with `--out text` and no `--js`, so it
never exercised the masquerade against the fingerprinter. **Does Amazon still
202 now that the persona answers its probes coherently?**

**Method.** The reference egress (§3) — the same address as §3.1/§3.8,
verified. Release binary from the `bl-7e34` worktree.
`frot https://www.amazon.com/ --js --out text` ×3 and `--out dom` ×1, then the
same request replayed through `curl` with frot's exact header set (read back
from `httpbin.org/headers`) to see the response headers frot's allowlist did
not yet surface.

**Result — yes, still 202, and the cause is now named by the server itself.**
All 3 `--js` runs: `status:"needs"`, `needs:["js"]`, `http.status: 202`,
`server: CloudFront`, `js:{scripts:3, errors:2, settled:true}`. The header
replay is decisive:

```
HTTP/2 202
x-amzn-waf-action: challenge
access-control-expose-headers: x-amzn-waf-action
server: CloudFront
content-length: 2007
```

and the 2007-byte body is an **AWS WAF challenge page**: `window.gokuProps`
(`key`/`iv`/`context`) plus `<script src=".../token.awswaf.com/…/challenge.js">`.

**Three premises die here.**

1. **The body is not withheld.** frot receives a real 2007-byte document. It
   reported `needs:["js"]` because that document has scripts and no rendered
   text — the §4 starvation heuristic doing exactly what it says. The header,
   not the body, was the missing evidence.
2. **It is not a "soft block" of unknown cause.** It is a **declared
   challenge**, the §3.7 category, and belongs in `needs.md` §3's existing
   mechanism. Fixed there — no new `needs` kind, no third envelope shape
   (§15 item 3 unaffected).
3. **`server: CloudFront` is not the separating signal**, contrary to this
   ball's filing premise. CloudFront fronts an enormous amount of genuine
   content; keying on it would flag all of it. The vendor **action** header is
   the signal, and it is structural, not textual (§3.6 Trap #1 respected).

**What actually blocks access — classified by evidence, per the §14 stop
rule.** Not an absent capability surface: the masquerade never got to speak,
because Amazon's WAF issues the challenge **at the transport, before any
fingerprint is read** — the 202 is the *first* response to a bare navigation.
The gate is a **JS proof-of-work / token challenge**: `challenge.js` computes a
token from `gokuProps` and posts it to `token.awswaf.com` to mint an
`aws-waf-token` cookie the retry then carries. Getting past it requires
**executing the challenge and submitting the result**, then re-issuing the
request — none of which `run.rs` does today (`docs/design/challenge.md`).

**So: there is no capability gap to file for Amazon.** No masquerade
improvement opens this gate, because the gate is not asking the client what it
can do — it is asking it to perform work and POST the answer. `needs:["human"]`
is the honest terminal verdict, and frot now reaches it **without executing the
challenge at all** (pre-parse flip: 3 fewer script executions and 2 fewer JS
errors than before this ball). §14 item 1's "null on access" verdict stands unchanged and
is, for this target, now *explained* rather than merely observed.

**Followed up.** Mark's directive filing this ball was *"ultimately, we need
to be able to get around it."* The design answering it is
`docs/design/challenge.md` (`bl-017a`).

---

## 4. The one profile

`BrowserProfile` is a single `const` (proposed home: `src/fetch/profile.rs`).
**Every fact below has exactly one definition site.** Nothing here is stored
twice, and nothing derived from it is stored at all.

Today these facts are scattered across four files, which is the defect:
`USER_AGENT` at `src/fetch.rs:26`, `FIREFOX_105` in `src/fetch/firefox_tls.rs`,
`productSub`/`platform`/`language` at `src/js/prelude/env.js:111-115`, and
`Accept-Language` in `fetch.rs` disagreeing with `language` in `env.js`.

### 4.1 Stored — the profile constant

| fact | value (Firefox 140.12.0esr) |
|---|---|
| `version_major` | `140` |
| `pinned` / `eol` | `2026-07-19` / *(ESR calendar, filled at implementation)* |
| `user_agent` | `Mozilla/5.0 (X11; Linux x86_64; rv:140.0) Gecko/20100101 Firefox/140.0` |
| `locale` | `en-US`, fallback `en` |
| `ciphers` (17, wire order) | `1301,1303,1302,c02b,c02f,cca9,cca8,c02c,c030,c00a,c009,c013,c014,009c,009d,002f,0035` |
| `extensions` (17, wire order) | `0,23,65281,10,11,35,16,5,34,18,51,43,13,45,28,27,65037` |
| `supported_groups` | `4588,29,23,24,25,256,257` |
| `key_share_groups` | `4588,29,23` — **3 shares** |
| `signature_algorithms` (11) | `0403,0503,0603,0804,0805,0806,0401,0501,0601,0203,0201` |
| `alpn` | `["h2","http/1.1"]` |
| `record_size_limit` | `16385` (`0x4001`) |
| `cert_compression` | `zlib(1), brotli(2), zstd(3)` |
| `grease` | **none** |
| `h2_settings` (4, in order) | `HEADER_TABLE_SIZE=65536, ENABLE_PUSH=0, INITIAL_WINDOW_SIZE=131072, MAX_FRAME_SIZE=16384` |
| `h2_window_update` | `12517377` |
| `h2_initial_stream_id` | `3` |
| `h2_headers_priority` | `PRIORITY` flag set, `weight=42, depends_on=0, exclusive=0` |
| `h2_pseudo_order` | `m,p,a,s` |
| `nav_headers` (order) | `Host, User-Agent, Accept, Accept-Language, Accept-Encoding, Connection, Upgrade-Insecure-Requests, Sec-Fetch-Dest, Sec-Fetch-Mode, Sec-Fetch-Site, Sec-Fetch-User, Priority` |
| `accept_document` | `text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8` |
| `accept_encoding` | `gzip, deflate, br, zstd` — **subject to I2**, see §6.3 |
| `js_build_id` | `20181001000000` — Gecko's *privacy-frozen* constant, not the real BuildID |
| `js_hardware_concurrency` | `8` — a pinned low-entropy constant, see §8 |

**Where this table lives (`bl-7523`, 2026-08-12): `src/fetch/profile/pin.rs`,
and nothing else is in that file.** It was split out of `profile.rs` — which now
holds only the shapes and the derivations — so the claim below that "a pin change
is an edit to §4.1 and nothing else" is literally true of the tree: it is an edit
to one file. The same ball moved four rows that were declared here but stored
nowhere into the const, because a fact the oracle must read cannot live only in a
document: `ciphers` and `extensions` (the full 17-entry ordered lists, which JA4
hashes), `record_size_limit`, and `cert_compression`. `h2_initial_stream_id`
moved in for the same reason. `cipher_count` was **deleted** in exchange — with
the list stored, a count beside it was the same fact twice (I1/I6).

### 4.2 Derived — computed, never stored

| derived fact | from |
|---|---|
| `Accept-Language: en-US,en;q=0.5` | `locale` (the q-value is Gecko's rendering of `en-US, en`) |
| `navigator.language` / `.languages` | `locale` — **the same source as the header**, which is why they can no longer disagree |
| `navigator.userAgent` / `.appVersion` | `user_agent` (`appVersion` = UA minus the `Mozilla/` prefix) |
| `navigator.platform` / `.oscpu` | the UA's platform segment |
| every JA4/JA4_r/JA4_ro and the akamai-h2 fingerprint | **computed from the capture** by `ja4.rs` / `h2_wire.rs`, never hand-written (§12). JA3/JA3N/peetprint are **not** computed — §12's 2026-08-12 correction says why |
| h1 header casing | the profile's canonical names — Title-Cased for h1, lowercased for h2, by the *one* serializer |
| ALPN offer | intersected with what the transport can actually speak (I2) |

Note what §4.2 buys: `firefox-esr140.md` lists "8 hash values that must be
regenerated" if the pin changes. Under this design there is **nothing to
regenerate** — the ordered capability lists are authoritative and the hashes fall
out. A pin change is an edit to §4.1 and nothing else.

---

## 5. Invariants

Each is a testable assertion, not a guideline.

- **I1 — One definition site.** No version literal, UA substring, cipher/group/
  extension id, HTTP header value, or `navigator` fact appears anywhere but the
  profile constant. `env.js` receives its facts through a syscall
  (`__frot_env_profile`), the way it already receives the UA — there is no second
  copy in JS.
- **I2 — Never advertise what you cannot speak.** Three sub-rules, one principle:
  ALPN ⊆ protocols the transport implements; `Accept-Encoding` ⊆ encodings the
  transport can decode; every group in `supported_groups`/`key_share` genuinely
  negotiable by the crypto provider. **This is not a preference — it is what
  makes the client coherent rather than a costume**, and it is why `bl-abca` may
  not flip ALPN to `h2` before h2 works (§7). The existing `Accept-Encoding`
  comment in `fetch.rs` already states this rule for content-encoding; I2
  generalizes it.
- **I3 — Identity is scheme- and transport-invariant.** The same profile yields
  the same header names, order, and values over `http` and `https`, and over h1
  and h2 (modulo h2's mandatory lowercasing and its pseudo-header block). This
  invariant is exactly what §3.4 violates today.
- **I4 — Identity is destination-derived, not seam-derived.** The header set is a
  pure function of `(profile, destination)` where destination ∈ `{document,
  style, script, empty}`, computed in one place. It is never a function of which
  connector, agent, or call site the request happened to traverse.
- **I5 — `-H` is a value override, not a profile switch.** A caller's
  `-H "User-Agent: …"` replaces the UA string in *both* the request and
  `navigator.userAgent` (already true via `fetch::user_agent` — the shim never
  lies about who fetched) and changes **nothing else**. There is no second,
  hidden profile system and no `--profile` flag.
- **I6 — Derived facts are computed.** Anything in §4.2 that appears as a stored
  literal is a bug.
- **I7 — Profile selection is a constant.** No flag, no env var, no config file.
  Severability: removing a capability deletes config, never edits core.
- **I8 — The profile expires.** `tests/hygiene/persona.rs` asserts
  `today < profile.eol` against the **wall clock**, and — as a negative control —
  that the pin *does* go stale one ESR horizon out, so the gate cannot be
  snoozed by pushing `eol` past the calendar. When the pinned ESR line goes
  end-of-life, `cargo test` fails, forcing a re-capture. No calendar, no
  reminder, no knob.

  *Corrected 2026-08-11 (`bl-04ab`): this read "the build fails", and nothing
  fired at all — `is_current` had no production caller and every test caller
  passed a fabricated `SystemTime`, so no code ever asked whether the pin was
  stale today. The gate now exists and it is a **test** failure, matching §2's
  wording. It is deliberately not a build-script clock read: that would make
  every compile consult the calendar to catch, later, exactly what the test run
  catches now, and `cargo build` is the one command that must not depend on what
  day it is.*

---

## 6. Transport route

### 6.1 Decision

**SELECTED and LANDED (Mark's Option-C ruling, 2026-07-21; `bl-abca`): retire
`craftls`, move to STOCK `rustls 0.23 + aws-lc-rs`, and add real `h2` over hyper
behind the `firefox_tls.rs`/`FetchSession` seam (§7).** The pure-Rust ML-KEM
path, config (c) below, is **preserved as the size-optimized fallback**.

> **Correction (2026-07-21, as-built).** The 2026-07-20 framing said "port the
> craft layer into frot's own repo." When `bl-abca` came to implement it, the
> escalation the ball was designed around fired: **stock rustls 0.23 has no
> ClientHello-crafting API** (upstream issues #1932/#1421/#2498 remain open —
> that is *why* craftls exists), and "porting the craft layer" would mean frot
> owning a rustls fork — re-creating the exact maintenance/CVE liability §6.3
> gives as the whole reason to leave craftls. Mark ruled **Option C**: ship stock
> rustls and **demote the ClientHello order/set to a declared §12 residual**
> rather than fork. frot's handshake is now coherent, current and `cargo
> update`-maintained; it is **explicitly not byte-identical to Firefox** (§1),
> and **JA4 does not match the pin** — accepted, and asserted as a residual, not
> hidden.
>
> **What stock rustls + aws-lc-rs *does* reproduce from the profile:** ALPN order
> `h2, http/1.1`; kx-group order with X25519MLKEM768 first; cipher wire order;
> absence of GREASE (140esr has none); the h2 SETTINGS initial-window / max-frame
> values. **Declared residuals (asserted in §12, never silent):** the cipher
> *list* is rustls's 9 AEAD suites, not Firefox's 17 (so the JA4 *cipher*
> component also differs, not only the extension component); the ClientHello
> extension order/set; secp521r1 + the two FFDHE groups; ≤2 key shares not 3;
> h2 pseudo-order `m,s,a,p` not `m,p,a,s`; no HEADERS PRIORITY.
>
> **Async, as-built.** Mark's async ruling illustrated "current-thread tokio";
> the transport uses a **multi-thread** runtime because `run/gather.rs` calls the
> blocking fetch API concurrently from up to six OS threads (a current-thread
> runtime cannot be `block_on`'d concurrently). The observable contract is
> unchanged — blocking API, runtime owned by the session and dropped at
> invocation end, no work escapes the call.

**Measured, static-musl stripped, verified 2026-07-21 (`bl-abca`):**

| stage | stack | binary | crates | vs today (craftls 7,328,856 B) |
|---|---|---|---|---|
| Stage A | stock rustls 0.23 + aws-lc-rs, ureq kept, ALPN h1 | **7,304,472 B** | 95 | **−24 KiB, −16 crates** |
| Stage B — SHIPPED | hyper h1+h2 + tokio, ureq dropped, ALPN h2 | **7,820,696 B** | 118 | **+480 KiB, +7 crates** |

Stage A confirms the doc's "seam-swap ≈ −32 KiB" thesis (measured −24 KiB).
Stage B is the honest full-h2 figure the §6.1-item-3 caveat said was owed: real
h2 (hyper + hyper-util + hyper-rustls + tokio + h2) plus the gzip/br decoders
that replace ureq's cost **+480 KiB over today**, at **7.46 MiB — comfortably
inside the 5–15 MiB envelope** (close to the §6.2 spike's projected +291 KiB).

The checkpoint this section once carried gated three items. Two are now settled
by Mark's rulings of 2026-07-20 (async, and the C-stack choice, both below); the
third — content-encoding decoders — is an owed *measurement*, not an open
*approval*. So the dependency checkpoint is discharged with evidence here rather
than left pending. `bl-abca` unblocks once the decoder measurement (item 3) lands.

#### Item 1 — the C stack: `aws-lc-rs`, measured 2026-07-20

> **Correction (Mark's ruling, 2026-07-20). `ring` is not pure Rust**, and the
> earlier framing of this move as "pure Rust → C" was wrong. ring 0.17.14 carries
> **17 C files (5,413 lines), 90 pregenerated asm files (156,641 lines) and 38
> perlasm generators** against 28,240 lines of Rust — most of it BoringSSL-derived.
> The migration is **C+asm → C+asm**, not "pure Rust → C". (The same error is
> corrected in `Cargo.toml`'s comment and in `ARCHITECTURE.md`.)

frot was built **for real in three configurations**, static musl, stripped, each
verified fetching live sites 2026-07-20:

| config | stack | binary | crates | vs today |
|---|---|---|---|---|
| **(a)** today | ureq + craftls/`ring` | **7,328,856 B** | 111 | — |
| **(b) — SELECTED** | rustls 0.23 + `aws-lc-rs` at `firefox_tls.rs` | **7,296,312 B** | 96 | **−32 KiB, −15 crates** |
| **(c)** preserved fallback | rustls 0.23 + `ring` + `libcrux-ml-kem` custom kx group | **5,538,872 B** | — | −1.71 MiB |

**Mark selected (b)** on 2026-07-20. So the selected path is **32 KiB smaller
than today and sheds 15 crates.** The earlier
"+2.1 MiB for aws-lc-sys" / "+291 KiB for the route" figures were arithmetically
right and both misleading: `aws-lc-sys` genuinely costs ~2.1 MiB in isolation,
but retiring craftls sheds ~2.13 MiB of baggage (`zstd-sys` + `zstd` +
`zstd-safe` + `brotli 3.5` + a second `brotli-decompressor` + `ring` +
`rustls-webpki 0.102`), almost exactly cancelling it.

Config **(c)** — the pure-Rust ML-KEM path — **works and is preserved as a
fallback, not a hypothesis.** A ~90-line custom `SupportedKxGroup` over the ring
provider backed by `libcrux-ml-kem` links static-pie musl and completes real
handshakes against google.com and cloudflare.com with
`negotiated_kx: X25519MLKEM768`. `libcrux-ml-kem` is formally verified **and
vendored by NSS**, so it is the ML-KEM implementation Firefox itself uses. It was
**not selected** (it is the preserved fallback) because it keeps semi-abandoned
`ring` (self-described "an
experiment", no release in 16 months) and takes permanent custody of hybrid-KEM
glue plus an unbuilt HPKE shim for ECH — the craftls failure mode one layer down.

#### Item 2 — async runtime: **SETTLED** (Mark's ruling, 2026-07-20)

> **Superseded — the old position.** `ARCHITECTURE.md` recorded *"Blocking I/O,
> no async runtime — a single-shot process fetching a handful of resources gains
> nothing from tokio and pays startup + size for it."* Mark superseded it in part
> on 2026-07-20; it is not deleted, it is amended.

Mark ruled: *"it's okay to run async internally to resolve a request more
efficiently, just the returns to the user must be posix-compliant,
lifecycle-deterministic."* An internal `tokio` current-thread runtime is
therefore **permitted** where it buys real concurrency (h2 multiplexing,
`bl-08f6`). The constraint is on the **observable contract**, not the
implementation — and it is **testable, so assert it** (a new I-class invariant
for `bl-d66b`'s oracle):

- the public fetch API stays **blocking**;
- **no work escapes the call** — no detached tasks;
- **no runtime outlives the invocation** — no background reactor surviving return;
- process lifecycle stays **deterministic**.

Startup + RSS delta is still **unmeasured** and stays a **live cost to record
once the runtime lands** — that half of the old rationale is a real cost, not a
prohibition. Measure it in `bl-abca`/`bl-08f6` and record it here.

**Landed (`bl-08f6`).** The concurrency this ruling permits is now exercised: one
bounded primitive (`fetch::fetch_many`) fans a wave of subresource fetches over
the shared pool from `std::thread::scope`, which **joins every worker before it
returns** — the "no work escapes the call" invariant, asserted directly (a wave
against a dead host with a short deadline returns promptly with nothing, no
request left running). It drives both the CSS gather and the JS initial-script
preload warm; the runtime still lives on the `Transport` inside the
`FetchSession`, so it dies with the invocation. h2 multiplexing is proven by a
barrier oracle: four requests that each wedge until all four are present come
back over **one** accepted connection — reason 3 of the §7 h2 call confirmed,
overturning-condition (c) not met.

#### Item 3 — content-encoding decoders: RESOLVED (Mark's steer, 2026-07-21)

**Decision: keep advertising only `gzip, br` — what frot actually decodes — and
leave `zstd`/`deflate` a declared residual (§14 item 6).** This satisfies I2 (do
not advertise what you cannot inflate) without incurring the size of extra
decoders. As-built, `bl-abca` reimplemented the gzip/br decoders (`flate2` +
`brotli`) that ureq used to provide, and the transport sets `Accept-Encoding:
gzip, br` on every request. So the settled Stage-B figure **7,820,696 B**
(measured, table above) already includes the decoders; the earlier "−32 KiB" was
the seam-swap only and is superseded. The Firefox persona's fuller
`gzip, deflate, br, zstd` offer is the residual — a coherent *subset*, honestly
advertised, not a false claim.

### 6.2 Route table (measured 2026-07-19; `lto=fat, codegen-units=1, strip, panic=abort`)

> **Superseded on the size axis by the 2026-07-20 remeasurement (§6.1 item 1).**
> The `+291,040 B (+284 KiB)` route figure and the `+1 crate net (51 → 52)` delta
> below are the 2026-07-19 spike's `r1b` numbers; the three-config rebuild of
> 2026-07-20 supersedes them with **config (b) at −32 KiB / −15 crates vs today**.
> The rows are kept for provenance and for the *qualitative* verdicts (which are
> unchanged); read §6.1 for the authoritative sizes and crate counts. The
> `7,111,056 B` baseline here was also remeasured to **7,328,856 B** (§6.1).

frot today: **7,111,056 B (6.78 MiB)**, **111 crates** *(remeasured 2026-07-20 to
7,328,856 B — see §6.1)*.

| route | pure Rust? | last release | static musl | Δ vs current transport (musl) | crates | new C/C++ | verdict |
|---|---|---|---|---|---|---|---|
| **R1 as-is** — keep `craftls` | **no** — `ring` is C+asm, `zstd-sys` is C | **2024-01-16**, sole release | yes (ships today) | 0 | 51 | yes (`ring`, `zstd-sys`) | **REJECT** — no ML-KEM, no ECH, rustls 0.22, abandoned |
| **R1-mod** — rustls 0.23 + aws-lc-rs + `h2` | **no** — `aws-lc-sys` vendors C+asm (as `ring` already did) | 2026-07-13 / 07-17 / 06-15 | **YES — built and ran** static-pie; printed `X25519MLKEM768(4588) present: true` | **−32 KiB / −15 crates** *(config (b), §6.1; supersedes the spike's +284 KiB)* | 96 total | yes (`aws-lc-sys`) | **SELECTED** (Mark, 2026-07-20), decoders owed |
| **R2** — `wreq` | **no — BoringSSL C++** | active | **NO — build fails**, no `x86_64-linux-musl-g++` | +1.29 MiB (glibc); musl n/a | **115** *(> frot's entire tree)* | yes, + cmake | **REJECT on constraints** — best fidelity, fails static musl + blocking I/O |
| **R3** — curl-impersonate | no (C) | active | not verified — no Rust crate | not measured | n/a | whole C stack + own BoringSSL | **REJECT** — heaviest possible escalation |
| **R4** — `warpsock` | no — `boring` + `tokio` | 2026-07-07 | **NO** — same C++ blocker | not verified | 84 | yes | **REJECT** — 480 downloads, 11 stars, **2 contributors**, renamed mid-flight; unacceptable for a TLS stack |

Raw stripped musl sizes (2026-07-19 spike): `cur` (frot's transport today)
4,227,800 → `r1b` (ureq + rustls 0.23/aws-lc-rs + h2 + tokio) 4,518,840, both
`static-pie, statically linked`, both **ran OK**. *(These are the superseded
spike figures; the 2026-07-20 whole-binary rebuild in §6.1 is authoritative and
lands config (b) 32 KiB **below** today.)* Projected frot total is inside the
5–15 MiB envelope with room on every config.

Tree delta (`cargo tree --edges normal`, unique name+version): the spike reported
**+1 crate net** (51 → 52 transport-subtree crates); the 2026-07-20 whole-binary
rebuild reports **−15 crates** (111 → 96) — the seam swap removes `craftls`,
`ring`, `rustls-webpki 0.102`, `brotli 3.5`, a second `brotli-decompressor`, and
`zstd`/`zstd-safe`/`zstd-sys`, and adds `aws-lc-rs`/`aws-lc-sys` plus (under
stage B) `h2` + `tokio`. *Caveat unchanged: neither figure includes the
content-encoding decoders I2 requires (§6.1 item 3), so the settled number is
(b) + decoders and has not been measured.*

### 6.3 Security maintenance — the independent argument

**`craftls` must be retired on security grounds alone, regardless of every other
decision in this document.**

| stack | last release | maintainers | CVE path to frot |
|---|---|---|---|
| **`craftls`** | **2024-01-16**, sole release ever; last commit 2024-01-19, **~2.5 yr stale**, 27 stars | effectively 1 | **BROKEN — none.** It is a fork of rustls **0.22**. A rustls CVE does not reach frot *at all*; frot would have to hand-port the fix into a dead fork. |
| `rustls` | 0.23.42, 2026-07-13, ~monthly | 7.5k stars, RustSec-wired | `cargo update` |
| `aws-lc-rs` | 1.17.3, 2026-07-17, ~biweekly | AWS-backed, 51 contributors | `cargo update`; AWS advisories |
| `h2` | 0.4.15, 2026-06-15 | hyperium, 98 contributors | `cargo update` |
| `wreq` | very active | **essentially one person**, over **`boring2`** — that author's *personal fork* of Cloudflare's `boring` | **two single-maintainer hops** to a vendored BoringSSL |

This is the single worst fact in the current design and the one item here that
does not depend on the corpus, the persona, or the h2 question.

**ACHIEVED (2026-07-21, `bl-abca`).** craftls is retired; the transport is stock
`rustls 0.23` + `aws-lc-rs` + `h2`/hyper, all on crates.io with the maintenance
posture above. A rustls CVE now reaches frot by `cargo update`. Crucially, this
property survives *because* Option C did **not** fork rustls to craft the
ClientHello — a frot-owned fork would have re-created the exact CVE-hand-porting
liability this section exists to remove. The price paid to keep it is the
byte-exact handshake, demoted to a declared §12 residual (§6.1).

### 6.4 What R1-mod still cannot do

Stated plainly so no reader mistakes the recommendation for a match:

- **Two key shares, not three.** rustls emits the hybrid plus its "free"
  component; Firefox emits 4588, 29, **and** 23. Closing this needs a rustls
  patch. *(JA4 does not encode key-share count; a deep inspector sees it.)*
- **Pseudo-order and HEADERS PRIORITY** — see §7.
- `padding(21)`: frot sends it where the 140esr capture does not. Padding is
  ClientHello-length-dependent and varies with SNI in real browsers too
  (frot's own extension hash differs between peet and browserleaks for this
  reason) — **not a defect**, and normalized out of the oracle (§12).

---

### 6.5 The connection pool stays rented — decided 2026-07-24 (`bl-fa12`)

**The question.** `hyper_util`'s pooled `Client` does not check an HTTP/1.1
connection back into the idle pool inline. It spawns a task
(`client/legacy/client.rs`):

```rust
if pooled.is_http2() || !pooled.is_pool_enabled() || pooled.is_ready() {
    drop(pooled);                                   // h2: checked in inline
} else {
    let on_idle = poll_fn(move |cx| pooled.poll_ready(cx)).map(|_| ());
    self.exec.execute(on_idle);                     // h1: checked in eventually
}
```

So a next same-origin request that arrives before that task is scheduled finds
an empty pool and dials a second socket. **Measured** (`bl-df88`, 16-core box,
48 spinning threads, 200 sequential same-origin pairs): 16/200 opened a second
connection; with a 5 ms gap, 0/200. No `hyper-util` knob makes check-in
synchronous — searched, it is not a config fix. The only route is to drop
`hyper_util::Client` and drive `hyper::client::conn` over a pool frot owns —
which would *also* unlock h2 SETTINGS id 1 `HEADER_TABLE_SIZE` (§11), since
`hyper::client::conn::http2::Builder` has `header_table_size` while
`hyper-util`'s `h2_builder` field is private.

**Decision: no. frot keeps `hyper_util::Client` and declares both residuals.**
The h2 half buys a fingerprint field that still will not match, and the h1 half
is behaviour a real Firefox also exhibits — so the trade is a wide correctness
surface bought with nothing.

#### The h2 half buys less than it looks like — verified against the sources

Read against `hyper-1.9.0` and `h2-0.4.15` rather than assumed:

| akamai-h2 fingerprint component | frot | Firefox 140esr | owning the pool fixes it? |
|---|---|---|---|
| SETTINGS id 1 `HEADER_TABLE_SIZE` | absent | `65536` | **yes** — `http2::Builder::header_table_size(impl Into<Option<u32>>)` |
| SETTINGS id 6 `MAX_HEADER_LIST_SIZE` | `16384` | *omitted* | **no** — hyper types it `u32` and applies it unconditionally (`proto/h2/client.rs`: `.max_header_list_size(config.max_header_list_size)`); `h2`'s own `client::Builder::max_header_list_size(u32)` can only set `Some`. Omission needs a fork of `h2`. |
| WINDOW_UPDATE increment | 12517377 | 12517377 | already matches (`bl-f312`) |
| HEADERS `PRIORITY` (weight 42) | absent | present | **no** — `h2` exposes no client PRIORITY |
| pseudo-header order | `m,s,a,p` | `m,p,a,s` | **no** — `h2`'s `frame::headers::Pseudo` is a struct whose *field order* is the wire order (`method, scheme, authority, path`) |

So owning the pool takes the akamai fingerprint from **four** mismatched
components to **three**. The hash still differs; every consumer that compares
the hash sees exactly what it saw before. That is the same trade §6.3 already
refused for the ClientHello — *a partial improvement to a hashed field that
still cannot hash equal is worth nothing*, and there the price was merely
keeping a fork alive, where here the price is a bespoke pool. The falsification
rule (§1) fires the same way it did for TLS: no route inside frot's constraints
reaches a byte-exact Firefox h2 preface, so the goal stays **coherent, not
byte-identical**, and id 1's absence stays a declared residual asserted in the
oracle (§12, `src/fetch/transport/h2_preface.rs`).

Note also what *is* coherent here. Omitting id 1 is not a lie: it means "my
HPACK decoder table is the 4096-byte default", which is true of frot's `h2`
decoder. The §5 invariant — *never advertise what you cannot speak* — is
satisfied by the omission, not violated by it. Only the *match* is missing.

#### The h1 half is inside the persona's own distribution

The re-dial is not a tell, for three independent reasons:

1. **Firefox opens up to six h1 connections per host.** That is where
   `POOL_PER_HOST = 6` comes from (`src/fetch.rs`; `concurrent.rs` calls it "the
   Firefox per-server limit"). A page load that opens one socket and then a
   second is *inside* the behaviour the persona is copying, not outside it.
   Contrast the §3.5 finding this design was filed on: N+1 connections for N
   requests, **never** reusing, is outside every browser's behaviour. An
   occasional extra socket is not the same class of fact.
2. **It cannot fire on h2, which is what the origins that fingerprint speak.**
   h2 takes the `drop(pooled)` branch above and checks in inline. Every
   defence frot actually meets — Cloudflare, Akamai, AWS WAF, DataDome — is
   ALPN-h2 (§3.8: frot negotiates h2 in the field). The residual is confined to
   legacy h1 origins, which by construction are not the ones hashing prefaces.
3. **The extra socket resumes the TLS session, exactly as Firefox's would.**
   `firefox_client_config` (`src/fetch/firefox_tls.rs`) does not override
   `ClientConfig::resumption`, and rustls 0.23's default is
   `Resumption::in_memory_sessions(256)`. One `ClientConfig` is built per
   `Transport`, i.e. per invocation, and shared by every connection, so a
   re-dial to an origin already visited in this invocation offers the ticket
   and takes the abbreviated handshake. The §3.5 worry — *"the TLS fingerprint
   is presented N+1 times per page"* — is therefore already answered on this
   path: the second presentation is a resumption, which is precisely what a
   browser does. **This is measured, not assumed** (`bl-17e7`):
   `src/fetch/transport/resumption.rs` makes one `Transport` dial the same
   throwaway-CA origin twice and asserts the origin's own
   `ServerConnection::handshake_kind()` — `Full` on the first connection,
   `Resumed` on the second. The re-dial is *forced*, not raced: the origin
   negotiates `http/1.1` by ALPN and answers `Connection: close`, so hyper
   retires the socket and the next request must dial afresh. The test goes red
   if `firefox_client_config` ever sets `Resumption::disabled()` or a
   dependency bump changes rustls's default.

It is also worth being exact about *what frot's contract observes*. The
re-dial changes the wire, never the envelope: the `--out *` payload, `needs`,
and the `http` block are byte-identical either way. So this does not fall under
the standing rule that host load must not decide frot's output (`bl-8dc0`) —
connection count is not an output.

#### What owning the pool would cost

Four behaviours `hyper_util::Client` supplies today would have to be rebuilt,
each with a live failure mode if it is rebuilt wrong:

- **ALPN-h2 connect dedup.** `Pool::connecting(&key, ver)` takes a per-key
  connecting lock and `Connecting::alpn_h2` converts it when ALPN comes back h2,
  so a six-wide wave to an h2 origin makes **one** connection. That number is
  measured (§3.8: `wiki --css` = 1) and asserted in CI
  (`run/gather/tests.rs`). A frot pool without this dedup opens six — a
  *regression on the very axis this ball is trying to improve*, and a louder
  tell than the one being fixed.
- **Liveness at checkout.** `pool.rs` drops idle entries whose `is_open()` is
  false (three separate sites) and expires on a 90 s timer. A pool that hands
  out a socket the origin has already closed turns a working fetch into an
  error — a user-visible correctness regression traded for a fingerprint
  nicety.
- **Checkout racing the dial.** `Client::connect_to` runs the pool checkout and
  the connect concurrently and takes whichever wins, cancelling the other; the
  pool holds a waiter queue for the losers.
- **Retry of canceled requests.** `retry_canceled_requests: true` re-issues a
  request that a reused connection dropped before it was ever sent — the
  ordinary keep-alive race every h1 client must handle.

Against `~/AGENTS.md`'s *build less* and *if it can't be tested, it mustn't be
built*: the coverage gate is 100% lines **and regions**, and every arm above is
a race. Reaching region coverage on "both threads missed the pool
simultaneously", "the dial failed with a waiter queued", "the idle socket died
between check-in and checkout" needs injected seams for each — mechanism added
to test mechanism added to match one SETTINGS entry that does not change a hash.
That is the definition of a compromise this repo says to push back on.

#### What frot does instead

Nothing new is built. The posture is *declare and assert*, which is what the
rest of this document does with every unreachable field:

- **h1 check-in is eventual** — a §11 residual row (added by this ball), the
  behaviour documented on `Transport::request_once`, and the invariant tests
  assert reuse as *eventual* (`session/tests.rs::same_origin_requests_reuse_a_
  pooled_connection`, `bl-df88`), never as "the second request rides the
  first's socket". A test that asserts a scheduler outcome is a flaky test, and
  it was one.
- **SETTINGS id 1 absent / id 6 present** — the existing §11 row, asserted *as
  residuals* in `src/fetch/transport/h2_preface.rs` so neither can drift
  silently. That row's claim that closing id 1 means *"dropping the pool or
  forking hyper"* was half wrong and is corrected there: dropping
  `hyper_util::Client` closes id 1; **id 6 needs a fork of `h2` either way.**

#### The trigger that flips this decision

Recorded so "revisit later" means something checkable:

1. **Upstream makes it free.** If `hyper-util` gains synchronous h1 check-in or
   an `h2_builder` passthrough, take it — it becomes a config edit at zero
   correctness cost, and both residuals close. This is the outcome to watch for,
   not to work around.
2. **A measurement, not a story.** If a field-corpus run shows the extra h1
   socket changing an *access outcome* — the §14 standard, evidence over
   folklore — the cost calculus changes and this section is rewritten. No such
   case exists today.
3. **Subsumption.** If frot ever forks or vendors `h2` for pseudo-order and
   PRIORITY (§7 stage C), id 6 and id 1 come along for that ride and the pool
   question is decided by that larger call, not this one.

#### What this decision does not solve

- frot's akamai-h2 fingerprint still does not match Firefox's, and after this
  decision it never will without a fork. Stated, not hidden (§11, §14 item 4).
- On a saturated host frot still occasionally opens a second h1 socket. The
  claim is that this is *unremarkable*, not that it is *absent*.
- The resumption argument above rests on rustls's defaults, which a future
  dependency bump could change silently. That is exactly why it needs the pin
  test, and why it is filed rather than merely written down.

---

## 7. The h2 decision — **add it, staged**

The evidence does not settle this; the two agents disagree. Reasoning in full.

**The case against (the spike's position):** `h2`'s `Pseudo` struct is hardcoded
`m,s,a,p` and it exposes no client API for the HEADERS `PRIORITY` flag. Getting
Firefox's `m,p,a,s` and weight-42 priority needs a fork — and forking is how frot
acquired `craftls`. *Do not fork speculatively.* Adding `h2` also drags `tokio`,
reversing an explicit architectural decision.

**The case for:** ALPN h1-only is the single dominant tell. A Firefox-shaped
ClientHello that refuses h2 is a combination **no real Firefox produces**.

**Weighing them.** The tempting framing — "is a nearly-right h2 fingerprint
better than an impossible ALPN?" — is a trap, because *both* are deterministic
tells. Firefox always negotiates h2; Firefox always sends `m,p,a,s`. Neither
residual hides. Three things break the tie:

1. **The oracle is unreachable without h2.** JA4 *embeds ALPN*:
   `t13d1717**h2**_…` vs `t13d1717**h1**_…`. An h1-only frot can **never** match
   the pinned profile's JA4 — not approximately, but by construction. §12's test
   oracle would be unsatisfiable on its most-cited field.
2. **The residuals are not equally shaped.** ALPN h1-only is a *contradiction*
   between two layers of the same client — the cheapest possible check, and
   unfixable while h1-only. `m,s,a,p` is a *deviation in one field* of an
   otherwise byte-exact h2 profile (SETTINGS, WINDOW_UPDATE, stream id, priority
   weight all match), and it is fixable by a small, well-scoped upstream API.
3. **h2 buys non-fingerprint value that h1 cannot.** §3.5 measures 3 requests →
   3 connections → 3 full handshakes → **the fingerprint presented 3 times**.
   Multiplexing collapses that to one, which is both what `bl-08f6` needs for
   concurrency and what removes the "never resumes a session" anomaly. This
   benefit is real whether or not the h2 fingerprint is perfect.

**Three stages — A and B LANDED in `bl-abca` (2026-07-21):**

| stage | ships | ALPN | status |
|---|---|---|---|
| **A** | stock rustls 0.23 + aws-lc-rs (no craft layer — Option C, §6.1); PQ group first, cipher/kx order from the profile | `["http/1.1"]` | **LANDED** (commit `17fed75`). Also landed the `BrowserProfile` single-source-of-truth. The ClientHello order/set is a declared residual, not crafted. |
| **B** | `h2` over hyper wired; ALPN flipped to `["h2","http/1.1"]`; ureq dropped | `["h2","http/1.1"]` | **LANDED** (commit `337a6f0`). ALPN-selected h2 and h1 fallback both verified against local TLS servers. `m,s,a,p` and the ClientHello residuals are asserted, not hidden (§12). |
| **C** (follow-up ball) | `m,p,a,s` + HEADERS PRIORITY | unchanged | **Prefer upstreaming** a small API to `h2` (`pseudo_order`, `headers_priority`). Fork only if upstream declines, and record it here if so. |

**The reframe that removes the flag.** Do not ask "should we offer h2?". The
profile *declares* `alpn: ["h2","http/1.1"]` (§4.1), and **I2 makes it a build
failure to declare a protocol the transport cannot speak**. Stage A satisfies I2
by intersecting the declared ALPN with implemented protocols; stage B widens the
implemented set. There is no policy decision left at the seam and no special
case — the general rule with a smaller input.

**What would overturn this call:** (a) upstream `h2` refusing the pseudo-order
API *and* a measured corpus A/B showing `m,s,a,p` scores worse than no-h2-at-all
— which would mean a wrong h2 fingerprint is more detectable than a missing one;
(b) a musl C++ cross-toolchain landing, making `wreq` viable and this entire
staging moot; (c) evidence that h2 multiplexing does **not** deliver `bl-08f6`'s
concurrency goal, removing reason 3 and leaving the decision on fingerprint
grounds alone.

---

## 8. The allowed JS surface (`bl-3972`)

**In: low-entropy, coherent, correct branding.** Facts a real Firefox 140esr
reports that frot can report truthfully or plausibly, all derived from §4.

Measured stock values to adopt, and frot's current gaps:

| fact | 140esr (measured) | frot today | action |
|---|---|---|---|
| `userAgent`, `appVersion`, `appName`, `appCodeName`, `product`, `productSub` | `…rv:140.0…`, `5.0 (X11)`, `Netscape`, `Mozilla`, `Gecko`, `20100101` | present, but **UA is 121** | derive from profile |
| `vendor` / `vendorSub` | `""` / `""` | `vendor` only | add `vendorSub` |
| `platform` / `oscpu` | `Linux x86_64` / `Linux x86_64` | `platform` only | add `oscpu` |
| `language` / `languages` | `en-US` / **`["en-US","en"]`** | `en-US` / **`["en-US"]`** | **fix** — derive both from `locale`, same source as `Accept-Language` |
| `doNotTrack` | **`"unspecified"`** | **`null`** | **fix** |
| `buildID` | **`20181001000000`** (privacy-frozen constant) | **absent** | add — reporting the *real* BuildID would itself be a tell |
| `pdfViewerEnabled` | `true` | absent | add |
| `plugins.length` / `mimeTypes.length` | `5` / `2` (Gecko PDF shims) | absent | add |
| `deviceMemory`, `userAgentData` | **absent** (`in navigator` → false) | absent | correct — keep absent |
| `maxTouchPoints`, `cookieEnabled`, `onLine` | `0`, `true`, `true` | matching | keep |
| **`webdriver`** | capture shows `true` — **Marionette's distortion, not stock** | **`false`** | **keep `false`. Real Firefox reports `false`; frot is under no remote control. Do not copy `true` into a golden capture.** |
| `hardwareConcurrency` | `16` — **host-dependent** | `1` | **pin `8`** — see below |

**`hardwareConcurrency`: pinned, not reported.** The real value is host-dependent
(this box: 16). Reporting the true host count would leak host entropy *and* make
frot's output non-deterministic across machines, which breaks VISION principle 1
(*calls are cacheable, reproducible*). Determinism decides it. But `1` is
implausible for a desktop Linux x86_64 in 2026 and contradicts the persona, so it
is not the honest choice either — pin a common desktop constant.

**Screen geometry stays frot's, not the capture's.** The 140esr capture reports
1366×768 — a *headless default*, not a persona fact. frot keeps its 1280×720
viewport constant (`layout.rs`), and `screen.*`/`window.*` derive from **it**, not
from the profile. Two viewport constants would be exactly the duplication this
design exists to prevent, and the existing `inner == outer` (no chrome) choice
stays as documented in `js.md` §7.

**LANDED (`bl-3972`, the Phase-5 framework task).** The whole supported subset
above is built and gated. All facts flow through **one** channel —
`__frot_env_profile()` returns a JSON payload the prelude parses once — so no
identity literal lives in `env.js` (I1), and `userAgent`/`appVersion` +
`language`/`languages` derive from the *effective* UA / `Accept-Language` (the
same source as the HTTP headers, `bl-20ec`), closing the UA-coherence gap. The
surface is **Firefox-shaped, not just correct in value**: `navigator`/`screen`/
`crypto` are branded interface instances (`[object Navigator]`, `instanceof`,
`@@toStringTag`) with facts as enumerable accessors on the prototype, and every
frot web API reads `function name() { [native code] }` through one
`Function.prototype.toString` wrapper + registry (`bl-3926`). Absorbed alongside:
`screen`/`devicePixelRatio`/visibility (`bl-1cb7`, geometry from the 1280×720
layout viewport, depth/DPR from the profile), the minimal `Intl.DateTimeFormat`
`resolvedOptions()` locale/UTC surface (`bl-ac8d`), and `crypto.getRandomValues`/
`randomUUID` from OS randomness (`bl-cf3a`). The golden oracle is
`tests/fixtures/js/persona-navigator.html` (brand/prototype/descriptor/function-
source checks, not only values). The registry (`__frot_brand`/`__frot_iface`) is
the extension point the six later capability balls (canvas/WebGL/audio/indexedDB/
Worker/permissions) plug into.

**High-entropy rendering signals: in scope where they match what the server
requires; unbuilt today, so filed as gaps.** *(Superseded 2026-07-20 by Mark's
masquerade ruling — the previous line read "Out, permanently: fabricated
high-entropy rendering … `getContext()` continues to return `null`"; it is
amended, not deleted.)* Canvas, WebGL, audio, font metrics, and media-device
enumeration are the surfaces fingerprinters read, and Mark's ruling is that
**masquerading a capability to match what the server requires is in scope**.
frot cannot *render* today, so `getContext()` still returns `null` — but that is
now a **filed gap** (`bl-bd4e` capability-gap measurement, plus its follow-ups),
not a permanent non-goal. The bar these must clear is coherence: a masqueraded
value must be a **coherent, profile-derived simulation** (a canvas hash that
varies across pages that should differ, a GPU string that fits the persona), not
noise — an *incoherent* fabricated value is a **louder** tell than absence (§10).
See §10 for the scope; the VISION principle-5 question this once raised is
**resolved** (Mark, 2026-07-20 — no conflict; §10).

---

## 9. Cookies and clocks

**Cookies (`bl-6dad`) — LANDED.** One per-invocation jar
(`src/fetch/cookie.rs`, `CookieJar`), born empty, owned by the `FetchSession` and
discarded at exit — the single authority for `Set-Cookie` (redirects, the final
document, subresources), the applicable `Cookie` header on later requests, and
`document.cookie` (previously an isolated in-memory string in `env.js`). This is
**state within a call**, which VISION principle 1 explicitly permits (*"State
within a call — redirects, JS event loop, etc. — is fine; nothing persists across
calls"*). It is not a session model: nothing is written to disk, nothing survives
the process, and there is no `--cookie-jar` flag. The coherence win is realised: a
server that sets a cookie on the document GET now sees it returned on subresource
fetches, and a JS `document.cookie` write feeds a later same-origin GET.

*As-built specifics:*

- **The jar threads explicitly, no globals.** `FetchSession` holds
  `Arc<Mutex<CookieJar>>`; `dispatch` reads/writes it per hop; the JS layer reaches
  the *same* jar through `FetchSession::cookie_jar()` → the `Host` → the
  `__frot_cookie_get`/`set` syscalls (`src/js/syscall/cookie.rs`). One authority,
  race-safe under the mutex across the concurrent gather threads.
- **The `Cookie` header** is computed by `CookieJar::header_for` and threaded into
  `request::derive_headers` as its `cookie: Option<&str>` argument, placed after
  `Referer` and before `Upgrade-Insecure-Requests` (Firefox's position). It
  honours Domain, Path, Secure (https only), Expires/Max-Age, and SameSite against
  a site context derived once per hop: a **navigation** is a top-level context
  (Lax/Strict ride; a first hop is same-site with itself); a **fetch/XHR** is
  same-origin only; **style/script/module** subresources are credentialed
  cross-origin but SameSite still gates a third-party context to `SameSite=None`.
  The registrable-domain "same-site" test reuses `request::registrable` (one
  definition), and Expires reuses `profile::CivilDate::days_since_epoch` (one
  civil→epoch algorithm).
- **Caller `-H Cookie` policy, decided once:** the jar supplies the default
  `Cookie` line; `request::apply_caller` layers the caller's `-H Cookie` *last* as
  a same-origin value override, replacing that one line in place — never a
  duplicate, never sent cross-origin, never stored in the jar.
- **`document.cookie`:** `CookieJar::document_cookie` at the final document URL
  omits HttpOnly and non-applicable cookies; `CookieJar::write_script` parses
  browser-allowed attributes but can never mint an HttpOnly cookie — so **HttpOnly
  never enters JS**, while the wire still carries it.
- **No new dependency.** A focused hand-rolled RFC 6265 parser (Path, Domain,
  Secure, HttpOnly, Max-Age, Expires, SameSite) replaced the option of a `cookie`
  crate — smaller, coverage-friendly, and it needs none of the crate's transitive
  tree. Declared residuals: the relaxed RFC 6265 §5.1.1 date tokenizer and
  2-digit-year mapping (an unparseable `Expires` degrades to a session cookie, as
  browsers do); schemeful-same-site and the `SameSite=None`-requires-`Secure`
  storage rule are not enforced.
- **Challenge cookies are not replayed:** `dispatch` may `store` a `Set-Cookie` received with
  a declared challenge, but `run.rs` still exits at the `needs:["human"]` verdict
  before parse/JS/subfetch, and the session (hence the jar) then drops. frot never
  retries, so a challenge cookie is never replayed.

**Clocks (`bl-e707`, LANDED).** Clocks derive from the profile plus the existing
virtual clock (`js.md` §5), not from a second source. The two facts that ball
settled: **timer-precision clamping** — `performance.now()`/`Date.now()` are
floored to `BrowserProfile::timer_precision_us` (1 ms, Firefox's default
`privacy.reduceTimerPrecision`; an unclamped sub-millisecond timer is itself a
tell), the one literal the observable clock reads, never a second hardcode; and
**timezone/locale**, where the capture showed host-dependent values
(`America/Los_Angeles`, offset 420) — locale is pinned to the profile and
timezone to `UTC` (the persona payload, §9), on the same determinism argument as
`hardwareConcurrency`, "a UTC browser is unusual" accepted as a declared residual
(§11) because non-deterministic output is the worse failure. The observable clock
is now one injectable monotonic source (real elapsed + a virtual timer offset)
feeding `timeOrigin`/`performance.now`/`Date.now` coherently from one origin —
the same clock the §5/§6 deadline bounds the run with (`js.md` §5).

---

## 10. Identity scope

The line is drawn on a principle, not a list:

> **Matching what the server requires is in scope, including by masquerading a
> capability frot does not physically have. A signal frot cannot yet interpret
> or produce is a *filed gap*, not a permanent non-goal.**

**In scope:**

- Deriving TLS, ALPN, HTTP version, headers, cookies, `navigator`, and clocks
  from one profile, mutually consistent.
- Genuinely negotiating everything advertised (I2).
- Low-entropy coherent facts and correct Gecko branding (§8).
- **Masquerading a capability to match what the server requires** — including the
  high-entropy rendering surfaces (canvas/WebGL/audio/font metrics/media-device).
  The bar is **coherence, not abstinence**: a masqueraded value must be a
  coherent, profile-derived *simulation*, because an
  *incoherent* fabricated value (a GPU string that contradicts the persona, a
  canvas hash that never varies across pages that should differ) is a **louder**
  tell than absence. "Masquerade" therefore means a deterministic, profile-derived
  simulation, never noise. *(This coherence bar is an engineering/detectability
  requirement, not an honesty one — VISION principle 5 is separately resolved as
  not conflicting, see the box below.)*

**Filed gaps — in scope, not yet built** (cross-referenced to the tasks filed
alongside this one):

- Every rendering/interpretation signal frot cannot yet produce or read is a
  **backlog item**, per Mark's ruling: canvas/WebGL/audio/font-metric/media-device
  simulation is owned by **`bl-bd4e`** (capability-gap measurement) and its filed
  follow-ups. `getContext()` returns `null` **today** because the simulation is
  unbuilt.

> ### ✔ RESOLVED — VISION principle 5 does not conflict (Mark, 2026-07-20)
>
> VISION principle 5 is *"Honest capability signals … Never silently produce a
> degraded result that looks complete."* The question raised here — is a
> fabricated canvas hash a dishonest *impression*? — was **put to Mark and
> answered: no conflict.** Verbatim: *"principle 5: no, it doesn't [conflict]. be
> honest to the user about what happened. How that got done is fine. Private
> browser windows do the same thing all the time; so long as you're safe on the
> wire, it's up to the client (frot in this case) what you do with what you're
> delivered."*
>
> **The resolution — two audiences, two contracts, one process:**
> - Principle 5 governs the **delivered impression** to the caller/user — honesty
>   about *what happened* (the `--out *` payload, `needs`, the `http` block). It
>   does **not** constrain the **wire persona** frot presents to obtain that
>   impression.
> - A masqueraded wire identity is a legitimate client-side choice, analogous to a
>   private browsing window, provided frot is *safe on the wire*. It does not make
>   the delivered result dishonest.
> - Therefore **no exception clause is added to principle 5** — VISION.md carries
>   only a short dated clarifying note (2026-07-20) saying principle 5 is about the
>   delivered impression, not the wire persona.
> - **The coherence constraint stands — but as engineering, not honesty.** A
>   masqueraded value must be a **deterministic, profile-derived function** (never
>   random/incoherent), because an incoherent fabricated value is a *louder* tell
>   than absence. That is a detectability requirement, independent of principle 5.

**The honest-outcome half is unchanged.** If a page defeats the shim, the outcome
is an honest `needs:["js"]`, `needs:["human"]`, or `error.kind: http.<code>` —
never a degraded result that looks complete.

---

## 11. Residuals — stated, not papered over

Permanently out of reach, by design or by constraint:

| residual | status |
|---|---|
| **IP / ASN reputation** | Out of scope entirely. Likely dominates real outcomes (§14). |
| **TCP/IP OS fingerprint** (TTL, window size, options) | Host kernel's, not frot's. Coherent by accident on Linux; **incoherent on any non-Linux host**, since the profile claims `X11; Linux x86_64`. Unfixable without raw sockets. |
| **Required POST telemetry** | Not supported — the navigation is GET-only (the frottage rule). Sites that gate on a beacon POST cannot be served. `sendBeacon` returns `false` (a legal denial). |
| **Proof-of-work challenges** | Not performed — frot computes no token of its own; `docs/design/challenge.md` is the proposed path (running the page's own challenge script). |
| **Behavioural challenges** (mouse paths, dwell time, scroll) | Not met — frot dispatches only the `DOMContentLoaded`/`load` lifecycle pair and synthesizes no input. |
| **High-entropy rendering** (canvas/WebGL/audio/font metrics) | **In scope since 2026-07-20.** In scope as a coherent masquerade. **Canvas 2D (`bl-05e6`), WebGL (`bl-f624`), and Web Audio (`bl-8733`) are now BUILT (LANDED)** — see their own rows below. Only font metrics remain an **unbuilt filed gap** — `bl-bd4e` and follow-ups (§10). Not a permanent residual. |
| **Font metrics via `offsetWidth` measurement loops** | The `document.fonts` / FontFaceSet API is a filed gap (a follow-up of `bl-bd4e`), but the **`offsetWidth`-based glyph-width channel is a residual**: frot's layout is a structural approximation (`layout.md` §6), so per-glyph text widths cannot be reproduced faithfully, and a *wrong* width is a louder tell than a missing font (§10's coherence bar). It is also unobservable to the `bl-bd4e` instrument (indistinguishable from ordinary layout reads), so no page can be cited as probing it — it is out by the no-folklore rule, not measured in. |
| **`crypto.getRandomValues` / `randomUUID`** | **Provided (`bl-3972`/`bl-cf3a`, LANDED)** — real OS randomness, not a fake, with the browser argument/quota/error contract. |
| **Worker / SharedWorker message delivery** | **Constructors provided (`bl-342a`, LANDED)** — `typeof Worker === 'function'`, `new Worker(url)` returns a Firefox-shaped, branded-native instance (`postMessage`/`terminate`/`onmessage`, `[object Worker]`); `SharedWorker` likewise, with a `MessagePort` `port`. Presence is the coherence requirement (§10). **The residual is delivery:** frot spawns no thread (js.md §11), so a constructed worker never delivers a message — `postMessage` is an honest no-op, `onmessage` never fires. Deterministic, fixed, no syscall. Actual worker-code *execution* is out of scope (js.md §11). |
| **IndexedDB open completion / persistence** | **Factory + zoo provided (`bl-8dde`, LANDED)** — `'indexedDB' in window`, `indexedDB instanceof IDBFactory`, `indexedDB.open` branded native; the `IDB*` interface zoo is Firefox-shaped (`[object …]` tags, "Illegal constructor" throws), and `IDBVersionChangeEvent` is a constructable `Event` subtype. Presence is the coherence requirement (§10). **The residual is completion:** frot has no backing store (stateless, §7 / VISION §1), so `open()`/`deleteDatabase()` return a real, permanently-`pending` `IDBOpenDBRequest` whose `onsuccess`/`onupgradeneeded`/`onerror` never fire — synchronously byte-identical to a real request (result/error throw `InvalidStateError` while pending), only never completing. This silence is **less** detectable than firing an error, because a real fresh `open()` succeeds, so an error callback would contradict the persona. `databases()` honestly resolves to `[]`; `cmp()` is a real key comparison. Deterministic, fixed, no syscall. |
| **Observer deliveries beyond the seam** | **MutationObserver GENUINE, IO/RO initial-delivery genuine (`bl-07ab`, LANDED; the full argument is js.md §7)** — `MutationObserver` is a real implementation over the five mutation syscalls (childList/attributes/characterData records with oldValue, subtree by parent-chain walk, microtask delivery): not a masquerade, no residual beyond record *granularity* (`innerHTML`/`textContent` replacement emits per-node records where a browser coalesces one "replace all" record — the same mutations, finer sliced). `IntersectionObserver`/`ResizeObserver` deliver a genuine INITIAL batch computed from the real §8 geometry at scroll 0 — presence-but-never-firing was REJECTED for these two, because a real browser always delivers an initial batch, so the costume that defends Worker/indexedDB (silence a real fresh browser also shows) fails here, and a dead-but-present IO makes lazy-load libraries wait forever where absence made them load eagerly. **The residual is post-initial deliveries:** later DOM-mutation-driven geometry changes produce no further IO/RO entries. A real browser's own post-initial deliveries are driven by scroll/resize/animation, none of which frot ever produces (js.md §11), so the gap is exactly the mutation-driven slice; closing it would re-diff layout per observed target per generation — real §5 CPU cost for a trigger frot structurally never fires. Deterministic, no new syscall. |
| **Permissions grant / Notification prompt** | **Surface provided (`bl-1548`, LANDED)** — `navigator.permissions` is a branded `Permissions` instance whose `query({name})` returns a `Promise<PermissionStatus>` (branded, `[object PermissionStatus]`, `state`/`name`/`onchange`, EventTarget); an unrecognised name rejects with the Firefox-coherent `TypeError` (the Firefox 140esr `PermissionName` enum: geolocation, notifications, push, persistent-storage, midi, storage-access, screen-wake-lock, camera, microphone). `window.Notification` is a branded, constructable interface with static `permission`/`maxActions`/`requestPermission`. Presence is the coherence requirement (§10). **The residual is the grant:** frot raises no prompt and shows no notification, so — on a FRESH profile, deterministic, never random — every `query()` resolves state `'prompt'` (nothing granted or denied), `Notification.permission` is `'default'`, and `requestPermission()` resolves an honest `'default'` (no grant); a constructed `Notification` fires no event, `onchange`/`onclick` never fire. This is exactly what a real, un-prompted page sees, so it is *coherent, not a wrong value* — asking for a grant frot cannot make would be the louder tell. Deterministic, fixed, no syscall. |
| **Canvas 2D pixel realism** | **Context + hash provided (`bl-05e6`, LANDED)** — `getContext('2d')` returns a branded, Firefox-shaped `CanvasRenderingContext2D`; the drawing API (`fillRect`/`fillText`/`arc`/`measureText`/…) is present and branded native; `toDataURL()` returns a well-formed, decodable `image/png` data URL and `getImageData()` a right-sized `ImageData`. Both are a DETERMINISTIC function of (the fixed profile `canvas_seed` + the exact draw sequence + dimensions): the SAME draws hash identically on every invocation (asserted across two runs in `persona_gold`), DIFFERENT draws diverge, and the hash is never random per call — which is precisely the tell a randomising privacy tool shows. Presence + determinism are the coherence requirement (§10). **The residual is pixel realism:** the bitmap is a digest expansion (an xorshift PRNG seeded by the draw digest), not a glyph raster, so it is stable and content-varying (the fingerprint properties that matter) but would not survive a pixel-level comparison against a reference Firefox render — and no such reference exists for an arbitrary draw sequence, while determinism defeats the louder randomised-canvas tell. `toDataURL` encodes only PNG (Firefox's default); a `jpeg`/`webp` request returns a coherent PNG (a minor residual). WebGL is a separate ball (`bl-f624`) — `getContext('webgl')` stays null. Deterministic, profile-seeded, **no syscall** (determinism forbids host entropy). |
| **WebGL rendering realism** | **Context + fingerprint provided (`bl-f624`, LANDED)** — `getContext('webgl')`/`'webgl2'`/`'experimental-webgl'` return branded, Firefox-shaped `WebGLRenderingContext`/`WebGL2RenderingContext` (both published as interfaces; `[object …]` tags; "Illegal constructor" throws). `getParameter(VENDOR)` and `getParameter(RENDERER)` are Firefox's masked `"Mozilla"` (never the real GPU); the real strings surface ONLY through the `WEBGL_debug_renderer_info` extension and are a **coherent, deterministic, Linux-plausible SOFTWARE renderer**: `UNMASKED_VENDOR_WEBGL` = `"Mesa"`, `UNMASKED_RENDERER_WEBGL` = `"llvmpipe (LLVM 19.1.7, 256 bits)"`. llvmpipe is chosen deliberately (Mark's ruling, 2026-07-20): it is common for real headless Linux Firefox and NEVER over-claims specific hardware — an NVIDIA/Intel string on this persona, or a value that varied per invocation, would be a LOUDER tell than absence. `VERSION`/`SHADING_LANGUAGE_VERSION` are Firefox's clean `"WebGL 1.0"`/`"WebGL 2.0"` + `"WebGL GLSL ES 1.0"`/`"3.00"`; the `MAX_*` limits, `getSupportedExtensions()`, and `getShaderPrecisionFormat` (all float qualifiers highp `{127,127,23}` — the desktop-GL signature) are ONE real Mesa 24.2/llvmpipe build's set, internally coherent (anisotropy present ⇔ modern Mesa; ASTC absent ⇔ software, not mobile). Every value is a deterministic function of the pinned profile — the SSOT const `src/fetch/webgl.rs`, delivered via the one `__frot_env_profile` channel `canvas_seed` uses, so `webgl.js` holds no identity literal (I1). `readPixels()`/`toDataURL()` are a deterministic, `canvas_seed`-derived digest expansion (shared with canvas via `webglpix.js`): the SAME draw sequence hashes identically on every invocation (asserted across two runs in `persona_gold`), a DIFFERENT one diverges, never random per call. Unknown enums return `null`, unsupported extensions `null` (Firefox's answers); a canvas binds ONE context type for life, so a cross-type `getContext` is `null`. **The residual is pixel realism:** frot runs no GL, so the pixels are a digest expansion, not a real llvmpipe raster — stable and content-varying (the fingerprint properties that matter) but not byte-equal to a reference render, for which no oracle exists for an arbitrary scene; determinism defeats the louder randomised tell. Deterministic, profile-seeded, **no syscall** (determinism forbids host entropy). |
| **Web Audio rendering realism** | **Context + fingerprint provided (`bl-8733`, LANDED)** — `AudioContext`/`OfflineAudioContext` are branded, constructable, Firefox-shaped (`[object …]` tags; `BaseAudioContext` is abstract — "Illegal constructor"); no `webkitAudioContext` alias (Firefox has none — an alias would be the tell). The node zoo (`OscillatorNode`/`DynamicsCompressorNode`/`GainNode`/`AnalyserNode`/`BiquadFilterNode`/`AudioBufferSourceNode`/`AudioDestinationNode`), `AudioParam`, and `AudioBuffer` are present and branded native; `create*` factories and `connect`/`disconnect` build a real graph. `sampleRate` is Firefox's `44100`, `destination.maxChannelCount` `2`, `baseLatency`/`outputLatency` DERIVED from the sample rate (`128`/`512` frames ÷ `44100`) so the one rate fact is their single source. The fingerprint — `OfflineAudioContext.startRendering()`'s rendered `AudioBuffer` float samples — is a DETERMINISTIC function of (the fixed profile `audio_seed` + the exact graph digest: every node create, param write, `connect`/`disconnect`, `start`/`stop` folded in order via FNV, exactly the canvas/WebGL digest model): the SAME graph hashes identically on every invocation (asserted across two runs in `persona_gold`), a DIFFERENT graph (e.g. a different oscillator frequency) diverges, never random per call — precisely the tell a randomising privacy tool shows. Samples sit in the plausible `[-1, 1]` range; a constructed (un-rendered) `AudioBuffer` is silent zeros, like a real one. The render digest is frozen at `startRendering()` so it is stable even if the graph is mutated afterward. Every value derives from the pinned profile — the SSOT const `src/fetch/audio.rs`, delivered via the one `__frot_env_profile` channel `canvas_seed`/`webgl` use, so `audio.js` holds no identity literal (I1). **The residual is waveform realism:** frot runs no audio DSP, so the samples are a digest expansion, not a real Gecko oscillator→compressor render — stable and graph-varying (the fingerprint properties that matter) but not sample-equal to a reference Firefox render, for which no oracle exists for an arbitrary graph; determinism defeats the louder randomised tell. Generic `AudioParam` min/max, realtime `currentTime` `0`, and no-op `AnalyserNode` data methods are minor accepted residuals. Deterministic, profile-seeded, **no syscall** (determinism forbids host entropy). |
| **`crypto.subtle` (WebCrypto)** | Residual. The full `SubtleCrypto` surface is a large capability left absent (`crypto` has no `.subtle`); a filed gap decides if/when it lands. Its absence is a mild coherence tell, accepted. |
| **`Intl` beyond `DateTimeFormat.resolvedOptions()`** | Residual. quickjs-ng ships without `Intl`; frot provides the low-entropy locale/timezone subset a page reads (`DateTimeFormat` + `resolvedOptions`, `timeZone` pinned UTC). `NumberFormat`/`Collator`/`RelativeTimeFormat`/real locale-aware formatting are unbuilt — a filed gap, not a permanent non-goal. |
| **`__frot_*` syscall names enumerable on `globalThis`** | Residual. `Function.prototype.toString` no longer leaks their source or the name (`bl-3926` branding), but the syscall **names** are still enumerable globals (`Object.getOwnPropertyNames(window)`). Making them non-enumerable touches every `bind!` site and is tracked as a separate concern. |
| **Three key shares** | Blocked by rustls (§6.4). |
| **ClientHello extension order/set** | Stock rustls emits its own order and omits Firefox's `compress_certificate`/SCT/`record_size_limit`/ECH shaping. Declared residual under Option C (§6.1); would need a rustls fork, which §6.3 forbids on security grounds. **Asserted since `bl-7523` (2026-08-12)**, in the two halves that can be asserted honestly: the *set* is pinned (28 and 27 absent, the three load-bearing extensions present, no GREASE on either side) and reaches the JA4 extension hash; the *order* is deliberately left free, because pinning rustls internals would break the `cargo update` Option C exists to keep cheap — see §12's correction of the same date, which fixes a doc claim that the order **was** pinned. |
| **Cipher *list* breadth (9 vs 17) → JA4 cipher component** | rustls advertises only its AEAD suites, not Firefox's legacy CBC/RSA. The cipher *order* matches; the *list* (and thus the JA4 cipher hash) does not. Declared residual (§6.1). |
| **secp521r1 + FFDHE2048/3072 groups** | Not offered by aws-lc-rs; the other four persona groups match in order (§6.1). |
| **`m,p,a,s` pseudo-order + HEADERS PRIORITY + first stream id** | Deferred to stage C (§7); the `h2` crate hardcodes `m,s,a,p`, exposes no client PRIORITY, and opens client streams at 1 where the persona's first request rides stream 3. **Asserted since `bl-7523` (2026-08-12)** in `src/fetch/transport/h2_request.rs`, off a real HEADERS frame: order `m,s,a,p` decoded from the HPACK block, `PRIORITY` flag clear (persona weight 42 named), stream 1 (persona `h2_initial_stream_id` 3 named). Before that ball, none of the three was asserted anywhere — §12 claimed all three, and the only mention of `pseudo_order`/`priority_weight` outside the const compared the const to its own literal. |
| **h2 SETTINGS id 1 `HEADER_TABLE_SIZE` absent; id 6 `MAX_HEADER_LIST_SIZE` present** | Blocked by `hyper-util` (`bl-f312`, 2026-07-22). frot sends `2:0; 4:131072; 5:16384; 6:16384`; Firefox 140esr sends `1:65536; 2:0; 4:131072; 5:16384`. ids 4 and 5 are enforced from the profile and the connection WINDOW_UPDATE increment now matches Firefox exactly (12517377). The two that do not: `hyper`'s conn builder has `header_table_size`, but `hyper-util`'s **pooled** `Client` builder — the one giving frot h2 connection reuse (§3.5) — exposes no passthrough and keeps its `h2_builder` private, so id 1 cannot be sent; and hyper types `max_header_list_size` as `u32`, not `Option<u32>`, so id 6 cannot be omitted. Both are asserted, with the persona's 65536 kept as the reference, in `src/fetch/transport/h2_preface.rs`. **Cost corrected 2026-07-24 (`bl-fa12`, §6.5)** — the earlier wording "closing either means dropping the pool or forking hyper" conflated two different prices, verified against `hyper-1.9.0`/`h2-0.4.15`: **id 1** closes by dropping `hyper_util::Client` for a frot-owned pool over `hyper::client::conn` (whose `http2::Builder` *does* have `header_table_size`); **id 6** closes for nobody short of forking `h2` — hyper applies `max_header_list_size` unconditionally and `h2::client::Builder` can only set `Some`. §6.5 declines the pool: it would leave three of the four akamai components still mismatched, so the hash still differs. |
| **h1 pool check-in is eventual, so a same-origin request can re-dial** | Declared 2026-07-24 (`bl-fa12`, §6.5; measured `bl-df88`). `hyper_util`'s `Client` checks an **HTTP/1.1** connection back in from a task it spawns, not inline, so a next same-origin request arriving before that task is scheduled dials a second socket: 16 of 200 sequential pairs on a saturated 16-core box, 0 of 200 with a 5 ms gap. **h2 is unaffected** (checked in inline via `drop(pooled)`), which is what the fingerprinting origins speak. Accepted rather than fixed because a second h1 socket is *inside* the persona's own behaviour — Firefox opens up to six per host, which is what `POOL_PER_HOST = 6` copies — and because the re-dial rides TLS session resumption (rustls's default `Resumption::in_memory_sessions(256)`, one `ClientConfig` per invocation), so it is an abbreviated handshake exactly as a browser's second connection is. No `hyper-util` knob makes check-in synchronous; owning the pool would, at the cost of rebuilding ALPN-h2 connect dedup, checkout liveness, the checkout/dial race, and canceled-request retry (§6.5). Invariant tests therefore assert reuse as **eventual**, never as a scheduler outcome. |
| **UTC timezone** | Deliberate — determinism over realism (§9). |
| **frot is identifiable *as frot*** | See §14 — this is accepted, not solved. |

**`bl-bd4e` measurement (dated evidence, `docs/design/probe-evidence.md`, 2026-07-21).**
The rows above that read *"filed gap"* are now backed by a live measurement of
**which capability surfaces the field corpus actually probes**, not a spec list.
The instrument records every watched surface a page touches while returning frot's
real value unchanged, and its follow-up balls are filed **only** for surfaces a
real page was measured probing — no speculative entries. Two facts from that pass
belong here as deliberate non-answers: (1) the `offsetWidth` font channel above is
**structurally unmeasurable** by the instrument; (2) the measurement **under-observes**
surfaces reached only inside external fingerprint bundles that fail `subfetch` or
throw early (frot records a probe only in scripts that execute), so absence of a
surface from the table is *not* evidence a browser would not probe it — only its
presence is load-bearing.

---

## 12. Test oracle — normalized golden captures

One mechanism for all four layers (`bl-d66b`). A local TLS/h2 server in-repo
records the handshake and frames; the existing throwaway CA at
`src/fetch/firefox_tls/testdata/{ca.der,leaf.der,leaf.key.der}` already makes
this work with **no new dev-dependency** (proven — `frot-https-head.md` captured
frot's real HTTPS head this way through the unmodified production path).

**Normalized — excluded from comparison** (per-connection random or
environment-dependent; *not* identity):

- `client_random`, `session_id`
- `key_share` **payload bytes** (the *groups and their order* are not normalized)
- ECH payload bytes
- SNI hostname, `Host` value, ephemeral ports
- `padding(21)` presence and length — ClientHello-length-dependent, and it varies
  across real browsers too (§6.4)
- timestamps, `Date`/`Cookie` expiry values

**Reconciliation with Option C (2026-07-21).** The "must match exactly" list
below was written against a *crafted* ClientHello. Under Option C (§6.1) frot
ships stock rustls, so the fields it cannot shape are **asserted as declared
residuals** — the oracle pins frot's *actual* emission (rustls's 9-suite cipher
list, ≤2 key shares, `m,s,a,p`) with a pointer to §6.1/§11, exactly as §1
requires ("coherent… explicitly not byte-identical"). A field is either an exact
Firefox match *or* a residual asserted against frot's own stable output; neither
may drift silently. This is the honest bridge between this section's "match
exactly" and §1's "not byte-identical": the residuals are the difference, and
they are tested, not hidden.

> **Correction (2026-08-12, `bl-7523`): the extension *order* is not pinned, and
> should not be.** This paragraph used to list "rustls's extension order" among
> the things the oracle pins against frot's own emission. It never did, and
> `src/fetch/transport/recorder.rs` says why in as many words: pinning the order
> would couple the test to rustls **internals**, so an ordinary `cargo update`
> would fail the build — and keeping `cargo update` cheap is the entire reason
> Option C exists (§6.1/§6.3). The doc was wrong, not the test.
>
> What the oracle pins about the extension list instead is order-free and still
> identity-bearing, so nothing is given up but the coupling: the three
> fingerprint-load-bearing extensions (`supported_groups`, ALPN, `key_share`)
> are **present**; the two Firefox-only ones are **absent, asserted as declared
> residuals** with the persona's own values named in the failure message; and
> **GREASE is absent on both sides** — a real match, since 140esr sends none and
> rustls sends none, asserted by RFC 8701's `0x?a?a` *class* rather than by
> listing values. The extension **set** still reaches the JA4 hash below, so a
> set change is caught there even though the order is free to move.

**Not normalized — an exact Firefox match where reachable, else a pinned
residual.** Ordered capabilities are identity:

- cipher list **and wire order** (17, `0xc009` at index 10)
- extension list **as a set** (17, ECH last in the persona). The *order* is
  deliberately unpinned — see the 2026-08-12 correction above.
- `supported_groups` **and order** (`4588,29,23,24,25,256,257`)
- `key_share` **groups and order** (4588, 29, 23)
- `signature_algorithms` **and order** (11)
- ALPN list and order; negotiated protocol
- `record_size_limit` = 16385; `compress_certificate` = zlib, brotli, zstd.
  **Built and dated (`bl-7523`, 2026-08-12):** both are now §4.1 fields of the
  profile const (they were declared in the table but stored nowhere), and
  `recorder.rs` asserts extension 28 and extension 27 are **absent from frot's
  wire** — declared residuals, since rustls has no API for either — quoting the
  persona's 16385 and zlib/brotli/zstd in the failure message so the gap stays
  legible. A residual's assertion is that it still differs in the declared way.
- absence of GREASE. **Built and dated (`bl-7523`, 2026-08-12):** asserted on
  frot's wire *and* on the persona's declared lists, by RFC 8701's `0x?a?a`
  class (`ja4::is_grease`, one definition site), so no GREASE value is written
  down anywhere. This one is a genuine **match**, not a residual.
- h2 SETTINGS: **the entry set, values and order**; WINDOW_UPDATE 12517377; first
  HEADERS on **stream 3**; `PRIORITY` flag with weight 42 / depends_on 0 /
  exclusive 0; pseudo-order. **Built and dated (`bl-f312`, 2026-07-22):
  `src/fetch/transport/h2_preface.rs`** captures frot's real preface off an
  ALPN-`h2` throwaway-CA origin and pins `2:0; 4:131072; 5:16384; 6:16384` plus
  the stream-0 WINDOW_UPDATE increment 12517377. The expectations are *derived
  from* `FIREFOX_140_ESR.h2`, so the profile stays the single source: ids 4/5 and
  the increment are enforced from it, id 2 is asserted against it (hyper hardcodes
  the same value, so the match is hyper's — pinned anyway), and the two §11
  residuals are asserted **as residuals** — id 1's absence, id 6's presence at
  hyper's default. A fifth entry, a missing one, or a changed value fails the
  build; no h2 SETTINGS fact is outside the declared set.

  **The request half was built later (`bl-7523`, 2026-08-12):
  `src/fetch/transport/h2_request.rs`.** The preface oracle stopped at SETTINGS
  and WINDOW_UPDATE and never captured a HEADERS frame, so the stream id, the
  `PRIORITY` flag and the pseudo-order in the line above were asserted **nowhere
  in the tree** — the only mention of `pseudo_order`/`priority_weight` outside
  the const was a struct-equality check of the const against its own literal,
  which is a drift guard on the *declaration*, not an observation of the wire.
  It now captures the first HEADERS off an origin that replies with its own
  SETTINGS (which the client waits for before opening a stream), and asserts all
  three **as declared residuals**, each with a pointer to §7 stage C: the frame
  rides **stream 1**, not the persona's 3 (`h2` opens client streams at 1, and
  `h2_initial_stream_id` is now a profile field so the persona value has a
  home); the `PRIORITY` flag is **clear**, with the persona's weight 42 named in
  the message; and the pseudo-order decoded out of the real HPACK block is
  `m,s,a,p`, not `m,p,a,s`. Frames are selected by kind, not index, because a
  replying origin puts the client's SETTINGS ACK in the stream.
- request header **names, order, and values**, per destination
- h1 header **casing**, asserted on **both** schemes (I3 — this is the regression
  test for §3.4)
- the §8 `navigator` fact set
- the envelope `http.headers` allowlist (`bl-acec`): `retry-after`,
  `cf-mitigated`, `x-amzn-waf-action`, `server`, `x-datadome`, `content-type` —
  lower-cased names, repeats preserved in wire order. Deterministic by
  construction (`set-cookie` and volatile per-request headers are excluded), so
  it is a golden field, not a normalized one; a surfaced header outside the
  allowlist is a drift. **Order across distinct names is not a contract**
  (measured, `bl-160d`): the capture is hyper's `HeaderMap::iter()`, whose
  cross-name order is documented as arbitrary, so the offline pin
  (`run::http_tests::surfaces_exactly_the_allowlist_and_nothing_else`) asserts
  the allowlist's *contents* as a multiset and pins sequence only for a
  repeated name.

**Hashes are assertions, not fixtures.** Fingerprint values are **computed from
the capture and compared to the value the same function computes from the
profile** — never stored as hand-written strings (I6). This is what makes a pin
change a one-table edit (§4.2): there is no fingerprint string anywhere in the
tree to regenerate.

> **Built and corrected (2026-08-12, `bl-7523`).** This paragraph named seven
> hash families and *nothing computed any of them* — `grep -ri "ja3\|ja4\|
> peetprint\|akamai" src/` returned doc comments only. It was the load-bearing
> claim under §4.2, so it is now built, but built for the families that can be
> honestly computed, and the list is corrected to say which:
>
> | family | status |
> |---|---|
> | **JA4, JA4_r, JA4_ro** | **Computed** — `src/fetch/transport/ja4.rs` is the FoxIO function; `recorder.rs` runs it over the real ClientHello *and* over §4.1's lists. SHA-256 comes from the TLS 1.3 suite rustls already links, so no dependency was added. |
> | **akamai-h2** | **Computed** — `h2_wire::akamai` builds `SETTINGS\|WINDOW_UPDATE\|PRIORITY\|PSEUDO` from the capture and from `FIREFOX_140_ESR.h2`; `h2_request.rs` compares them. |
> | **JA3, JA3N** | **Not computed, and the claim is withdrawn.** They are MD5, which nothing in the tree provides, so they would cost a new dependency (an AGENTS.md escalation) — for a deprecated fingerprint whose *only* variable input under Option C is the cipher list, which is already a declared residual and already reaches JA4's cipher component. Zero information for a dependency. |
> | **peetprint** | **Not computed, and the claim is withdrawn.** It is one service's format, defined only by `tls.peet.ws` and unversioned; pinning it in-repo pins a third party's undocumented output. §12 already puts peet in the opt-in live-check bucket below, which is where it belongs. |
>
> **What the JA4 comparison asserts, precisely.** §6.1 states plainly that "JA4
> does not match the pin", so a test demanding equality would be a test demanding
> a known failure. The oracle instead asserts the *shape* of the difference:
> equal on what Option C reproduces (TCP, TLS 1.3, SNI present, `h2` first in
> ALPN), different on both hashed halves and on both counts — the cipher-list and
> extension-set residuals. Both sides are computed, so a re-pin moves both and a
> silent drift in either moves only one.

**Connection-level facts pinned here too (`bl-fa12`, 2026-07-24).** They are not
handshake bytes, so they live in `src/fetch/session/tests.rs` rather than the
capture, but they are oracle facts by the same rule: one pool per session and
none across sessions (`separate_invocations_cannot_share_a_connection`), and
same-origin reuse asserted as **eventual** — a request that adds no connection —
because h1 check-in is a scheduler outcome (§6.5, §11). Asserting "the second
request rides the first's socket" is asserting the scheduler, and it flaked
(`bl-df88`). The resumption claim §6.5 leans on — a re-dial inside one
invocation offers the ticket and takes the abbreviated handshake — is pinned
separately (`bl-17e7`, `src/fetch/transport/resumption.rs`): one `Transport`
dials the throwaway-CA origin twice and the origin's own `handshake_kind()` must
read `Full` then `Resumed`, so a dependency bump that changed rustls's
`Resumption` default could not flip it silently. That pin does *not* assert the
scheduler either — the second dial is forced by an ALPN-`http/1.1` origin
answering `Connection: close`, which hyper may not pool.

**Declared residuals are asserted too.** Stage B's `m,s,a,p` is written into the
oracle *as the expected value with a pointer to §7 stage C*, so it cannot drift
silently and cannot be mistaken for a match. A residual that isn't in the oracle
is a residual that will be forgotten. **True since `bl-7523` (2026-08-12), not
before** — `m,s,a,p` appeared in no test at all until `h2_request.rs` decoded it
off the wire.

**A literal in the oracle is not a second copy of a persona fact.** I1 forbids a
persona value having two definition sites; it does not forbid the oracle writing
down *what frot's own dependencies emit*. `recorder.rs`'s nine-suite cipher list
and `h2_request.rs`'s `m,s,a,p` are observations of rustls and `h2`, and pinning
them is the whole point — a `cargo update` that changes either must fail the
build. Every value that comes from the **persona** is read from the const.

**Live checks are opt-in.** A network-gated test against `tls.peet.ws` /
`tls.browserleaks.com` (the two services that cross-validated the 140esr capture)
stays out of the default run — CI has no network, and determinism is the point.

---

## 13. Migration order → the eight siblings

```
bl-5191 ──► bl-20ec ──► bl-abca ──┬──► bl-3972 ──┐
(session)   (profile)   (TLS+h2)  ├──► bl-6dad ──┼──► bl-08f6 ──► bl-d66b
                          ▲       └──► bl-e707 ──┘   (scheduling)  (capstone)
              checkpoint DISCHARGED (§6.1, 2026-07-20); decoder measurement owed
```

| # | task | delivers | why here |
|---|---|---|---|
| 1 | **`bl-5191`** per-invocation fetch session | one `Agent` per invocation, not per URL | Prerequisite for everything. Root cause of §3.5 (`fetch_within()` builds a fresh agent per URL). **Needs no new dependency — not blocked by the checkpoint.** |
| 2 | **`bl-20ec`** request metadata | the `BrowserProfile` constant + the *one* ordered request description, handed to both serializers | The single-source-of-truth core. Dissolves §3.4 by construction and fixes §3.3 rows 1–3, 5, 7, 8. **Also not blocked by the checkpoint** — it is a refactor plus value fixes. |
| 3 | **`bl-abca`** TLS + h2 | stage A then stage B (§7) | **Checkpoint discharged** (§6.1, 2026-07-20 — async settled, C-stack measured). Still owes I2's content-encoding decoders, or `Accept-Encoding` stays a declared residual at `gzip, br`. |
| 4 | **`bl-3972`** JS persona | `env.js` facts via one syscall from the profile (§8) | After 2 (the profile must exist). Independent of 3. |
| 5 | **`bl-6dad`** cookie jar | one per-invocation jar shared by transport and `document.cookie` (§9) | After 1 (needs the session) and 2. |
| 6 | **`bl-e707`** clocks | precision clamping, locale/TZ (§9) | After 2. Independent of 3–5. |
| 7 | **`bl-08f6`** resource scheduling | concurrency over multiplexed h2 | **Hard dependency on stage B.** Cannot start before 3. |
| 8 | **`bl-d66b`** capstone | golden captures (§12) + corpus re-run | Last. Its corpus re-run is the **only** thing that can answer §14's open question. |

Steps 1, 2, 4 and 6 deliver real coherence wins **without touching the dependency
tree**, so the checkpoint blocks less than half the epic. If the checkpoint is
declined, they still land and §1's ceiling statement simply becomes the shipped
position.

---

## 14. What this design does not solve

Attacking it before committing it, per `~/AGENTS.md`.

1. **It changes no measured access outcome — the falsifier ran and this is the
   result (`bl-d66b`, 2026-07-21, §3.8).** The corpus re-run after every sibling
   landed is **outcome-identical** to the pre-transport baseline: no gate that
   failed now succeeds, none that succeeded now fails. StackOverflow still does
   not reproduce its gate from this IP; Amazon 202 is still a withheld body, not
   fingerprint-caused. **This null belongs here, not in a drawer** — recorded as
   promised. The justification standing on measured ground is *coherence* (the
   §3.8 tells are gone) and *security maintenance* (§6.3), **not** access
   improvement; that was always the honest claim (§1) and the measurement holds
   it to it. The A/B instrument is `examples/ab_harness.rs` (`bl-46f5`) and the
   capability-gap measurement is `docs/design/probe-evidence.md` (`bl-bd4e`);
   both report the null as measured.

   > **Amended 2026-07-22 (`bl-017a`) — this verdict is no longer universal in
   > scope, and must be re-measured, not left standing.** The null above is
   > correct **for the Phase 5 identity work** and stays as written: nothing in
   > the transport/persona epic changed an access outcome. But `bl-017a` proposes the
   > challenge round trip (`docs/design/challenge.md`), and its whole premise is that a
   > *different* class of change — executing a declared challenge and completing
   > the round trip it poses — may change an outcome that no persona improvement
   > could. **The scope of the null is therefore narrowed to the identity epic**,
   > and the challenge design owes its own measurement (`challenge.md` §8), on the
   > same egress IP and the same corpus, with the same standing obligation to
   > record a null as loudly as a win. Until that measurement runs, the honest
   > statement is *"no measured access improvement from identity work; the
   > challenge path is unmeasured"* — not *"no measured access improvement,
   > period."* If the challenge path also nulls, item 2 below is the likely cause
   > and the result belongs in a new §3.10.

2. **IP/ASN reputation probably dominates.** Everything here is client identity.
   If a target scores the egress IP, a perfect profile changes nothing.
3. **Determinism makes frot a cohort.** Pinning `hardwareConcurrency`, timezone,
   and screen means *every frot install looks identical*. That is itself a
   fingerprint — just a *consistent* one rather than a *self-contradictory* one.
4. **Most residuals in §11 are permanent under the current constraints**, and
   three of them (key shares, pseudo-order, and the h2 SETTINGS set — `bl-f312`
   added that row on 2026-07-22, and its absence from this list was itself the
   §12 drift that ball closed) are inside the fingerprint a serious defence
   actually hashes. *(One row is no longer permanent: high-entropy
   rendering became an in-scope, unbuilt gap on 2026-07-20 — §10, `bl-bd4e`.)*
5. **Profile staleness is silent without I8.** CI has no network, so nothing
   notices Firefox moving. I8's build-time expiry is the mitigation and it is
   coarse — it catches EOL, not mid-line drift.
6. **`Accept-Encoding` may not reach parity.** The transport checkpoint is now
   discharged (§6.1: async settled by Mark, C-stack measured), but the
   zstd/deflate **content-encoding decoders I2 requires are still owed and
   unmeasured**. Until `bl-abca` lands them, I2 forces `gzip, br` to stay — an
   honest but non-matching value. I2 is not negotiable; the *value* is. This is
   also why config (b)'s −32 KiB is a floor, not the settled size (§6.1 item 3).

---

## 15. Adjacent defects surfaced here, owned elsewhere

Recorded so they are not lost, and **not** silently absorbed into a sibling.
Each needs its own ball.

1. **The envelope discards response headers. — DELIVERED (`bl-acec`,
   2026-07-20).** The `http` block was `{"status": N}` and nothing else. It is
   now `{status, headers}`, `headers` a bounded, ordered allowlist —
   `retry-after`, `cf-mitigated`, `server`, `x-datadome`, `content-type` — that
   surfaces *exactly the evidence needed to tell a bot refusal from a genuine
   one* (§3.7). Surfaced from the *same* header capture `needs::challenge`
   decides on (single source, `src/envelope/http.rs`). `set-cookie` is
   **excluded** here — session material the cookie jar (`bl-6dad`) owns — and so
   are volatile per-request headers, to keep golden captures deterministic
   (§12). The **oracle allowlist** here is that same set; a golden capture that
   surfaces a header outside it, or omits a challenge marker inside it, is a
   drift. Additive: the output-schema change was escalated and approved (Mark,
   2026-07-20).
2. **Amazon's 202 is mislabelled `needs:["js"]`** (§3.6). — **DELIVERED
   (`bl-7e34`, 2026-07-22; evidence §3.9).** The defect was real; its stated
   *cause* was not. The body is not withheld — Amazon serves a 2007-byte AWS WAF
   `challenge.js` page and declares it with **`x-amzn-waf-action: challenge`**.
   So this was never a starvation-detector ambiguity needing a new taxonomy: it
   is a **declared challenge** (§3.7) whose vendor spelling `needs.md` §3 did
   not yet recognize. Adding that one declaration reclassifies it to
   `needs:["human"]` pre-parse, with **no new `needs` kind and no third envelope
   shape** (defect 3 unaffected). Note the correction to defect 1's parting
   suggestion: the separating signal is *not* `server: CloudFront` — a CDN name
   is not a refusal — it is the vendor **action** header, now allowlisted.
3. **Two envelope shapes for one condition** (§3.7): reddit's declared challenge
   is `needs:["human"]`, g2's is `error{http.403}`. Both mean "a bot defence
   refused us". A consumer must handle both.
4. **`ARCHITECTURE.md`'s StackOverflow justification is stale** — corrected in
   this pass. The underlying question (does the TLS masquerade earn its keep?)
   is now **answered** by `bl-d66b` (§3.8/§14 item 1): **not on access** (a
   measured null), but **yes on coherence and security maintenance** — the axis
   the epic was actually filed on. It earns its keep as engineering, not as a
   gate-opener; the docs now say that plainly rather than implying access value.
