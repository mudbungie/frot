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
its own `PINNED` capture date and its line's `EOL` date, and **a unit test fails
once `EOL` passes** (see I8) — silent staleness becomes a build failure, with no
config knob and no calendar to remember. *(The 140esr EOL date is taken from
Mozilla's published ESR calendar at implementation time; it is not measured
here.)*

---

## 3. Evidence matrix

All rows measured 2026-07-19 from the **same egress IP `[redacted-egress-ip]`**, so
client identity is separated from IP reputation. frot binary: worktree `bl-0356`
@ `4aac8b2`, `cargo build --release`.

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

### 3.7 Negative controls — declared challenges (one request each, never executed)

| site | frot envelope | HTTP | declaring header |
|---|---|---|---|
| reddit.com | `needs:["human"]` — **correct** | 200 | `retry-after: 0`, `server: snooserv` |
| g2.com | `error{kind:"http.403"}` | 403 | `x-datadome: protected`, `server: cloudflare` |

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

### 4.2 Derived — computed, never stored

| derived fact | from |
|---|---|
| `Accept-Language: en-US,en;q=0.5` | `locale` (the q-value is Gecko's rendering of `en-US, en`) |
| `navigator.language` / `.languages` | `locale` — **the same source as the header**, which is why they can no longer disagree |
| `navigator.userAgent` / `.appVersion` | `user_agent` (`appVersion` = UA minus the `Mozilla/` prefix) |
| `navigator.platform` / `.oscpu` | the UA's platform segment |
| every JA3/JA3N/JA4/JA4_r/JA4_ro/peetprint/akamai hash | **computed from the capture**, never hand-written (§12) |
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
- **I8 — The profile expires.** A test asserts `today < profile.eol`. When the
  pinned ESR line goes end-of-life the build fails, forcing a re-capture. No
  calendar, no reminder, no knob.

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
See §10 for the boundary; the VISION principle-5 question this once raised is
**resolved** (Mark, 2026-07-20 — no conflict; §10).

---

## 9. Cookies and clocks

**Cookies (`bl-6dad`).** One per-invocation jar, born empty, discarded at exit —
shared by the transport (`Set-Cookie` / `Cookie`) and `document.cookie`, which is
today an isolated in-memory string in `env.js`. This is **state within a call**,
which VISION principle 1 explicitly permits (*"State within a call — redirects,
JS event loop, etc. — is fine; nothing persists across calls"*). It is not a
session model: nothing is written to disk, nothing survives the process, and
there is no `--cookie-jar` flag. The coherence win is that a server that sets a
cookie on the document GET sees it returned on subresource fetches, as a browser
would — today it never does.

**Clocks (`bl-e707`).** Clocks derive from the profile plus the existing virtual
clock (`js.md` §5), not from a second source. Two facts to settle in that ball:
timer-precision clamping (Firefox clamps `performance.now()`/`Date.now()` to 1 ms
by default under `privacy.reduceTimerPrecision`, and an unclamped
sub-millisecond timer is itself a tell), and timezone/locale, where the capture
shows host-dependent values (`America/Los_Angeles`, offset 420). **Recommendation:
pin locale to the profile and timezone to UTC**, on the same determinism argument
as `hardwareConcurrency` — accepting "a UTC browser is unusual" as a declared
residual (§11), because non-deterministic output is the worse failure.

---

## 10. Scope boundary

This supersedes the blanket non-goal in `js.md` §11. The boundary **moved again on
2026-07-20** (Mark's masquerade ruling), days after `bl-0356` last moved it. The
line is drawn on a principle, not a list.

> **Superseded 2026-07-20 — the old drawing of the line.** This section, and
> `js.md` §11, previously read: *"frot may present a coherent identity for a
> client that genuinely has the capabilities it claims. It may not fabricate
> evidence of capabilities it does not have,"* with canvas/WebGL/audio/font/
> media-device fabrication listed as **"out, permanently"**. Mark superseded that:
> *"I don't mind masquerading capabilities. We want to match what the server is
> requiring as much as possible. If there are gaps on what we can interpret on our
> side, we should file backlogs for them, to figure out how to simulate/interpret
> them."* The old text is kept here as superseded, not deleted.

**The line as it now stands** — note it separates *two different axes* that the
old single rule conflated:

> **Matching what the server requires is in scope, including by masquerading a
> capability frot does not physically have. What is refused is a different axis:
> executing or solving a challenge, and evasion loops. A signal frot cannot yet
> interpret or produce is a *filed gap*, not a permanent non-goal.**

**In scope:**

- Deriving TLS, ALPN, HTTP version, headers, cookies, `navigator`, and clocks
  from one profile, mutually consistent.
- Genuinely negotiating everything advertised (I2).
- Low-entropy coherent facts and correct Gecko branding (§8).
- **Masquerading a capability to match what the server requires** — including the
  high-entropy rendering surfaces (canvas/WebGL/audio/font metrics/media-device)
  that were previously refused. The bar is **coherence, not abstinence**: a
  masqueraded value must be a coherent, profile-derived *simulation*, because an
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
  unbuilt, not because it is forbidden.

**Refused — a different axis, unchanged by the 2026-07-20 ruling** (the `bl-abe5`
hard boundary):

- **Executing or solving any challenge.** CAPTCHA, JS proof-of-work, behavioural
  interstitials. A *declared* challenge (`needs.md` §3 — `Retry-After` on a 2xx,
  `cf-mitigated: challenge`) is reported as `needs:["human"]` **before its
  scripts are ever executed**. Detection is refusal to pretend, not a step
  toward evasion.
- **Evasion loops** — no UA rotation, no IP rotation, no retry-until-allowed, no
  backoff-and-try-again. One request, one answer.
- **Body-copy classification** of block pages (`needs.md` §5 — fragile, and the
  first step down the evasion road).
- **Submitting anything.** GET-only, permanently — the frottage rule.

Why this line is principled rather than arbitrary: it separates **matching a
requirement** (masquerade, now in) from **defeating a defence** (challenge-solving
and evasion, still out). `webdriver: false` is in because it is *truthful*;
speaking h2 is in because I2 makes frot *actually* speak it; a simulated canvas
hash is in *as a filed gap* because it matches what a fingerprinter requires — but
solving a CAPTCHA stays out because that is defeating a defence, not matching an
identity.

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
| **Required POST telemetry** | Permanently refused — GET-only frottage rule. Sites that gate on a beacon POST cannot be served. `sendBeacon` returns `false` (a legal denial). |
| **Proof-of-work challenges** | Refused (§10). |
| **Behavioural challenges** (mouse paths, dwell time, scroll) | Refused — frot dispatches only the `DOMContentLoaded`/`load` lifecycle pair, never synthetic input. |
| **High-entropy rendering** (canvas/WebGL/audio/font metrics) | **No longer refused (2026-07-20).** In scope as a coherent masquerade; **unbuilt today**, so a filed gap — `bl-bd4e` and follow-ups (§10). Not a permanent residual. |
| **Font metrics via `offsetWidth` measurement loops** | The `document.fonts` / FontFaceSet API is a filed gap (a follow-up of `bl-bd4e`), but the **`offsetWidth`-based glyph-width channel is a residual**: frot's layout is a structural approximation (`layout.md` §6), so per-glyph text widths cannot be reproduced faithfully, and a *wrong* width is a louder tell than a missing font (§10's coherence bar). It is also unobservable to the `bl-bd4e` instrument (indistinguishable from ordinary layout reads), so no page can be cited as probing it — it is out by the no-folklore rule, not measured in. |
| **`crypto.subtle` (WebCrypto)** | `crypto.getRandomValues`/`randomUUID` are cheap capabilities frot can genuinely provide (a `bl-bd4e` follow-up owns them); the full `SubtleCrypto` surface is a large capability that may stay residual — the filed gap decides. |
| **Three key shares** | Blocked by rustls (§6.4). |
| **ClientHello extension order/set** | Stock rustls emits its own order and omits Firefox's `compress_certificate`/SCT/`record_size_limit`/ECH shaping. Declared residual under Option C (§6.1); would need a rustls fork, which §6.3 forbids on security grounds. |
| **Cipher *list* breadth (9 vs 17) → JA4 cipher component** | rustls advertises only its AEAD suites, not Firefox's legacy CBC/RSA. The cipher *order* matches; the *list* (and thus the JA4 cipher hash) does not. Declared residual (§6.1). |
| **secp521r1 + FFDHE2048/3072 groups** | Not offered by aws-lc-rs; the other four persona groups match in order (§6.1). |
| **`m,p,a,s` pseudo-order + HEADERS PRIORITY** | Deferred to stage C (§7); the `h2` crate hardcodes `m,s,a,p` and exposes no client PRIORITY. |
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
list, rustls's extension order, ≤2 key shares, `m,s,a,p`) with a pointer to
§6.1/§11, exactly as §1 requires ("coherent… explicitly not byte-identical"). A
field is either an exact Firefox match *or* a residual asserted against frot's
own stable output; neither may drift silently. This is the honest bridge between
this section's "match exactly" and §1's "not byte-identical": the residuals are
the difference, and they are tested, not hidden.

**Not normalized — an exact Firefox match where reachable, else a pinned
residual.** Ordered capabilities are identity:

- cipher list **and wire order** (17, `0xc009` at index 10)
- extension list **and wire order** (17, ECH last)
- `supported_groups` **and order** (`4588,29,23,24,25,256,257`)
- `key_share` **groups and order** (4588, 29, 23)
- `signature_algorithms` **and order** (11)
- ALPN list and order; negotiated protocol
- `record_size_limit` = 16385; `compress_certificate` = zlib, brotli, zstd
- absence of GREASE
- h2 SETTINGS: **4 entries, values and order**; WINDOW_UPDATE 12517377; first
  HEADERS on **stream 3**; `PRIORITY` flag with weight 42 / depends_on 0 /
  exclusive 0; pseudo-order
- request header **names, order, and values**, per destination
- h1 header **casing**, asserted on **both** schemes (I3 — this is the regression
  test for §3.4)
- the §8 `navigator` fact set
- the envelope `http.headers` allowlist (`bl-acec`): `retry-after`,
  `cf-mitigated`, `server`, `x-datadome`, `content-type` — lower-cased names in
  wire order, repeats preserved. Deterministic by construction (`set-cookie`
  and volatile per-request headers are excluded), so it is a golden field, not a
  normalized one; a surfaced header outside the allowlist is a drift.

**Hashes are assertions, not fixtures.** JA3/JA3N/JA4/JA4_r/JA4_ro/peetprint/
akamai values are **computed from the capture and compared to the profile's
derived values** — never stored as hand-written strings (I6). This is what makes
a pin change a one-table edit (§4.2).

**Declared residuals are asserted too.** Stage B's `m,s,a,p` is written into the
oracle *as the expected value with a pointer to §7 stage C*, so it cannot drift
silently and cannot be mistaken for a match. A residual that isn't in the oracle
is a residual that will be forgotten.

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

1. **It is not proven to change any outcome.** No A/B was run. The one hard case
   that reproduces (Amazon 202) is **not** shown to be fingerprint-caused, and
   the case that motivated the whole masquerade (StackOverflow) **no longer
   reproduces** (§3.6). The justification standing on measured ground is
   *coherence* and *security maintenance* (§6.3) — not measured access
   improvement. `bl-d66b`'s corpus re-run is the falsifier; if it shows no
   change, that result belongs in this section, not in a drawer.
   **Filed responses (Mark's ruling, 2026-07-20 — *"on a/b, well we should
   probably file some tests then!"*):** the A/B gap is now owned by **`bl-46f5`**
   (field-corpus A/B harness) and capability-gap *measurement* — which signals a
   server requires and whether frot produces a matching one — by **`bl-bd4e`**.
   The gap is tracked, not merely noted.
2. **IP/ASN reputation probably dominates.** Everything here is client identity.
   If a target scores the egress IP, a perfect profile changes nothing.
3. **Determinism makes frot a cohort.** Pinning `hardwareConcurrency`, timezone,
   and screen means *every frot install looks identical*. That is itself a
   fingerprint — just a *consistent* one rather than a *self-contradictory* one.
   **The goal was never to be unidentifiable; it is to not be incoherent.** Say
   this plainly rather than letting a reader infer stealth.
4. **Most residuals in §11 are permanent under the current constraints**, and two
   of them (key shares, pseudo-order) are inside the fingerprint a serious
   defence actually hashes. *(One row is no longer permanent: high-entropy
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
2. **Amazon's 202 is mislabelled `needs:["js"]`** (§3.6). An empty body the
   server *withheld* is not a page that needs JS. The starvation detector
   (`needs.md` §4) cannot currently tell the two apart. **Unblocked, not yet
   fixed:** the separating signal — the response headers, defect 1 — is now
   observable (`http.headers` surfaces the `server` header), so the
   reclassification can key off it; the reclassification itself is a separate
   ball (per `bl-acec`'s acceptance — additive header-surfacing does not change
   the `needs` verdict).
3. **Two envelope shapes for one condition** (§3.7): reddit's declared challenge
   is `needs:["human"]`, g2's is `error{http.403}`. Both mean "a bot defence
   refused us". A consumer must handle both.
4. **`ARCHITECTURE.md`'s StackOverflow justification is stale** — corrected in
   this pass, but the underlying question (does the TLS masquerade earn its
   keep?) is open until `bl-d66b`.
