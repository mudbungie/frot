# Vision

## The problem

There is a gap between `curl` (raw bytes, no semantics) and a full headless browser (Chromium, hundreds of MB, slow start, stateful, fragile). Harnesses live in this gap. They need to look at web pages programmatically — fetch a doc, read its structure, find a link, extract a table, see what an accessibility tree would expose — without standing up a browser pool.

The available options force a binary choice: too little (curl + regex, hope the page is SSR'd, brittle), or too much (Playwright, browser pool, container). Nothing sits cleanly in the middle as a small, fast, stateless tool that produces *machine-readable impressions* of the web.

## The name

`frottage` is an impression technique: press a surface against paper, rub graphite or charcoal across it, and capture the topology — outlines, raised features, structure — **without altering the original**. Archaeologists use it for inscriptions. Artists use it for surface texture. The metaphor maps to this tool with uncanny precision:

- No interaction with the origin beyond reading. When JS is enabled, it runs against our local DOM copy; we don't submit forms or fire mutating requests back.
- Output is a *structural impression* (DOM, text, AX tree, optionally geometry), not a faithful pixel render.
- The technique is fast, deterministic, and non-destructive.

The name scales with scope. Even when JS is in play, we're still taking an impression of what the page becomes — not driving it.

## Principles

1. **Stateless per process.** A single invocation is `(url, flags) → one output`. State *within* a call (redirects, JS event loop, etc.) is fine; nothing persists across calls. Calls are cacheable, reproducible, parallelizable.
2. **Single static binary.** No container, no runtime deps, no Chromium install. Drop on a machine, run.
3. **Minimal footprint.** Optimize for binary size, startup time, and memory floor. The interesting envelope is "smaller than a curl wrapper, more useful than one."
4. **Machine-first output.** Every output format is designed to be parsed, not displayed. AX tree is a first-class citizen, not an afterthought.
5. **Honest capability signals.** When a page can't be rendered faithfully under the current capability recipe, return a clear, structured `needs-X` signal. Never silently produce a degraded result that looks complete. Honesty covers transport facts too: the HTTP outcome (a 404, a 500, a bot-challenge 403) is part of the impression and must be surfaced in the envelope, never swallowed. A flag that is accepted must do what it says or be rejected.

   **Clarification (Mark, 2026-07-20) — principle 5 governs the *delivered impression*, not the *wire persona*.** This principle is about being honest to the user about **what happened**: the `--out *` payload, `needs`, and the `http` block must truthfully report the impression frot obtained. It does **not** constrain the browser identity frot masquerades on the wire to obtain that impression. Presenting a masqueraded fingerprint (canvas/WebGL/TLS/`navigator`) is a legitimate client-side choice — analogous to a private browsing window — *"so long as you're safe on the wire, it's up to the client what you do with what you're delivered."* So capability masquerade (`docs/design/identity.md` §10) does **not** conflict with principle 5, and no exception clause is carved into it. One engineering constraint stands, distinct from honesty: a masqueraded value must be a **deterministic, profile-derived function**, never random or incoherent, because an incoherent fabricated value is a louder tell than absence.
6. **Elegance over completeness.** It's better to do less, well, with a clean boundary, than to half-implement the whole web. Where compromise is forced, push back.
7. **Testability is non-negotiable.** Every capability ships with golden-fixture tests against pinned real-world pages. If it can't be tested, it isn't built.

## The model

frot is composed of two orthogonal axes:

- **Capability flags** describe *what to do* before producing output. They are transformations on the document:
  - `--css` — parse and apply stylesheets, annotating the DOM with computed styles. Visibility (`display:none`, `visibility:hidden`) and generated content propagate to all subsequent outputs. Pre-interaction chrome — hover/click-gated dropdowns, mega-menus, language switchers — is therefore expected to be absent from `--out text`/`--out ax` under `--css`, permanently and regardless of `--js`: frot never synthesizes interaction (see non-goals below). See `docs/design/css.md` Finding 2 for the traced field-trial evidence.
  - `--js` — execute scripts against the current DOM. JS mutates the DOM the way it does in a real engine, within bounded, stateless execution semantics.
- **The output flag** picks *what to return*. Exactly one of: `dom`, `text`, `ax`, `links`, `forms`, `bboxes`, `meta`.

```
frot <url> [--css] [--js] [--js-errors] --out <view>
```

The caller composes a recipe of capabilities; frot returns one output reflecting the document under that recipe. Internally, capabilities are composable functions over a shared artifact (DOM + side tables); they are not a strict pipeline. JS can run without CSS, and the DOM that downstream readers see is the DOM as the chosen capabilities have left it.

Layout (block / inline / flex) is a *derivation*, not a capability. There is no `--layout` flag. Layout runs implicitly when something needs geometry — an output like `bboxes`, or a JS script reading `offsetWidth`. Outside that, it is skipped entirely.

When the recipe is insufficient — e.g. the page is an empty SPA shell and `--js` is off — frot emits a structured `needs-X` signal rather than a misleadingly thin output.

## The scope arc

frot is built in phases. Each phase delivers a usable tool; later phases extend the capability envelope without changing the interface.

### Phase 0 — Fetch + parse + envelope — **landed**

- HTTP client with sane defaults (redirects, content-encoding, character set detection). **Landed as HTTP/1.1-only via `ureq`; replaced in Phase 5** by an HTTP/2 client (hyper + `rustls 0.23`/`aws-lc-rs`, h1.1 fallback by ALPN) so the ALPN now offers `h2, http/1.1` and h2 is selected. *(History: h1-only was first recorded as "a residual soft tell", then corrected 2026-07-20 to the dominant tell — a Firefox-shaped ClientHello that refuses h2 is a combination no real Firefox produces, and JA4 embeds ALPN (`…h2` vs `…h1`) so an h1-only client cannot match a Firefox JA4 by construction. Phase 5 fixed it — `docs/design/identity.md` §7/§3.8.)*
- HTML5 parsing (via `html5ever`).
- CLI surface and the machine-first output envelope. Outputs at this phase: `dom`, `text` (raw, source-order, no visibility filter), `links`, `forms`, `meta`.
- Browser-identity masquerade: a Firefox User-Agent and a Firefox-leaning TLS ClientHello behind `src/fetch/firefox_tls.rs`. **Landed on the `craftls` fork (`FIREFOX_105`); replaced in Phase 5 (`bl-abca`, Option C)** by stock `rustls 0.23` + `aws-lc-rs` deriving its shape from the one Firefox-140-esr profile — no ClientHello-crafting fork. *(History: the StackOverflow 403 this cleared no longer reproduces (HTTP 200, 3/3 runs, measured egress), so the masquerade's access value was unproven and `craftls` — sole release 2024-01-16, rustls 0.22, no CVE path — was a live security liability. Phase 5 retired it; the capstone `bl-d66b` confirmed the swap keeps every gate result unchanged while restoring `cargo update` CVE reach — §6.3/§3.8.)*
- The refused boundary, restated precisely (`docs/design/identity.md` §10). **Amended 2026-07-20 (Mark's masquerade ruling):** matching what the server requires is in scope, **including by masquerading a capability frot does not physically have** — canvas/WebGL/audio/font fingerprint simulation is now in scope (as a coherent, profile-derived value; unbuilt today, so a *filed gap* — `bl-bd4e` — not a permanent non-goal). *(The previous line read that such fabrication was refused; superseded, not deleted.)* What stays refused is a **different axis**, untouched by the ruling: executing or solving a challenge (CAPTCHA / anti-bot JS-challenge / proof-of-work) and evasion loops (UA rotation, retry-until-allowed). Masquerade does not conflict with principle 5 — that governs honesty of the *delivered impression*, not the *wire persona* (see principle 5's 2026-07-20 clarification).
- The envelope shape, the `needs-X` signal shape, and the error taxonomy are load-bearing and do not change in later phases.

### Phase 1 — Semantic impression — **landed**

- AX tree from semantic HTML + ARIA, computed without styling. The headline output: `--out ax`.
- The `needs-X` taxonomy lands here; `needs-js` is its first inhabitant — a heuristic signal that the page is an SPA shell and the impression under the current recipe is empty.

### Phase 2 — CSS application — **landed**

- `--css` capability flag.
- Parse and apply CSS for visibility and content semantics: `display:none`, `visibility:hidden`, pseudo-content. Implemented as a small dependency-free subset engine rather than pulling in `cssparser`/`selectors` — the binary-size and dependency-discipline constraints outweighed full CSS fidelity for a narrow visibility/content need. External `<link>` stylesheets are fetched best-effort (failures skipped); `@media` width/screen queries are evaluated against the fixed 1280×720 viewport (the same evaluator JS `matchMedia` uses — mobile-first pages render as a desktop browser would); `@supports` is out of scope; layout (and thus list-marker generation) defers to Phase 3.
- Subsequent outputs (text, AX) reflect what's *visible*, not what's in the markup.

### Phase 2.5 — Impression fidelity — **landed**

Debt surfaced by the 2026-07 arch pass; it precedes new capability because it is the honesty principle catching up with the shipped surface:

- Surface the HTTP outcome in the envelope (additive `http` block). Today a 404/500/bot-challenge body is reported as an `ok` impression. *(Extended in Phase 5, `bl-acec`: `http.headers` surfaces a bounded allowlist of decision-relevant response headers — `retry-after`, `cf-mitigated`, `server`, `x-datadome`, `content-type` — so the evidence behind a `needs`/error signal is observable, not discarded. `set-cookie` and volatiles stay out.)*
- Reject `--js` with a usage error until Phase 4 implements it; today it parses and silently does nothing.
- Accessible-name computation restricted to `nameFrom: contents` roles. Today container roles (`table`, `rowgroup`, …) are named from their full subtree text, so ancestor names repeat all descendant content — bloat for the AX view's primary consumer.
- Layout tables demoted to `presentation` in the AX tree (a DOM-based heuristic, as browsers do): a caption-less, header-less table used for layout should contribute content, not `table`/`row`/`cell` structure noise.

The live decomposition of this stage (and everything else) is the `bl` backlog, not this file.

### Phase 3 — Layout (on-demand) — **landed**

- Block flow, inline flow, basic flex. Enough to compute reading order and reasonable bounding boxes for elements.
- New output: `--out bboxes`. AX reading order is refined against layout — and since exactly one view runs per call, `--out ax` under `--css` triggers layout itself, so flex `order` / `flex-direction: *-reverse` surface as visual reading order; without `--css` no reorder is possible and layout is skipped, so source order stands.
- Triggered implicitly by output demand or by JS reading geometry; not a flag.

### Phase 4 — JavaScript — **landed**

- `--js` capability flag. Page scripts run against the *real* arena (not a shimmed copy) through a narrow host-syscall table plus a JS prelude — the one capability that mutates the DOM, run first so the rest of the pipeline consumes the post-JS document. Design: `docs/design/js.md`.
- Bounded execution as promised: a virtual-clock event loop under a single wall-clock deadline (1 s, covering script *and* network time), no timers firing past load (a 10 s virtual horizon self-terminates `setInterval`/`requestAnimationFrame`), a 64 MiB engine heap, no persistence (`localStorage`/`sessionStorage`/`cookie` in-memory + born-empty, `indexedDB` a coherent but stateless masquerade — present so feature detection passes, with no backing store, so an `open()` never completes), and `fetch`/XHR served GET-only once-then-frozen.
- Engine chosen at first claim: **`rquickjs`** (Rust bindings to the maintained quickjs-ng fork) over Boa — Boa cannot *bound* untrusted execution (no interrupt hook, no memory limit), which was disqualifying. Vendored C via `cc`, static musl holds, +1.18 MiB. The engine sits behind a swappable seam (`src/js/engine.rs`); Boa remains the documented fallback.
- Sites that exceed the shim get `needs-js`, same as before — the post-JS document is re-tested, so `needs-js` now means "needs more JS than the shim gives." The boundary moved; it did not disappear. Partial runs are additionally reported through an envelope `js:{scripts, errors, settled}` block; `--js-errors` (requires `--js`) adds a bounded `messages:[{kind, text}]` array — the *what* behind the count, opt-in and additive.
- **ES modules ship (`bl-1b98`).** A `type="module"` script evaluates as a real module: `import` specifiers resolve against the importing module's URL and load through the same once-then-frozen subfetch cache, top-level `await`/dynamic `import()` settle inside the one budget. Bare specifiers have no import map, so they are unresolvable (a counted error) as in a browser without one. It rides `rquickjs`'s `loader` feature — the one new dependency (`relative-path`), approved 2026-07-10.
- **Non-goals held honest (`docs/design/js.md` §11).** No interaction (no synthetic clicks/input/scroll — the only events dispatched are the `DOMContentLoaded`/`load` lifecycle pair), no navigation, no persistence, no iframes/workers/WASM/canvas rendering, not a stealth runtime.

### Phase 5 — Network identity — **landed** (`docs/design/identity.md`)

Not new capability: the honesty principle catching up with the transport, the way Phase 2.5 caught up with the output. The 2026-07-19 same-egress measurement found frot's identity was **incoherent** — a Firefox-121 UA over a 2022-era ClientHello that refuses h2, a Chrome-shaped `Accept`, header casing that changes with the URL scheme, and one TCP connection per request with no reuse. Every one of those is now gone (capstone re-measurement, `bl-d66b`, 2026-07-21, §3.8): h2 negotiated, a coherent Firefox-140-esr UA/`navigator`/`Accept`, one request serializer (scheme-independent casing), and one reused connection per page. The honest verdict is stated plainly below.

- **One `BrowserProfile` constant is the single source of truth.** TLS, ALPN, HTTP version, headers, cookies, `navigator`, and clocks all *derive* from it. Every fact has one definition site; the fingerprint hashes are computed, never stored.
- **The persona is pinned to Firefox `140.12.0esr`** (Mozilla tarball, not the Ubuntu snap) — an ESR line so the fingerprint holds still, and one whose cipher list frot already emits exactly.
- **One invariant carries most of the design: never advertise what you cannot speak.** ALPN ⊆ protocols implemented, `Accept-Encoding` ⊆ encodings decodable, every advertised group genuinely negotiable. That is what makes the client coherent rather than a costume, and it removes the "should we offer h2?" question — the profile declares h2, so the transport must speak it or the build fails.
- **Retired `craftls` for `rustls 0.23 + aws-lc-rs` and added real `h2` — LANDED (`bl-abca`, Option C, Mark's ruling 2026-07-21).** The transport is now hyper (`hyper-util` + `hyper-rustls`, aws-lc-rs, h1+h2) driven by a per-invocation `tokio` runtime + `block_on`; `ureq` is gone. This discharges the security argument §6.3 calls load-bearing: a rustls CVE now reaches frot by `cargo update` (the dead 0.22 fork gave it no path). The "blocking I/O, no async" decision is superseded to *external-contract-blocking, async-permitted-internally* (Mark: async internally is fine, returns must be posix-compliant/lifecycle-deterministic) — the public fetch API stays blocking, no work escapes the call. As-built static-musl size is **7.63 MiB** (up ~0.4 MiB for the hyper/tokio/h2 stack; the earlier −32 KiB estimate was a seam-swap-only figure that predated adding the h2 client), inside the 5–15 MiB envelope. `Accept-Encoding` stays `gzip, br` (zstd/deflate decoders remain a declared residual, I2). The pure-Rust `ring` + `libcrux-ml-kem` path is preserved as the size-optimized fallback.
- **The falsification rule fired, and the ceiling is reported rather than dressed up.** No route delivers a byte-exact Firefox inside frot's constraints: every stack that reproduces the measured profile is built on BoringSSL (C++), and `musl-tools` ships no C++ compiler. Under Option C, stock rustls's own ClientHello shape (10 ciphers, 11 extensions, ≤2 key shares) and the `h2` crate's `m,s,a,p` pseudo-order are **declared residuals**, not silent misses — so frot's JA4 is `t13d1011h2_…`, deliberately not the pin's `t13d1717h2_…`. The goal is **coherent, current, and maintained — explicitly not byte-identical.** frot is not trying to be unidentifiable; it is refusing to be self-contradictory.
- **The capstone measured the payoff, and it is a null on access — recorded, not buried (`bl-d66b`, §3.8/§14).** After every sibling landed, the field corpus is outcome-identical to the pre-phase baseline: no gate that failed now succeeds, none that succeeded now fails (StackOverflow still does not reproduce its gate; Amazon still soft-blocks 202). The phase earns its keep on **coherence** (the incoherences above are gone) and **security maintenance** (§6.3) — not on bot-gate bypass, which frot never claims and still refuses at declared challenges (`needs:["human"]`).

### Phase 6+ — TBD

If Phase 4 holds, push toward more Web APIs. If it doesn't, accept the ceiling and ship a clean tool that defers to a real browser for the long tail.

## What this is not

- Not a browser. Not a competitor to Playwright or Browserless.
- Not a screenshot tool. Pixels are not the output.
- Not a business. This is a harness component, shared for portability.

## The non-goals discipline

Resist three temptations specifically:

- **Don't add conditionals to the parameter schema.** The moment frot needs to express "click X if Y is present," it becomes a browser-automation DSL, badly. The escape hatch for that case is *a different tool*, not a feature.
- **Don't grow a session model.** Statelessness is load-bearing. If a workflow needs sessions, the harness composes calls; frot does not maintain them.
- **Don't optimize for the long tail of broken sites.** Coverage of the easy 70% with a tiny binary is worth more than 95% with a heavy one.

## What success looks like

A 5–15 MB static binary that returns a faithful structural impression of a web page in single-digit milliseconds at the lower capability tiers (sub-second once JS is in play), exposes a clean AX-tree output that LLM agents prefer over raw DOM, fails honestly when the page demands more, and has a roadmap that can be ignored without the tool feeling incomplete.
