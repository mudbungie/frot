# JS execution design (Phase 4)

Living design for frot's `--js` capability. Scope-setting document: it decides
the engine, the DOM binding architecture, the execution semantics (event loop,
timers, budgets), the network and persistence policy, the honest-signal story,
and the non-goals *before* any JS code exists. It fits inside `VISION.md`
("Phase 4 — JavaScript", principle 1 "stateless per process", principle 5
"honest signals", principle 6 "elegance over completeness") and extends the
as-built patterns in `ARCHITECTURE.md`. When code and this doc disagree, fix
whichever is wrong.

## The one-sentence shape

`--js` is the one capability permitted to mutate the DOM: page scripts run in
an embedded engine against a handle-based facade over the *same* arena
everything else reads (single source of truth — no mirror DOM), under a
bounded, virtual-clock event loop; when execution settles, the rest of the
pipeline — needs detection, CSS, layout, views — consumes the post-JS document
exactly as it consumes a static one.

## 1. Engine choice

**Decision: `rquickjs` (Rust bindings to the maintained `quickjs-ng` fork).**
Boa was the pure-Rust contender; rquickjs wins on every load-bearing criterion
(surveyed 2026-07; the spike, subtask 1, re-validates before code lands):

> **Spike result (2026-07-08, bl-b5df) — non-falsifying; rquickjs confirmed.**
> The seam landed at `src/js/engine.rs` (rquickjs 0.12, `default-features =
> false` so no `allocator` feature — the default C allocator, per the caveat
> below). All four §1 primitives proven by test: `eval`; the wall-clock
> interrupt handler stops `while(true){}` inside a 20 ms budget; the
> `set_memory_limit` cap throws on a 256 MiB allocation under an 8 MiB heap
> (the caveat holds — the limit **is** enforced on the default allocator); and
> the host-driven job queue drains a `Promise.then` microtask. A real
> `react.production.min.js` UMD bundle parses and runs (`React.createElement`/
> `useState` become functions), needing only a `self` → global alias (subtask
> 8). **Static musl build proven:** `x86_64-unknown-linux-musl` links a fully
> static, stripped binary and the engine test suite passes when run under it —
> vendored quickjs-ng C via `cc`, no cmake/bindgen/system dep (build host needs
> the `musl-tools` cross-cc `x86_64-linux-musl-gcc`; note for CI/subtask 10).
> **Binary delta (corrected):** the reachable-code delta is **+1.18 MiB → 4.59
> MiB** total (the ~+0.8 MB / 4.3 MiB estimate below undercounted `Context::full`
> intrinsics), still comfortably inside the 5–15 MB VISION envelope.

- **Bounding primitives exist — this is the disqualifier.** rquickjs's sync
  `Runtime` has `set_interrupt_handler` (periodic callback → wall-clock
  budget), `set_memory_limit`, `set_max_stack_size`, and a host-driven job
  queue (`execute_pending_job`) — exactly §5, at the engine level. Boa has
  **no interrupt hook and no memory limit at all** (only stack/recursion/
  loop-iteration `RuntimeLimits`); its maintainer confirms no timeout
  termination exists (boa-dev/boa discussion #3238). Untrusted page scripts
  cannot be bounded on Boa today.
- **Real-world robustness.** quickjs-ng passes near-100% of test262 for its
  ES2025 feature set including Annex B (legacy web compat — jQuery-era code);
  AWS LLRT is built on rquickjs and runs server-rendered React in production.
  Boa is at 94.12% test262 (v0.21) with no published evidence of running
  framework bundles. Interpreter-class speed (~6–7× Boa in the 2025 V8-suite
  benchmark, sub-ms runtime startup) is ample for page-init scripts.
- **Size.** Measured: ~+0.8 MB over baseline (vs ~+4.2 MB for Boa); frot goes
  ~3.5 → ~4.3 MB, comfortably inside the 5–15 MB VISION envelope.
- **The C precedent.** frot chose rustls to avoid C TLS; this is not that.
  The rustls choice avoided *system-library linkage and build fragility*
  (OpenSSL). quickjs-ng is vendored C compiled by the `cc` crate with
  pregenerated bindings — no cmake, no bindgen, no system dep, no dynamic
  linkage; `*-linux-musl` is a CI-tested target, so the static binary holds.
  The `~/AGENTS.md` escalation rule for C dependencies is discharged by this
  document plus the spike's static-build proof. Caveat carried into the
  spike: `set_memory_limit` is a no-op under rquickjs's custom-allocator
  features — stay on the default C allocator.

**Rejected:** Boa (pure-Rust purity play; unbounded execution is
disqualifying — the only credible fallback if the C dependency ever becomes
untenable, hence the engine seam below), `v8`/`mozjs` (~35–50 MB + C++
toolchain — size non-starters), quickjs-rusty (same engine, smaller
ecosystem than rquickjs), Nova/Kiesel/LibJS/Brimstone (immature or unbindable),
Duktape/mujs/JerryScript (ES5/ES2015-era — cannot parse modern bundles).

All engine types stay behind `src/js/engine.rs`; nothing outside `src/js/`
names an rquickjs type, mirroring how no `markup5ever` type leaks past
`dom.rs`. The engine is swappable without touching the shim.

## 2. The mutation exception

`ARCHITECTURE.md`: capabilities produce side tables and never mutate the DOM —
"`--js` (which genuinely mutates) will be the deliberate exception that has to
say so." This section says so, with rules that keep the damage contained:

- **The arena stays the one document.** JS never holds a copy of the DOM; it
  holds opaque integer handles (`NodeId`s). Every read and write goes through
  the syscall layer (§3) to the arena. Two representations of one fact would
  drift; there is one representation.
- **Mutation is append + relink.** `Document` gains mutation ops:
  `create_element`, `create_text`, `set_attr` / `remove_attr`, `set_text`,
  `insert_child` (append is insert-at-end), `detach`. The arena `Vec` is
  append-only; `detach` unlinks a node from its parent, the entry stays in the
  arena. **`NodeId`s are stable forever**, so `NodeId`-keyed side tables stay
  parallel and detached subtrees are simply never reached by `walk`.
- **A generation counter** on `Document` increments on every mutation. Styles
  and Layout computed mid-execution (§8) are cached per generation and
  invalidated by the next mutation. After settle, the pipeline computes them
  once more on the final generation, as today.
- **Mutation ends at settle.** Views still receive `&Document` — immutable.
  The mutable window is exactly the JS phase.

## 3. Binding architecture — narrow syscall table, JS prelude

Two layers, one narrow interface:

- **Rust "DOM syscalls"** — a small host-function table (~20 ops), each a
  function over `(&mut Document, ...)` or the side tables: node queries
  (kind/tag/attr/text/parent/children), the §2 mutations, fragment parsing
  (`innerHTML` runs the html5ever *fragment* parser through the same absorb
  path), selector matching (reusing `css::selector` — the CSS subset engine is
  the single source of selector semantics for both the cascade and
  `querySelector`), geometry (§8), subfetch (§6), the static environment facts
  §7 derives from (the UA string, the final URL, the viewport width, and the
  refused-navigation count), and console logging. This table is the **entire**
  Rust↔JS surface.
- **A bundled JS prelude** (`src/js/prelude/*.js`, embedded via
  `include_str!`, evaluated before any page script) implements the web-facing
  API on top of the syscalls: `Node`/`Element`/`Document` prototypes,
  `querySelector(All)`, `getElementById`, `innerHTML`/`textContent`,
  `addEventListener`/`dispatchEvent`, timers, `fetch`/`XMLHttpRequest`,
  storage, `navigator`/`location`/`matchMedia`. Shim breadth grows in JS
  without widening the Rust interface.

Consequences of the repo's hard rules:

- **≤300 lines applies to prelude files too** — hence a `prelude/` directory
  of concatenated modules, not one giant file.
- **Coverage:** `cargo llvm-cov` cannot see JS lines. The syscall layer gets
  100% Rust coverage as usual; the prelude is exercised by the golden fixture
  suite (subtask 9), which is the test regime VISION principle 7 demands
  anyway.
- `querySelector` accepts exactly the selector grammar `css::selparse`
  supports; an unsupported selector **throws** (a counted error, §10) rather
  than silently matching nothing.

## 4. What runs, and when

Parsing already happened (html5ever ran to completion); frot never executes
during parse. So:

1. **Script discovery:** all `<script>` elements in document order. Classic
   scripts (no `type`, or a JS MIME) and `type="module"` run; `nomodule` and
   non-JS types are skipped. External `src` is fetched under the §6 policy and
   executed — a classic script as a classic script, a `type="module"` script as
   a real ES module. **Module `import` resolution is built (bl-1b98)**: rquickjs's
   module loader (`Runtime::set_loader`/`Resolver`/`Loader`, gated behind the
   crate's `loader` feature — the sole reason the `relative-path` dep rides in,
   approved 2026-07-10; §1 decision log) resolves specifiers and loads source
   through the *same* §6 subfetch cache. A `type="module"` script — inline (its
   body is the module source, imports resolving against the page URL) or external
   (imports resolving against its own fetched URL) — is evaluated as a module:
   relative/absolute-URL specifiers resolve and load through subfetch; a **bare**
   specifier has no import map, so it is unresolvable exactly as in a browser
   without one and fails as a counted §10 error, sibling scripts continuing. A
   failed/refused/non-2xx module fetch is likewise one counted error. Module
   evaluation is async by spec — top-level `await` and dynamic `import()` settle
   through the §5 microtask drain / settle loop under the one wall-clock deadline.
2. **Order:** source order, one queue. `defer` semantics *are* "after parse,
   in source order", and `async`'s any-order license makes source order a
   legal schedule — so one rule covers all three script modes, no scheduler.
   A fetch failure for a `src` skips that script (counted), like a failed
   stylesheet.
3. Scripts inserted by other scripts (`appendChild` of a `<script>`) join the
   end of the queue — that is how bundlers chain-load.
4. After the queue drains: `DOMContentLoaded`, then `load`, then the settle
   loop (§5). `document.readyState` tracks this: `'loading'` while the queue
   runs (deferred scripts included), `'interactive'` immediately before
   `DOMContentLoaded`, `'complete'` before `load` — each transition dispatching
   `readystatechange` on `document`, so readyState-polling init unblocks. Re-entry (rule 3) is a *script-phase* affordance: once the queue
   drains the phase is over, so a `<script>` inserted later by a lifecycle
   handler or a timer callback is not executed — one linear pass, no feedback
   from the event loop into the script queue.
5. **`document.write` throws** (counted). Post-parse `write` implies reopening
   the document — semantics frot will not fake. Ad-tech long-tail is a
   non-goal.
6. **`<noscript>` flips.** Under `--js` its content must not render: the
   cascade treats `noscript` as UA-implicit `display:none` (author-overridable)
   when the js capability ran — the single-source-of-truth home for display
   semantics, and browser-accurate (scripting hides `<noscript>`).
   **Resolved (4.10): the flip is the semantic home and stays; the views'
   structural suppression is an equivalent fast path, not collapsed into it.**
   The flip is *not* observationally inert: it is exactly what a page script
   reading `getComputedStyle(noscriptEl).display` observes — `"none"` when
   `--js` ran, versus `"inline"` (noscript is not a block tag) without it. Its
   live consumer is the geometry path (`src/js/geometry.rs` computes styles with
   `js = true`, since geometry only exists inside a JS session, §8). Separately,
   the structural views suppress `<noscript>` **unconditionally, regardless of
   `--js`** — `text`/`ax` via `SKIP_TAGS`, `layout`/`bboxes` via
   `NON_RENDERED_TAGS` — which is frot's denoising policy (noscript fallback is
   "enable JS" boilerplate, noise in a structural impression). That policy
   *agrees with* the cascade flip when JS ran, so it is an equivalent fast path
   there. It was considered and deliberately **not** replaced by having the views
   consult the cascade: that would make `<noscript>` render *without* `--js` (a
   regression from the denoising default a real no-JS browser wouldn't share) and
   force `text`/`ax` to consult a `Styles` table they don't build without
   `--css` — added mechanism and behavior risk for zero gain. Views that surface
   `<noscript>` structurally today (`links`, `forms`, raw `dom`) list its
   contents regardless of the cascade, by the same policy-vs-semantics split.

## 5. Bounded execution — the event loop

Statelessness and the sub-second target make the loop's *termination* the
design center. Two clocks, three limits:

- **Virtual clock.** Timers schedule at virtual timestamps; the loop always
  pops the earliest-due task and *jumps* the virtual clock to it. `Date.now()`
  / `performance.now()` read the virtual clock (seeded from real time at
  process start). One horizon rule terminates every timer pattern:
  **`VIRTUAL_HORIZON_MS = 10_000`** — a task due past the horizon is dropped.
  `setTimeout(f, 30_000)` never fires ("no timers past load"); `setInterval`
  pollers and `requestAnimationFrame` chains (rAF = 16 ms virtual timer)
  self-terminate at the horizon instead of needing per-API caps. Sub-millisecond
  timer delays clamp to 1 ms so every fire advances the virtual clock — a
  `0`-delay poller (`setInterval(f, 0)`, or a self-rescheduling `setTimeout`)
  therefore self-terminates at the horizon rather than spinning out the
  wall-clock budget.
- **Wall-clock budget: `EXEC_BUDGET_MS = 1_000`**, enforced by the engine's
  interrupt handler; covers execution *and* §6 subfetch time in one deadline.
  Hitting it stops the loop and marks the run unsettled (§10).
- **Memory cap: `JS_MEM_LIMIT = 64 MiB`** engine heap, engine-enforced.

Constants, not flags — same severability posture as the 1280px viewport
(`docs/design/layout.md` §4). Microtasks (the engine job queue) drain after
every task, host-driven. An unhandled exception aborts *that script/task* and
is counted; the loop continues — browser semantics, and a page that throws
after rendering is still a good impression. **Settled** = queue empty and
nothing due before the horizon, **within the budget** — the run driver also
reads the deadline once at conclusion (bl-c7e9): the loop can conclude
*because* the deadline expired (the §6 seam refuses network dispatch past it,
emptying the remaining work) while the interrupt — which fires only between
JS instructions — never happened to trip, and that run must report
`settled: false` deterministically, not by interrupt-timing luck.

## 6. Network policy — once-then-frozen

JS-visible network is `fetch` + `XMLHttpRequest` (both wrap one syscall; XHR
is trivially "sync" since the whole loop is single-threaded and blocking):

- **GET only.** Anything else rejects (fetch — a rejected promise) / throws
  (XHR `send`). frot reads the web; it does not submit to it (the frottage
  rule — no mutating requests back to the origin). Refusals and transport
  failures surface through the *same unified channel as every other §5 error*:
  a `fetch` rejection counts when unhandled (§10 rejection tracking), an XHR
  `send` throw counts when uncaught — a page that gracefully `.catch()`es is
  not penalised. (As built: the once-frozen cache is `src/js/subfetch.rs`; the
  `__frot_subfetch` syscall and the `fetch`/`XMLHttpRequest` prelude ride it,
  and so does the external-`<script src>` runner.)
- **Once-then-frozen.** Each absolute URL is fetched at most once per call and
  its response cached for the call's lifetime. Deterministic within the call,
  nothing persists past it.
- **Same rules as stylesheet subfetches:** resolved against the final page
  URL, `-H` headers ride only same-origin (scheme+host+port), redirects
  followed, 16 MiB cap, remote→local blocked (`file:` targets from an http(s)
  page are refused). `file://` pages may fetch remote resources, as with CSS.
- **Bounds — one per resource, none per request count (bl-c7e9).** Work is
  bounded by *time*: subfetch network time spends the §5 wall-clock budget —
  one deadline, not two — and the deadline is enforced **at this seam**: the
  cache consults the engine's armed window (the same `base`/deadline pair the
  interrupt handler reads — one authoritative clock) before dispatching any
  network request, and refuses once it has passed. The seam check exists
  because the interrupt can only fire between JS instructions: a module graph
  loading through the §4.1 loader is a chain of blocking host fetches with no
  JS in between, which would otherwise outrun the budget unchecked. Overshoot
  is bounded to the one in-flight request. Memory is bounded by *bytes*:
  **`SUBFETCH_BYTES = 64 MiB`** of response body pooled across the call — the
  frozen cache is the network-side analogue of the engine heap and takes the
  same allowance as `JS_MEM_LIMIT` — on top of transport's per-response
  16 MiB cap (`fetch::MAX_BODY_BYTES`, shared with stylesheets and the
  document fetch). Both refusals surface through the unified §5/§10 channel
  (counted; named under `--js-errors`).

  There is deliberately **no request-count cap**. `SUBFETCH_MAX = 16` was
  deleted (bl-c7e9, field trial 2026-07-19): a count is a proxy for no real
  resource — it prices a 200-byte chunk and a 16 MiB bundle identically — and
  modern bundlers code-split into 30–100 chunks, so the proxy tripped on
  essentially every modern app (linear.app, vercel.com, figma.com,
  angular.dev) while time and memory stood idle (golden fixtures settle in
  ~9–16 ms of the 1 s budget). Worse, it corrupted the settle signal: a run
  whose chunk loads were *refused* reported `settled: true` — false
  quiescence. With the cap gone, `settled` has exactly one meaning (§5
  quiescence within the time budget) and each resource has exactly one bound:
  time — `EXEC_BUDGET_MS`; engine heap — `JS_MEM_LIMIT`; fetched bytes —
  `SUBFETCH_BYTES`. A page that genuinely needs more than the budget allows
  now dies honestly of time (`settled: false`), not of an arbitrary count
  masquerading as completion.
- **Per-destination request metadata (landed, `bl-20ec`).** *(Supersedes the
  earlier "subfetches stay honest-minimal" deferral: full per-`Dest` subresource
  fidelity is now cheap because it derives from one place.)* One derivation —
  `fetch::request::derive_headers` — builds the protocol-correct ordered header
  set for **every** request from `(Intent, target, initiator, caller -H)` against
  the pinned Firefox 140.12.0esr persona (`identity.md` §4), replacing the old
  `DOCUMENT_HEADERS` const and the ad-hoc `fetch::fetch` subfetch policy. A
  navigation carries `Accept: text/html,…`, `Upgrade-Insecure-Requests`,
  `Sec-Fetch-Dest: document` / `Mode: navigate` / `Site: none` / `User: ?1`,
  `Priority: u=0, i`; each subresource carries its own honest set instead — a
  stylesheet `Sec-Fetch-Dest: style` / `Mode: no-cors`, a classic script
  `script` / `no-cors`, a module `script` / `cors`, a `fetch`/XHR `empty` /
  `cors`, each with its own `Accept` and `Priority`, and a `Referer` /
  `Sec-Fetch-Site` computed from URL facts (`none` / `same-origin` / `same-site`
  / `cross-site`) under Firefox's `strict-origin-when-cross-origin` policy. The
  rule the old bullet stated still holds — a subresource is *not* a navigation,
  so it never carries navigate-mode `Sec-Fetch` — but the fix is now to send the
  *right* per-destination metadata, not the minimal one. Credentials and custom
  `-H` never leak cross-origin; the caller `-H` is the final same-origin
  override. The exact ordered wire set is pinned for all five intents on both h1
  and h2 by `fetch::request::recorder`.
- Absent-by-design channels: `WebSocket`/`EventSource` are undefined (feature
  detection falls through); `navigator.sendBeacon` returns `false` — where the
  platform spec offers a legal denial, prefer it over an exception.

## 7. Environment shims — no persistence

- `localStorage` / `sessionStorage`: real `Storage` semantics, in-memory,
  born empty, discarded at exit. Pages that gate on its *existence* work;
  nothing survives the process (VISION: no persistence).
- `document.cookie`: reads/writes the one per-invocation cookie jar the transport
  owns (`bl-6dad`, `identity.md` §9), born empty, discarded at exit — not a
  separate string. HttpOnly cookies never enter JS; a JS write feeds a later
  same-origin GET (OQ-3, resolved).
- `indexedDB`: absent. Apps that require it fail into the §10 outcome story.
- `navigator` / `location`: static facts frot already has (the UA string it
  sends, the final URL, `webdriver: false` — truthful: no remote control). `location` *assignment* is navigation — a counted
  no-op; frot takes an impression of one document, it does not browse.
  **Superseded in part by `docs/design/identity.md` §4/§8 (bl-3972):** the
  `navigator` facts stop being literals in `env.js` and become derivations of
  the one `BrowserProfile`, delivered through a syscall the way the UA already
  is. That closes measured incoherences this file's original list carried —
  `languages: ['en-US']` where Firefox reports `['en-US','en']` (and where the
  `Accept-Language` header disagreed), `doNotTrack: null` where Firefox reports
  `'unspecified'`, and missing `oscpu`/`vendorSub`/`buildID`/`pdfViewerEnabled`.
  Screen/viewport geometry does **not** move: it stays derived from
  `layout.rs`'s 1280×720 constant, because two viewport constants would be the
  duplication the profile exists to prevent.
- `history`: in-memory, born fresh (`state: null`, `length: 1`,
  `scrollRestoration: 'auto'`), discarded at exit. SPA routers read
  `history.state` on first render; absent, they throw. `pushState`/
  `replaceState` set `.state` (the only fact read back) — `pushState` also
  bumps `length`; `go`/`back`/`forward` are no-ops. They do **not** mutate
  `location`: it stays the honest fetched URL (location is a static fact),
  because frot takes one impression and routers pick their initial route from
  that location. A page that genuinely needs `location` to reflect `pushState`
  is a separate concern.
- `URL` / `URLSearchParams`: WHATWG subset routers reach for
  (`new URL(location.href)`, `url.searchParams`). Parsing is delegated to the
  Rust `url` crate through the `__frot_url_parse(spec, base)` syscall — the
  **single source of URL semantics**, shared with `location` via one `decompose`
  helper; there is no second URL parser in JS. `new URL` **throws** a
  `TypeError` on an invalid URL (unlike `location`, a fact that empties);
  base resolution rides `Url::join`. `URLSearchParams` is built in JS over the
  parsed `search` string; editing a URL's `searchParams` re-serializes back into
  its `search`/`href` (string surgery on the authoritative href, not a re-parse).
- Viewport size is a fixed **1280×720** (`layout.rs`
  `VIEWPORT_WIDTH`/`VIEWPORT_HEIGHT`; a constant, not a flag). `window.inner*`
  and `window.outer*` report it (inner == outer — no browser chrome); pages that
  gate desktop chrome on `window.innerWidth` (mdbook does) get the honest width
  instead of `undefined`/NaN → the mobile branch.
- `matchMedia`: delegates to the CSS engine's media-query evaluator
  (`src/css/media.rs`, via the `__frot_media_matches` syscall) — the same
  authority `@media` blocks cascade through, so CSS and JS can never disagree.
  Width/height queries in `px`/`em`/`rem` (`em`/`rem` at a **16px** root
  font-size, so `80em == 1280px`) evaluate against the fixed viewport;
  `screen`/`all` match, `print` does not; `and`/comma/`not`/`only` are
  understood; anything unknown matches never. `getContext()` on canvas returns
  `null` (spec-legal). `Worker`, `WebAssembly`, `serviceWorker`: absent.

## 8. Geometry reads — the third layout trigger

VISION names JS as the third layout trigger. `offsetWidth`/`offsetHeight`/
`offsetTop`/`offsetLeft`/`getBoundingClientRect`/`getComputedStyle` route
through two narrow geometry syscalls — a box read (`__frot_rect`, one `[x,y,w,h]`
the prelude shapes into every `DOMRect`/`offset*`) and a computed-style read
(`__frot_computed_style`) — both served from the per-generation cache:

- On demand it computes `Styles` + `Layout` for the **current generation** —
  authored CSS under `--css`, `css::compute_bare` without it (the same rule
  `bboxes` already established) — and caches until the next mutation (§2).
- `getComputedStyle` exposes only what the cascade actually computes
  (`display`, `visibility`, the flex reorder keys); unknown properties return
  `""`. No pretense of a full computed-style set.
- `clientWidth`/`clientHeight` equal `offsetWidth`/`offsetHeight`: frot's model
  is borderless and scrollbar-less (`layout.md` §6 parses no border/padding), so
  content box == border box. The one exception is the `documentElement`, which
  reports the **viewport** (1280×720) as browsers do — the primary way a page
  reads viewport size.
- Geometry answers inherit the layout approximation contract wholesale
  (`docs/design/layout.md` §6): structural estimates, not pixel truth.

## 9. Pipeline ordering

```
fetch ─► parse ─► JS (mutates, §2–§8) ─► needs::detect ─► css ─► layout ─► view
```

Two moves against today's `run.rs`:

- **JS runs before the cascade** — the cascade and layout must see the final
  DOM, and JS-inserted `<style>`/`<link>` sheets get collected naturally
  because sheet collection reads the post-JS document. Mid-execution style
  reads are served per-generation (§8), so ordering stays coherent.
- **`needs::detect` moves after JS when `--js` is on.** The detector's
  question is "is the impression *under this recipe* empty" — the recipe now
  includes JS. Same detector, same invariant, run at the right time.

## 10. Honest signals

- **`needs: ["js"]` survives `--js`, unchanged.** VISION: "Sites that exceed
  the shim get needs-js, same as before. The boundary moves; it doesn't
  disappear." The SPA-shell heuristic re-runs on the post-JS document: if the
  app rendered, body text is non-empty and no signal fires; if the shim was
  insufficient, the impression is still a shell and `needs-js` now means
  "needs more JS than frot can give — use a real browser." **Outcome-based,
  not exception-based:** scripts throwing is normal web weather; an empty
  impression is the honest trigger. Zero new detector mechanism.
- **Additive envelope block** (fields only added, never renamed):

  ```json
  "js": { "scripts": 14, "errors": 2, "settled": true }
  ```

  `scripts` = scripts executed, `errors` = counted failures (§4–§7 throws,
  unhandled promise rejections, refused/failed subfetches — which surface as
  those same rejections/throws, §6 — and unhandled errors surfaced via
  `reportError`/`window.onerror`/a dispatched window `'error'` event that nothing
  suppresses, the channel frameworks like React ≥16 use to *catch* a render
  crash rather than throw, so a dead app does not read `errors: 0`), `settled`
  = the §5 loop reached quiescence within budget. Unhandled-rejection counting is wired at the engine
  seam via quickjs's host rejection tracker (a running net that a late `.catch`
  un-counts), read once after the settle loop. This is the honesty channel for
  *partial*
  execution that outcome detection can't see — a budget-killed run that still
  rendered something must not look complete. Emitted only when `--js` is on.
- **Opt-in error messages behind `--js-errors`** (bl-3356). `errors` answers
  *how many*; `--js-errors` adds *what they said*, a bounded array on the `js`
  block — present only with the flag, `errors` always present and unchanged:

  ```json
  "js": { "scripts": 14, "errors": 2, "settled": true,
          "messages": [ { "kind": "throw",  "text": "TypeError: x is not a function" },
                        { "kind": "report", "text": "Minified React error #418" } ] }
  ```

  The flag **requires `--js`** — bare, it is a usage error (exit 2, no envelope),
  mirroring the `-H`/`file://` gating. Capture is unconditional but bounded to
  `MESSAGES_MAX` (**32**, a constant, not a flag — like `SUBFETCH_BYTES`): a noisier
  page keeps climbing `errors` while the detail array stops at the first 32. Three
  `kind`s **carry a message** (message in hand at the seam, cheap):
  - `throw` — a script/module top-level exception (`EvalError::Exception`, `js.rs`
    `tally`), the message quickjs surfaced.
  - `report` — `reportError` / `window.onerror` / a dispatched window `'error'`
    the shim reported (the bl-249c channel: a React ≥16 render crash is *caught*
    and reported, never thrown, so this is what makes a dead app's messages
    visible, not just counted). Carried through `__frot_report_error(text)`.
  - `subfetch` — a failed/non-2xx external `<script src>`, captured by its spec so
    the operator learns *which* bundle went dark.

  Three classes stay **count-only** (still in `errors`, no message): unhandled
  **rejections** (the engine net-counts via quickjs's tracker; a late `.catch`
  un-counts, so attaching a stable message is invasive), refused **navigations**
  (a `Denials` no-op carries no per-event detail beyond a constant), and
  **timer / lifecycle-listener** throws (`loop.js` `fire`/`__frot_next_timer`
  catch and return counts only; reaching their messages is invasive). This keeps
  the capture set to the seams where a meaningful message is already in hand.
- Envelope `status` is unaffected by script errors; only the needs detector
  and the existing error taxonomy set non-`ok` status.
- **A host-side coercion failure is not a script error (as built, 4.9).** The
  run driver reads each task's completion value as a string for its protocol,
  but a page script's completion is not its output, and some values won't
  `ToString` (a bare `Object.create(null)`; a framework's public proxy — Vue 3's
  `mount()` returns one). Such a coercion throw is the host's, not the page's, so
  the engine seam degrades it to the empty string rather than counting it — the
  script ran clean. (Only the page's own throws and unhandled rejections count.)

## 11. Non-goals

- **No interaction.** No synthetic clicks, no form submission, no input
  events, no scrolling. The lifecycle events (`DOMContentLoaded`, `load`) are
  the only events the host ever dispatches. "Click X if Y" is a different
  tool (VISION non-goals).
- **No navigation.** `location` writes, `history` pushes, meta-refresh:
  no-ops (counted where a page could observe the lie). One URL, one document,
  one impression.
- **No persistence** across calls, of any kind (§7).
- **No iframes.** Frame documents are not fetched or executed;
  `contentWindow`/`contentDocument` are `null`.
- **No workers, no WASM, no media, no canvas rendering** (§7).
- **Coherent identity, yes; defeating defences, no** (`docs/design/identity.md`
  §10 — this boundary **moved twice**: `bl-0356` moved it, then Mark's
  masquerade ruling moved it again on 2026-07-20, and this bullet is the latest
  line). The rule now separates two axes: *matching what the server requires is
  in scope, including by masquerading a capability frot does not physically have;
  what stays refused is a different axis — executing or solving a challenge, and
  evasion loops. A signal frot cannot yet produce is a filed gap, not a
  permanent non-goal.*
  > **Superseded 2026-07-20.** This bullet previously read *"frot may present a
  > coherent identity for a client that genuinely has the capabilities it claims;
  > it may not fabricate evidence of capabilities it does not have,"* with
  > canvas/WebGL/audio/font/media-device fabrication listed **"out,
  > permanently"**. Mark superseded that (*"I don't mind masquerading
  > capabilities … file backlogs for the gaps"*). Kept here as superseded, not
  > deleted.
  - **In:** the JS-visible facts — `navigator` branding, `language`/`languages`,
    `platform`/`oscpu`, `buildID`, the plugin/mimeType shims — derived from the
    one `BrowserProfile` (identity.md §4, §8), consistent with the UA and TLS
    the transport actually sent. `webdriver: false` stays: it is *truthful*, not
    a costume.
  - **In scope, unbuilt — a filed gap:** masquerading high-entropy rendering
    signals — canvas, WebGL, audio, font metrics, media-device enumeration — to
    match what a fingerprinter requires. `getContext()` keeps returning `null`
    **today** because the simulation is unbuilt (§7), *not* because it is
    forbidden; the gap is owned by `bl-bd4e` and follow-ups (identity.md §10).
    The bar is **coherence**: a masqueraded value must be a deterministic,
    profile-derived simulation, because an incoherent one is a louder tell than
    absence — an engineering requirement, not an honesty one. The **VISION
    principle-5 question is resolved** (Mark, 2026-07-20 — no conflict: principle
    5 governs honesty of the delivered impression, not the wire persona; see
    identity.md §10 and VISION.md principle 5).
  - **Out — a different axis, unchanged:** executing or solving a challenge —
    CAPTCHA, JS proof-of-work, behavioural interstitials — and evasion loops of
    any kind (no UA rotation, no retry-until-allowed). This is the `bl-abe5` hard
    boundary and the 2026-07-20 ruling did not touch it.
  - **Unchanged — the honest outcome.** If a challenge script defeats the shim,
    the outcome is an honest `needs-js` or `error.kind: http.403`; and a
    *declared* challenge (`needs.md` §3, e.g. `Retry-After` on a 200) is
    reported as `needs: ["human"]` **before its scripts are ever executed**.
    Detection is refusal to pretend, not a step toward evasion.

## 12. Decomposition (proposed subtasks for bl-b3c5)

Proposals only — the dispatcher files these; this doc does not run `bl
create`. Each inherits bl-b3c5's ordering. The spike is first because it is
the falsifiable check on §1.

1. **Engine spike** — embed rquickjs behind `src/js/engine.rs` (eval,
   interrupt hook, memory cap, host-driven job queue); prove the static/musl
   build; measure the binary delta; smoke-run a minified React bundle. A
   falsifying result reopens §1 toward Boa. *Dep: none.*
2. **DOM mutability** — §2 arena mutation ops + generation counter + fragment
   absorb path. Pure `dom.rs` work, no JS. *Dep: none.*
3. **Syscall layer + prelude core** — handle-based Node/Element/Document,
   `innerHTML`, `querySelector` via `css::selector`, console. *Dep: 1, 2.*
4. **Script execution + wiring** — script discovery/order (§4), `--js` flag
   accepted (drop the Phase-2.5 usage error), pipeline reorder (§9), envelope
   `js` block + post-JS needs detection (§10), `noscript` flip. *Dep: 3.*
5. **Event loop** — virtual clock, horizon, timers/rAF, microtask drain,
   lifecycle events, wall-clock interrupt + settled flag (§5). *Dep: 4.*
6. **Subfetch: fetch/XHR + modules** — once-then-frozen cache, GET-only,
   same-origin header rules, the count cap as first shipped (`SUBFETCH_MAX`,
   later deleted for the §6 deadline/byte bounds — bl-c7e9); external
   `<script src>` execution.
   *As built (bl-00ba):* classic-script pages landed; unhandled-rejection
   counting wired (§10). **ES module `import` resolution landed as its follow-up
   (bl-1b98)** — rquickjs's `loader` feature (the approved `relative-path` dep)
   drives a `Resolver`/`Loader` over the same §6 cache; `type="module"` scripts
   evaluate as real modules (§4.1). *Dep: 5.*
7. **Geometry syscalls** — per-generation Styles/Layout cache,
   `getBoundingClientRect`/`offset*`/`getComputedStyle` subset (§8). *Dep: 3;
   layout exists since Phase 3.*
8. **Environment breadth** — storage, cookie string, `navigator`/`location`,
   `matchMedia`, spec-legal denials (§7). *Dep: 3.*
9. **Golden fixture suite** — pinned real-world pages (a React SPA shell, a
   Vue app, a jQuery page, a beyond-the-shim page that must yield `needs-js`)
   driven end-to-end; this is where prelude coverage lives (§3). *Dep: 4–8.*
10. **Docs fidelity pass** — README "Where it stands", ARCHITECTURE pipeline/
    module map/decision log, VISION Phase-4 reconciliation. *Dep: 9.*

## 13. Open questions (with recommended defaults)

- **OQ-1 — budget constants** (`EXEC_BUDGET_MS = 1000`, `VIRTUAL_HORIZON_MS =
  10_000`, `JS_MEM_LIMIT = 64 MiB`, `SUBFETCH_MAX = 16`): recommended as
  written; the spike and fixture suite tune them. Constants stay constants —
  no flags without a filed need. **Resolved (4.9, bl-6358): unchanged, measured
  with wide margin.** The golden fixtures settle end-to-end (React 17 UMD +
  ReactDOM.render, Vue 3 global build, jQuery 3.7.1) with `errors: 0,
  settled: true`; measured whole-run wall time is **~9–16 ms release / ~26–31 ms
  debug** — a >30× margin under `EXEC_BUDGET_MS`. None trips `VIRTUAL_HORIZON_MS`,
  `JS_MEM_LIMIT`, or `SUBFETCH_MAX` (the shells load 2–3 classic bundles, well
  under 16). No constant needed changing.
  **Amended (bl-c7e9, field trial 2026-07-19): the fixture margin didn't
  survive contact with the field.** Live code-split apps (30–100 chunks)
  tripped `SUBFETCH_MAX` on essentially every run while the other budgets
  stood idle. The resolution is not a bigger number — no count is principled,
  because a request count measures no resource — but deleting the count cap
  and bounding each real resource once: the §5 deadline now enforced at the
  subfetch seam (work), and a pooled `SUBFETCH_BYTES = 64 MiB` (memory,
  sized to `JS_MEM_LIMIT` — the cache is the network-side heap). See §6.
- **OQ-2 — deterministic `Math.random` / frozen clock for reproducibility?**
  Recommended: **no.** Network content already varies across calls;
  reproducibility-within-a-call is what the virtual clock and frozen fetches
  give. Seeding `Math.random` is speculative hardening.
- **OQ-3 — seed `document.cookie` from the response's `Set-Cookie`?**
  **Resolved: yes, and superseded by `identity.md` §9 (`bl-6dad`, landed).** The
  "born-empty, frot is not a cookie jar" recommendation was reversed once the
  transport gained one per-invocation jar: an *isolated* `document.cookie` string
  is a browser contradiction (a server that `Set-Cookie`s on the document GET
  never sees it back on subresources). `document.cookie` now reads/writes the same
  jar the transport owns (`src/js/syscall/cookie.rs` → `src/fetch/cookie.rs`) at
  the final document URL — reads omit HttpOnly/non-applicable cookies, writes can
  never mint HttpOnly, and the jar is still born-empty and dies with the process
  (state within a call, not a session model).
- **OQ-4 — ES modules in the first cut?** Recommended: **yes** (subtask 6) —
  the 2026 web is module-first; skipping them guts coverage. If the spike
  shows module loading is disproportionately heavy, demote to a follow-up and
  let classic-script pages land first. **Resolved: demoted in 4.6 (bl-00ba),
  then landed in bl-1b98.** The implementation surfaced the cost: rquickjs's
  `Runtime::set_loader`/`Resolver`/`Loader` (the only way `import` resolution
  rides the subfetch path) live behind the crate's `loader` feature, which adds
  the `relative-path` dependency — tripping the escape hatch's "new deps" clause
  and `~/AGENTS.md`'s no-new-deps rule. So classic-script pages (inline +
  external `<script src>` + `fetch`/XHR) landed first (bl-00ba), and module
  `import` resolution was filed as bl-1b98. **The `relative-path` dep was
  approved (Mark, 2026-07-10) and bl-1b98 landed it:** the `loader` feature adds
  exactly `relative-path` and nothing else; a `Resolver`/`Loader` pair over the
  §6 subfetch cache now resolves and links module graphs, so `type="module"`
  scripts evaluate as real modules (§4.1) with bare specifiers spec-legally
  unresolvable (no import map) and failed fetches counted (§10).
- **OQ-5 — raise `EXEC_BUDGET_MS` for the heaviest apps?** (bl-f859, field
  trial 2026-07-19: excalidraw.com, open.spotify.com, discord.com hit the 1 s
  deadline before quiescence; post-bl-c7e9, chunk-heavy sites like linear.app
  additionally convert their refused-chunk errors into `settled: false`, as
  that ball predicted.) **Resolved: no — the ceiling is accepted; the budget
  stands at 1 s.** Four reasons, in force order:
  1. With the count cap gone (bl-c7e9), the deadline is the *single* work
     limiter by design. Its value is the product promise — VISION's success
     criterion is "sub-second once JS is in play" — so it is a contract, not a
     fidelity tunable; raising it trades the promise away for the long tail
     VISION explicitly refuses to optimize for.
  2. The virtual clock (§5) never waits on timers, so 1 s of budget is 1 s of
     *genuine* compute and network. A page that cannot settle in that is doing
     more real work than a fast structural-impression tool should underwrite —
     exactly Phase 5+'s "accept the ceiling and defer to a real browser."
  3. The trial's unsettled sites are canvas/WebGL/audio apps that exceed the
     *shim*, not the clock — no budget renders them, and two of three already
     emit `needs-js` correctly. `settled: false` is the honesty contract
     working, not a defect.
  4. The chunk-heavy conversion (refused-chunks → `settled: false`) is a
     *latency* shape, not a budget-size shape: the §6 cache fetches
     sequentially, so N chunks cost N round-trips and doubling the budget buys
     linear chunk count for doubled wall time. If a filed need ever demands
     more inside the same promise, the principled lever is subfetch
     concurrency (parallel dispatch under the same deadline and byte pool),
     not more time — noted, deliberately not built.
