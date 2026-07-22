# Capability-surface probe evidence (`bl-bd4e`)

Which capability surfaces the field corpus **actually probes** — measured, not
invented from folklore (`bl-0356`). Companion to `identity.md` §8/§10/§11 and the
Mark ruling landed in `bl-4896`: masquerade is in scope, bounded by a *coherence*
bar (a masqueraded value must be a deterministic, `BrowserProfile`-derived
simulation, or it is a louder tell than absence). This document is the dated
evidence that decides *which gaps are real*; the follow-up balls own each gap's
"simulate or interpret?" question.

## The instrument

`src/js/probe.rs` + `src/js/probe.js`, reached through `frot::js::measure`. A
second prelude wraps each watched surface so that reading or calling it records a
probe (the `__frot_probe` syscall) and then **returns exactly what frot returns
today** — the same `null` / `undefined` / value / throw. Recording without
changing the outcome is the whole point: the measured probe path is the path a
real `--js` run takes, and the honesty line (`identity.md` §10) is not crossed by
measuring. The deterministic half (the instrument + offline fixtures in
`src/js/probe/tests.rs`) runs under the 100% coverage gate; the live corpus is
driven by `examples/probe_corpus.rs` and is **dated evidence, not CI**.

**Watched surfaces:** canvas `getContext`/`toDataURL`/`getImageData`, WebGL
(`getContext`, `WebGL(2)RenderingContext`), `AudioContext`/`OfflineAudioContext`,
`document.fonts`, `navigator.mediaDevices`/`plugins`/`mimeTypes`/… (a fingerprint
watchlist), `screen.*`, `devicePixelRatio`, `Intl`, `crypto`, `WebAssembly`,
`Worker`/`SharedWorker`, `RTCPeerConnection`, `WebSocket`/`EventSource`,
`indexedDB`/`BroadcastChannel`/`Notification`, `performance`, `Date.getTimezoneOffset`,
`Function.prototype.toString`, and property-descriptor / prototype-shape probing
of the identity objects.

## Run conditions

- **Date:** 2026-07-21 (UTC), egress not pinned (client identity only).
- **Binary:** worktree `bl-bd4e` @ `aba1ab2` base, `cargo run --release`. **UA is
  still Firefox 121** (this worktree predates `bl-3972`'s persona pin to 140esr);
  the persona-fact incoherences below are already owned by `bl-3972`.
- **Recipe:** `--js` only (no `--css`); `StyleSource::Bare`; the shipping 1 s
  budget. Layout is not triggered, so `offsetWidth` font-measurement loops are not
  reached (and are structurally unobservable to the instrument anyway).
- **Challenge pages:** fetched with exactly one request and **never executed**
  (`bl-abe5` boundary).

## Ranked table — probed surfaces per page (live, 2026-07-21)

Count = times the surface was touched in scripts that executed.

| page (class) | http | scripts/errors/settled | probed surfaces (count) |
|---|---|---|---|
| en.wikipedia.org (control) | 200 | 5/4/true | `performance`(2) |
| news.ycombinator.com (control) | 200 | 1/0/true | — |
| lobste.rs (control) | 200 | 0/0/true | — |
| react.dev (control) | 200 | 10/5/false | `performance`(5), `navigator.userAgentData`(2), `Function.prototype.toString`(1), `navigator.platform`(1) |
| stackoverflow.com/questions (soft-gate) | 200 | 34/23/false | `Function.prototype.toString`(7), `navigator.platform`(1) |
| **amazon.com** (soft-gate, **202 block**) | 202 | 3/3/false | `crypto`(5), `navigator.webdriver`(5), `performance`(4), `screen`(4), `navigator.doNotTrack`(2), `navigator.platform`(2), `navigator.plugins`(2), `navigator.product`(2), `navigator.productSub`(2), `navigator.vendor`(2), `Worker`(1), `canvas.getContext(2d)`(1), `canvas.getContext(experimental-webgl)`(1), `chrome`(1), `navigator.hardwareConcurrency`(1), `navigator.languages`(1), `navigator.maxTouchPoints`(1), `navigator.sendBeacon`(1) |
| reddit.com (declared-challenge) | 200 | — not executed — | (declared challenge; `bl-abe5`) |
| g2.com (declared-challenge) | 403 | — not executed — | (declared challenge; `bl-abe5`) |
| fingerprintjs testbed (fp) | 200 | 1/0/true | `navigator.vendor`(11), `screen`(3), `Date.getTimezoneOffset`(2), `Intl`(2), `navigator.languages`(2), `navigator.maxTouchPoints`(2), `OfflineAudioContext`(1), `canvas.getContext(2d)`(1), `canvas.getContext(experimental-webgl)`(1), `canvas.getContext(webgl)`(1), `chrome`(1), `indexedDB`(1), `navigator.deviceMemory`(1), `navigator.hardwareConcurrency`(1), `navigator.oscpu`(1), `navigator.pdfViewerEnabled`(1), `navigator.platform`(1), `navigator.plugins`(1), `navigator.userAgentData`(1) |
| browserleaks.com/canvas (fp) | 200 | 3/2/true | `chrome`(1) |
| browserleaks.com/webgl (fp) | 200 | 3/1/true | `WebGL2RenderingContext`(1), `WebGLRenderingContext`(1), `canvas.getContext(webgl)`(1), `canvas.getContext(webgl2)`(1), `canvas.getContext(moz-webgl)`(1), `canvas.getContext(webgl2-compute)`(1), `canvas.getContext(webkit-3d)`(1), `chrome`(1) |
| webbrowsertools.com/canvas-fingerprint (fp) | 200 | 14/6/true | `chrome`(4), `screen`(4), `Function.prototype.toString`(3), `navigator.userAgentData`(2), `navigator.webdriver`(2), `Notification`(1), `canvas.getContext(2d)`(1), `devicePixelRatio`(1), `navigator.maxTouchPoints`(1), `navigator.permissions`(1) |

**The headline finding:** the access-focused corpus (controls + soft-gates)
barely touches high-entropy surfaces — its scripts throw early (react 5/10,
SO 23/34) or the fingerprinting rides external bundles that go dark. **Amazon's
202 soft-block page is the exception: it *is* a bot-fingerprinting script**, and
it probes `crypto`, `screen`, `canvas`(2d + experimental-webgl), `webdriver`,
`Worker`, and the persona facts — a direct read on what the WAF checks. The
fingerprint testbeds confirm the rest.

> **Amended 2026-07-22 (`bl-7e34`, `identity.md` §3.9).** Amazon's row above is
> still an accurate inventory of *what that script reads*, but calling it a
> "soft block" was wrong and the row is no longer a live measurement target.
> The 202 carries **`x-amzn-waf-action: challenge`**: it is AWS WAF's
> **declared JS challenge** (`challenge.js` + `window.gokuProps`), which
> computes a token and POSTs it to mint an `aws-waf-token` cookie. So its
> probes are not a gate a coherent persona can pass — they are one half of a
> challenge whose other half is work-and-submit, the `identity.md` §10 refused
> boundary. **frot now flips to `needs:["human"]` pre-parse and never executes
> it**, which is also why this row can no longer be re-harvested live. The
> capability balls it motivated (`bl-05e6`/`bl-f624`/`bl-cf3a`/`bl-1cb7`/
> `bl-342a`) all landed and keep their independent justification from the
> testbed rows below; none of them was ever going to open *this* gate.

## Three-case classification (union of probed surfaces)

frot's current answer is from a direct inventory (`--js`, this worktree). Firefox
reality is the 140esr persona (`identity.md` §2/§8).

### Case 1 — Absent-and-probed, and the absence is COHERENT (no action)

Firefox lacks these too, so `undefined` matches the persona.

| surface | frot | Firefox | probed by |
|---|---|---|---|
| `window.chrome` | undefined | undefined (Chrome-only) | amazon, fingerprintjs, browserleaks×2, webbrowsertools |
| `navigator.userAgentData` | undefined | undefined (FF unimpl.) | react.dev, fingerprintjs, webbrowsertools |
| `navigator.deviceMemory` | undefined | undefined (FF unimpl.) | fingerprintjs |

### Case 3 — Present-and-coherent (no action)

| surface | frot | Firefox | probed by |
|---|---|---|---|
| `navigator.platform` | `Linux x86_64` | `Linux x86_64` | react, SO, amazon, fingerprintjs |
| `navigator.vendor` | `""` | `""` | fingerprintjs(11), amazon |
| `navigator.product` / `productSub` | `Gecko` / `20100101` | same | amazon |
| `navigator.webdriver` | `false` | `false` (truthful) | amazon(5), webbrowsertools(2) |
| `navigator.maxTouchPoints` | `0` | `0` | fingerprintjs, amazon, webbrowsertools |
| `navigator.sendBeacon` | function (returns `false`) | function | amazon |
| `performance` (`.now`) | present | present | wikipedia, react, amazon |

*(Caveat: `performance.now` precision clamping is a separate coherence question,
already owned by `bl-e707` clocks. `Function.prototype.toString` is present-native
in the engine but **exposes frot's JS shims** when called on them — a real tell,
filed as `bl-3926`, so it is not clean Case 3.)*

### Case 2 — Present-but-INCOHERENT (a bug, owned already — not re-filed here)

| surface | frot | Firefox | owner |
|---|---|---|---|
| `navigator.languages` | `["en-US"]` | `["en-US","en"]` | `bl-3972` (§8) |
| `navigator.doNotTrack` | `null` | `"unspecified"` | `bl-3972` (§8) |
| `navigator.hardwareConcurrency` | `1` | pin `8` | `bl-3972` (§8) |
| `navigator.plugins` / `mimeTypes` | undefined | present (len 5 / 2) | `bl-3972` (§8) |
| `navigator.oscpu` / `pdfViewerEnabled` | undefined | `Linux x86_64` / `true` | `bl-3972` (§8) |
| `Date.getTimezoneOffset` | `420` (host TZ) | host-dependent | `bl-e707` (§9) |

### Case 1 — Absent-and-probed, and the absence is INCOHERENT (filed as gaps)

Firefox has these; frot returns `undefined` (or `null` for `getContext`). Each is
a measured gap → one follow-up ball, governed by `bl-4896`'s coherence bar.

| surface | frot | Firefox | probed by | ball |
|---|---|---|---|---|
| canvas 2D (`getContext('2d')`) | ~~`null`~~ → **context (bl-05e6, LANDED)**: deterministic, profile-seeded `toDataURL`/`getImageData` | context | amazon, fingerprintjs, webbrowsertools | **bl-05e6 ✓** |
| WebGL (`getContext('webgl'…)`, `WebGL(2)RenderingContext`) | ~~`null` / undefined~~ → **context + branded interfaces (bl-f624, LANDED)**: masked `VENDOR`/`RENDERER` = `"Mozilla"`, coherent Mesa/llvmpipe `UNMASKED_*`, deterministic `readPixels`/`toDataURL` | context / present | fingerprintjs, browserleaks/webgl, amazon | **bl-f624 ✓** |
| audio (`OfflineAudioContext`/`AudioContext`) | undefined | present | fingerprintjs | **bl-8733** |
| `screen.*` + `devicePixelRatio` | undefined | present | amazon, fingerprintjs, webbrowsertools | **bl-1cb7** |
| `crypto` (getRandomValues/randomUUID/subtle) | undefined | present | amazon (WAF, ×5) | **bl-cf3a** |
| `Intl` (ECMA-402) | undefined | present | fingerprintjs | **bl-ac8d** |
| `indexedDB` | undefined | present | fingerprintjs | **bl-8dde** |
| `Worker` / `SharedWorker` | undefined | present | amazon | **bl-342a** |
| `Function.prototype.toString` (shim-detection) | reveals JS source | `[native code]` | react, SO, webbrowsertools | **bl-3926** |
| `navigator.permissions` + `Notification` | undefined | present | webbrowsertools | **bl-1548** |

## Limitations (deliberate, per the no-folklore rule)

1. **Under-observation via subfetch failure.** frot records a probe only in
   scripts that execute. Fingerprinting shipped in an external bundle that fails
   `subfetch` or throws early is invisible (SO threw 23/34, react 5/10). So
   **absence of a surface from this table is not evidence a browser wouldn't probe
   it — only its presence is load-bearing.** No ball is filed on an unobserved
   surface.
2. **The `offsetWidth` font channel is unmeasurable.** Glyph-width font
   enumeration is indistinguishable from ordinary layout reads; the instrument
   watches the `document.fonts` API only, which the corpus did not probe. The
   width channel is recorded as a **residual** in `identity.md` §11, not a gap.
3. **`Function.prototype.toString` is an aggregate** — it fires on ordinary
   library code too, so its counts are a weak signal; `bl-3926` scopes any fix to
   frot's own shim functions.
4. **`--css`/layout off**, so geometry-triggered probes (`getBoundingClientRect`
   fingerprinting) were not exercised; none appeared, and none is claimed.
