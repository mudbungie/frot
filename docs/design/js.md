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
   loop (§5). Re-entry (rule 3) is a *script-phase* affordance: once the queue
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
nothing due before the horizon.

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
- **Caps:** `SUBFETCH_MAX = 16` requests per call (counted when exceeded);
  network time spends the §5 wall-clock budget — one deadline, not two.
- Absent-by-design channels: `WebSocket`/`EventSource` are undefined (feature
  detection falls through); `navigator.sendBeacon` returns `false` — where the
  platform spec offers a legal denial, prefer it over an exception.

## 7. Environment shims — no persistence

- `localStorage` / `sessionStorage`: real `Storage` semantics, in-memory,
  born empty, discarded at exit. Pages that gate on its *existence* work;
  nothing survives the process (VISION: no persistence).
- `document.cookie`: in-memory string, born empty (OQ-3).
- `indexedDB`: absent. Apps that require it fail into the §10 outcome story.
- `navigator` / `location`: static facts frot already has (the UA string it
  sends, the final URL). `location` *assignment* is navigation — a counted
  no-op; frot takes an impression of one document, it does not browse.
- `matchMedia`: evaluated against the fixed 1280px viewport for width queries;
  everything else matches never. `getContext()` on canvas returns `null`
  (spec-legal). `Worker`, `WebAssembly`, `serviceWorker`: absent.

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
  unhandled promise rejections, and refused/failed subfetches — which surface
  as those same rejections/throws, §6), `settled` = the §5 loop reached
  quiescence within budget. Unhandled-rejection counting is wired at the engine
  seam via quickjs's host rejection tracker (a running net that a late `.catch`
  un-counts), read once after the settle loop. This is the honesty channel for
  *partial*
  execution that outcome detection can't see — a budget-killed run that still
  rendered something must not look complete. Emitted only when `--js` is on.
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
- **Not a stealth runtime.** Fingerprinting resistance and anti-bot evasion
  stay non-goals; if a challenge script defeats the shim, the outcome is an
  honest `needs-js` or `error.kind: http.403`.

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
   same-origin header rules, `SUBFETCH_MAX`; external `<script src>` execution.
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
- **OQ-2 — deterministic `Math.random` / frozen clock for reproducibility?**
  Recommended: **no.** Network content already varies across calls;
  reproducibility-within-a-call is what the virtual clock and frozen fetches
  give. Seeding `Math.random` is speculative hardening.
- **OQ-3 — seed `document.cookie` from the response's `Set-Cookie`?**
  Recommended: **no** until a pinned fixture demands it; born-empty is
  simpler and honest (frot is not a cookie jar).
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
