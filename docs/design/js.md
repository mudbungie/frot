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
  `insert_child` (positioned by a *reference sibling*, as `insertBefore` is —
  never by an index, which the relink's own unlink would invalidate, bl-ae88;
  `None` appends), `detach`. The arena `Vec` is
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
  refused-navigation count), the executing `<script>` (§4.1
  `document.currentScript`), and console logging. This table is the **entire**
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
- **A reflected attribute IS an accessor over the attribute** (bl-d313, found
  live on `mdn.github.io/js-examples/module-examples/`: `divWrapper.id =
  'myCanvas'`, the plainest line in the basic-modules example, threw). Getter-
  only reflections are not a smaller shim, they are a *wrong* one: ES modules
  are strict, so the assignment is a TypeError that kills the module before it
  renders, and since bl-0679 a classic script instead loses the write in
  silence. So the whole DOM-string family — `id`, `className`, `rel`, `type` —
  comes from **one maker** (`reflectString` in `elem.js`, shared with `elem2.js`
  as the non-enumerable `__frot_reflect`, the seam `__frot_brand`/`__frot_iface`
  already use), and `href`/`src` from its resolving sibling `reflectUrl`. The
  getter reads the attribute (`''` when absent, as WebIDL requires); the setter
  writes it through the very syscall `setAttribute` uses, so `el.id = x` and
  `el.setAttribute('id', x)` are indistinguishable and there is still exactly
  one home for the fact: `getElementById`, the selector engine, the views'
  serialization and the generation counter that invalidates the §8 style/layout
  cache all see the write at once. Nothing is indexed by id — `getElementById`
  is a selector query over the one arena — so duplicate ids resolve in document
  order for free. The remaining getter-only-where-browsers-write surfaces
  (`outerHTML`, the `[PutForwards]` pair `style`/`classList`, `document.body`)
  are audited and filed as **bl-273b**, not folded in here.
- **DOM collections are the spec-named interfaces** (bl-e5c3, found live: a
  bundle ran `NodeList.prototype.forEach = Array.prototype.forEach` at init
  and died on an undefined `NodeList`). One invariant, one maker (`dom.js`):
  every collection a query returns is an instance of the interface the web
  spec names for it — `childNodes`/`querySelectorAll`/`getElementsByName` a
  `NodeList`; `children`/`getElementsByTagName`/`getElementsByClassName` an
  `HTMLCollection` — never a bare `Array`. Both prototypes are WebIDL-shaped
  the way Gecko exposes them (`new` throws Illegal constructor, `length` is a
  prototype accessor, the iteration methods ARE the `Array.prototype` ones),
  and instances are snapshots of wrapped nodes: like every wrapper, they cache
  no arena state. The env-contract fixture pins the whole surface.
- **Two ES iterator helpers are frot's, not the engine's** (bl-5249, found
  live on astro.build: `qsa('.integration-tab').values().find(…)`).
  quickjs-ng's C `Iterator.prototype.find` and `.filter` never release a value
  their predicate *rejects*, so every skipped object stays referenced and
  engine teardown aborts the whole process — `JS_FreeRuntime: Assertion
  list_empty(&rt->gc_obj_list) failed`, SIGABRT, no envelope, in violation of
  the one-envelope contract. A leaked reference cannot be freed from outside
  the engine and the assertion is not ours to silence (turning it off only
  trades the abort for a panic reclaiming the arena, `session.rs`). Upstream
  fixed `find` in quickjs-ng 0.16.0, but rquickjs ships 0.15.1 and a git pin
  does not survive `cargo publish` (the registry substitutes the released
  crate, restoring the crash in the *published* frot), so
  `prelude/iterator.js` composes those two out of the sibling helpers that are
  free of these leaks: `find` is a `some` that keeps the value it stopped on,
  `filter` a `flatMap` yielding one value or none. Laziness, `next` caching,
  iterator closing (including at suspended start and on a non-callable
  predicate, ES2026 §27.1.3.3.4–.5), and the `Iterator Helper` shape stay the
  engine's; the two are captured *before* the swap and applied reflectively, so
  a page overriding `some`/`flatMap` redirects only its own calls, and they are
  concise methods — non-constructible like the built-ins — branded native like
  the rest of the prelude. Every other helper is untouched, and the file goes
  away when rquickjs ships a quickjs-ng with the `filter` free too. The pin is
  a **process-boundary** test (`tests/binary.rs`), because in-process the run
  completes and only teardown fails.
- **Token lists are spec-named DOMTokenList instances** (bl-3a36, found live:
  the `vite:build-import-analysis` modulepreload polyfill that every Vite
  production build inlines at module top level runs
  `link.relList.supports("modulepreload")` and, absent `relList`, falls through
  to a then-absent `MutationObserver` (genuine since `bl-07ab`, §7) — the
  module rejects, a
  count-only §10 error, so `--js-errors` showed `errors: 1, messages: []`).
  Same maker discipline as the collections above, one file (`tokenlist.js`):
  `classList` and `relList` are both live views over their backing attribute
  (no cached state), WebIDL-shaped as Gecko exposes them (Illegal constructor,
  prototype `length`/`value` accessors, live indexed access, the iteration
  methods ARE the `Array.prototype` ones). `supports()` answers from the
  pinned Firefox 140.12.0esr supported-token tables (Gecko
  `HTMLLinkElement.cpp` `SUPPORTED_REL_VALUES_BASE` plus the default-on
  `manifest`/`modulepreload` prefs; `Element.cpp` `sAnchorAndFormRelValues`
  for a/area/form), and throws the coherent TypeError for `class`, which
  defines no supported tokens. `relList` exists only where Firefox has it
  (link/a/area/form) — elsewhere it stays undefined so feature detection is
  honest.
- **Element/document breadth from the modern-bundle field trial** (bl-3a36 —
  each item below was absent and killed a deployed React app inside library
  code, so the fix is the general surface, not a per-page shim):
  `document.createElementNS` and the `*AttributeNS` trio (React/Vue create
  every SVG element through them; the arena stores local names — the parser
  already flattens foreign content the same way — so the namespace argument is
  honestly dropped, and attribute reads normalize to the arena's lowercase
  names so SVG camelCase round-trips); `closest`/`getRootNode` (no shadow DOM,
  §11: connected roots to the document, detached to the subtree top);
  live `nextSibling`/`previousSibling`; the ParentNode/ChildNode mixin
  `append`/`prepend`/`remove`; the form-control interface prototypes
  (`HTMLInputElement`/`HTMLTextAreaElement`/`HTMLSelectElement`) carrying the
  same reflection descriptors as the nodes (Radix-style libraries call
  `getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set`); and a
  **constructible** `new DocumentFragment()` returning the real staging
  fragment with `ownerDocument`/`insertBefore`/`removeChild` (React portals
  into fresh fragments and reaches the document through the container) — an
  **EventTarget** like every node (bl-e81b, §7: React listens on every portal
  container, and a missing `addEventListener` mid-render corrupts React's
  unwind); and
  `rel`/`href` reflection (`bl-07ab`: MutationObserver's flagship consumer —
  the Vite modulepreload polyfill — reads `link.rel`/`link.href` off observed
  records; `href` reflects resolved against the document URL, as Firefox's
  getter does). The
  env-contract fixture pins all of it.
- **Comments are real arena nodes** (bl-79db, found live: the Vue 3.5 TodoMVC
  deployment mounted, settled with `errors: 0`, and rendered nothing). The
  invariant: every node a script can hold is a genuine arena node whose
  insertion rides the §2 syscalls — no host-side fakes, no silently dropped
  inserts. `createComment` was a fake `{nodeType: 8, _id: -1}` object that
  `appendChild`/`insertBefore` no-opped on, and frameworks use comments as
  **anchors**: Vue's RouterView at start-location mounts a comment
  placeholder, then the router-ready patch computes its container as
  `parentNode(placeholder)` — undefined on a fake, so the patch threw inside
  Vue's scheduler, whose production error path is `console.error` (recorded,
  never counted — see §10). One arena kind the parser already stored
  (`NodeKind::Comment`), one creation syscall (`__frot_create_comment`), and
  the whole anchor contract (parent/sibling navigation, positioned
  replacement) works for free; comment data never leaks into rendered text
  because `text_content` collects only text nodes.

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
   - **Language mode is the script kind's, never the host's (`bl-0679`).** A
     classic script runs **sloppy** unless its own source opts in with a `'use
     strict'` directive, which the parser honours as always; an ES module is
     strict by definition. rquickjs's `EvalOptions::default` forces `strict`, and
     inheriting that default silently changed the language every page script was
     written in: found live on svelte.dev and hn.svelte.dev, whose untyped inline
     bootstrap assigns a generated `__sveltekit_*` global with no `var` and so
     threw `ReferenceError` under forced strictness — one throw, nothing
     rendered, in a page Chrome renders. Sloppy is not a laxity setting: implicit
     globals, a plain call's `this` being the global, silent writes to read-only
     properties, `delete` of an identifier, legacy octal literals, and Annex B
     block-function hoisting are all *load-bearing* in real bundles. So frot's
     own JS (the prelude, the §5 event-loop drivers) evaluates through
     `Engine::eval_armed` — strict, the mode the prelude is written in — and page
     input through `eval_script` (sloppy) or `eval_module` (strict); every flag
     is stated at the call, none inherited. The rule is executable:
     `tests/fixtures/js/script-mode.html` + `js::tests::scriptmode`.
   - **`document.currentScript` is a host query, not a JS field (`bl-a19d`).**
     While a classic script runs — inline or external — it is that `<script>`
     element; between scripts, and throughout module evaluation, it is `null`.
     Found live across at least six Next/Turbopack sites (nextjs.org 36 errors,
     tailwindcss.com 12, nodejs.org, shadcn, Mantine, Vercel): a fetched chunk
     derives its own URL from `document.currentScript` to register itself, so
     with the property absent every chunk threw and the page stayed a shell.
     The executing script is a fact the *host* has — `run_script_queue` is
     holding the `NodeId` — so it lives in one host cell the syscall reads back,
     rather than being mirrored into JS: the host sets it around one evaluation
     and restores the prior value after, which makes staleness impossible by
     construction. A throw, a budget trip, and a nested evaluation all restore,
     because the restore is not something the script could skip. `script.src`
     (like `href`) reflects **resolved**, as Firefox's getter does, since the
     chunk's next move is `new URL(currentScript.src)`.
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
design center. Two clocks, four limits:

- **One coherent *observable* clock, with a virtual offset (`bl-e707`; scope
  narrowed by `bl-8dc0`).** There is exactly one monotonic **wall** clock per
  invocation — the injectable `js::engine::Clock` — and it is what the *page*
  observes and what §6 network dispatch is deadlined on. It is no longer what
  the compute budget is spent in; that is CPU time, accounted separately below,
  because wall time prices frot's own work in the host's scheduling luck. The
  observable browser clock is the wall clock's *real elapsed* (host/CPU/network time,
  read in JS through the `__frot_now` syscall) **plus** a virtual offset the
  timer loop adds. `performance.timeOrigin` (wall-clock ms at session start),
  `performance.now()`, and `Date.now()` all derive from that **one origin**:
  `Date.now() - timeOrigin == performance.now()`. So observable time advances
  with actual work — a busy script or a blocking subfetch moves it (real elapsed
  time, whether or not it was frot's CPU that spent it — which is precisely why
  it is the wrong meter for the compute budget and the right one for a page
  probing its host), and a synchronous/subfetch probe never sees
  perpetual zero — while a virtual timer *jump* only bumps the offset so the
  reading reaches **at least** the timer's due time. Values never go backward
  (a monotone ratchet) and are floored to the profile's timer precision —
  `BrowserProfile::timer_precision_us` (identity.md §9: Firefox's 1 ms
  `reduceTimerPrecision` clamp), the single literal, never a second hardcode.
  Timers still fire immediately in host wall time, in due order; only the
  *reading* jumps, so no artificial wait and no jitter is ever introduced.
  One horizon rule terminates every timer pattern:
  **`VIRTUAL_HORIZON_MS = 10_000`** — a task due past the horizon is dropped.
  `setTimeout(f, 30_000)` never fires ("no timers past load"); `setInterval`
  pollers and `requestAnimationFrame` chains (rAF = 16 ms virtual timer)
  self-terminate at the horizon instead of needing per-API caps. Sub-millisecond
  timer delays clamp to 1 ms so every fire advances the observable clock — a
  `0`-delay poller (`setInterval(f, 0)`, or a self-rescheduling `setTimeout`)
  therefore self-terminates at the horizon rather than spinning out the compute
  budget.
  - **Resource timing (`bl-e707`).** `performance.getEntries*` exposes a
    `PerformanceResourceTiming` for **real measured requests only** — each §6
    subfetch, bracketed on the one clock for its true `startTime`/`duration`.
    Phases frot does not measure (DNS/TCP/TLS) stay spec-legal `0`, never
    fabricated; no navigation entry is synthesised for the pre-JS document fetch
    (unmeasured here) — omission over invented phases.
- **Compute budget: `EXEC_CPU_MS = 1_000` of *CPU* time** (`bl-8dc0`, superseding
  the wall-clock `EXEC_BUDGET_MS = 1_000`), enforced by the engine's interrupt
  handler and re-read between macrotasks. It prices frot's own work — script
  compile, interpretation, DOM syscalls, GC — in the unit frot actually spends.
  Exhausting it stops the loop and reports `settled: false, stopped: "budget"`
  (§10): *the page wants more compute than frot gives*, and that is the same
  verdict on an idle laptop and on a saturated CI box.
- **Network budget: `NET_BUDGET_MS = 1_000` of *wire* time** (`bl-79dc`,
  superseding the wall *deadline* armed once per run), metered at the §6 seam:
  every dispatch is bracketed on the observable clock and its elapsed charged to
  the budget, and the seam refuses the next dispatch once the budget is spent.
  Network wait is elapsed time and is *not* frot's work: charging it to CPU would
  make it free (a blocked socket burns no cycles). But a wall *window* over the
  whole run charged the reverse mistake back again — it was spent by frot's own
  compute and by the host's scheduling, so a page heavy enough to burn a second
  of compute had its next subfetch refused and reported `stopped: "network"` with
  no network having been slow at all. A budget only real dispatch can spend
  cannot be spent by anything else, so the §10 name is true whenever it is
  emitted, and `VISION.md`'s arithmetic is literal: **~1 s of CPU plus ~1 s of
  network wall, not one shared second**. A refused dispatch reports
  `settled: false, stopped: "network"` (§10).
- **Memory cap: `JS_MEM_LIMIT = 64 MiB`** engine heap, engine-enforced.

All three bound *page scripts and the event loop*; the one-time API install is
exempt from every one of them (`Engine::setup`: it disarms the compute window —
the network budget needs no exemption, since install dispatches nothing to charge
it — and lifts the heap cap for the duration, restoring the constructed cap after — so a
dialed-down budget cannot turn setup into a spurious failure, `bl-5ac3`, and the
prelude is never *parsed* on a starved heap). **The heap half of that is not a
convenience (`bl-c385`): quickjs's parser is not allocation-failure-safe.**
`js_parse_block` ignores a failed `push_scope` and calls `pop_scope` anyway,
which reads `fd->scopes[garbage]` — a **segfault**, not a catchable OOM —
whenever a failing allocation lands on a function's *fifth* scope (the first one
past the inline `def_scope_array[4]`, i.e. a nested block inside a function).
The prelude is frot's own fixed-size program, not page input: starving it buys
nothing and risks crashing the process with no envelope. Before the exemption
the starve sweep passed only by the byte-layout luck of where the OOM fell, so
any prelude edit could re-trip it. What is still starved, and must be, is the
*binding* fold (`js::syscall::bind`, swept in `syscall::starve_tests`): it
allocates but parses nothing, and its failure is a clean `Err` the install turns
into one loud panic rather than a half-bound table. **Residual, upstream:** a
*page* script that exhausts the 64 MiB cap mid-parse can still hit the same
quickjs defect; the fix belongs in quickjs-ng, and nothing frot does from the
host side can catch it.

> **The unit was the bug, not the size (`bl-8dc0`, 2026-07-22).** One wall-clock
> deadline meant a *correct* run on a busy host emitted a *different envelope*:
> Adduce measured `vue_app_renders_and_clears_needs_js` and
> `react_shell_needs_js_without_and_renders_with` flipping to `settled: false`,
> and `react19_createroot_app_renders_from_empty_root` flipping `status` from
> `ok` to `needs`, with zero errors and every script run — the page simply did
> not finish in 1 s of *host* time. An envelope kind that depends on how busy the
> machine is contradicts VISION ("fast, deterministic") and the whole reason
> `persona_gold` exists.
>
> Two candidate host-independent units were measured on this repo's golden
> bundles (16 cores; "loaded" = 48 spinners, 4× oversubscription):
>
> - **Interpreter steps** (counting quickjs's `JS_INTERRUPT_COUNTER_INIT = 10000`
>   interrupt ticks) — **rejected.** Perfectly load-independent, but blind: the
>   fixtures burn only **2–4 ticks** for **18–23 ms** of work, because quickjs
>   polls interrupts at JS back-edges and calls *only* — never during compile
>   and never inside a host syscall, which is where nearly all of frot's JS-phase
>   time goes. A step budget would leave `for(…){ el.innerHTML += big }` unbounded.
> - **CPU time** (`CLOCK_THREAD_CPUTIME_ID`) — **adopted.** It covers every class
>   of frot's own work, including the two steps miss, and it is what contention
>   does not inflate. Measured (release, per fixture idle → loaded):
>   react17 **23.1 → 97.7 ms wall** but **22.6 → 22.9 ms CPU**; vue3 23.6 → 179.1
>   wall / 23.4 → 57.9 CPU; jquery 18.6 → 192.6 wall / 18.1 → 43.6 CPU; react19
>   22.0 → 251.9 wall / 21.5 → 58.8 CPU. Under `llvm-cov` — the actual `make cov`
>   gate — idle 48–78 ms CPU, loaded 50–85 ms CPU against 76–292 ms wall.
>   **Contention inflates wall time 4–11×; it inflates CPU time 1.0–2.7×.** The
>   worst-case margin under the gate goes from **3.4×** (wall) to **12×** (CPU),
>   and the residual is cache/memory-bandwidth contention, not scheduling.
>
> What this trades away, stated plainly: **frot's *runtime* is no longer bounded
> in wall time — its *output* is bounded in host-independent work.** Worst case is
> now ≈1 s CPU + 1 s network wall, and under starvation the wall clock stretches
> with the host. That is the right trade for a tool whose product is a
> deterministic impression: a slow answer is recoverable, a different one is not.
> A residual host-*speed* dependence remains (a 4×-slower core gets 4× less work
> done per CPU-second) and is accepted — no unit removes it without the blindness
> that disqualified steps.

Constants, not flags — same severability posture as the 1280px viewport
(`docs/design/layout.md` §4). Microtasks (the engine job queue) drain after
every task, host-driven. An unhandled exception aborts *that script/task* and
is counted; the loop continues — browser semantics, and a page that throws
after rendering is still a good impression. **Settled** = queue empty and
nothing due before the horizon, with neither bound tripped.

The second half of that — "the §6 seam emptied the remaining work by refusing
network, so the loop concluded falsely quiescent" — was detected (bl-c7e9) by
re-reading the deadline once at conclusion, because the interrupt fires only
between JS instructions and may never have tripped. **`bl-8dc0` replaces that
time comparison with the fact it was proxying for:** the driver asks the §6
cache whether it *refused* a dispatch. Same guarantee, one less clock read, and
it fixes the case the proxy got wrong — a run that drained every task and merely
happened to cross the deadline on its way out reported `settled: false` while
being genuinely quiescent.

**Testability is half-inherited; the network half needed the seam after all
(`bl-c81a`, superseding this section's earlier "no new seam is owed").** With
the compute budget in CPU time, a golden asserting `settled: true` asserts that
the fixture's **~20 ms of CPU** fits in 1000 ms of CPU, which no amount of host
*load* changes (measured above: 1.0–2.7× inflation, and 12× margin under
`llvm-cov` at 4× oversubscription). This section originally called the residual
`NET_BUDGET_MS` coupling "a bound the suite *should* be honest about rather
than mock away", and predicted that a mock server made it harmless. **That was
wrong, and four independent agents hit it:** the wall window is armed once and
spans the *whole* run, so on the heaviest golden (`vite-react-tailwind`, 578 KB
of StrictMode React 19 + router + framer) it is the fixture's own **compute**,
stretched by parallel build load, that spends the *network* budget — the run
reports `stopped: "network"` with no network involved. Over an in-process
`mockito` server there is no real wait to be honest about; the only thing the
wall clock was metering was the machine's business, which is exactly the input
`bl-8dc0` removed from the compute side.

So the two §5 budgets are now one `js::Bounds` value taken **through the call
signature** from `run::deliver::run_bounded` down to `js::run`. That is not the
hidden test-only state `AGENTS.md` forbids — it is the opposite, and the same
posture `run::deliver` already takes with the SIGPIPE disposition (`SIG_DFL` in
production, `SIG_IGN` from the test). Production passes `Bounds::shipping()`
(both host clocks) at one call site; the golden suite passes the shipping
*compute* window on a real `Clock::cpu` and the shipping *network* window on a
frozen `Clock::manual`. The bound is handed to the test, not disabled:
`run/golden_bounds_tests.rs` spends that same budget and still gets
`settled: false, stopped: "network"`, and a spinning script under a dialled-down
CPU window still gets `stopped: "budget"` — the negative controls that make
every golden's `settled: true` mean something.

**And the root cause was the bound, not the fixture (`bl-79dc`).** The paragraph
above diagnosed it correctly and then fixed only the symptom: if the *only* thing
the wall clock was metering was the machine's business, then a wall window is the
wrong instrument for a network bound in production too, not merely in the golden
suite. A page heavy enough to spend that window computing had its next subfetch
refused and reported `stopped: "network"` on an origin that was never slow — the
same misdescription, one layer up, with no test watching it once the goldens
froze their clock. `NET_BUDGET_MS` is therefore metered on **dispatch elapsed**
alone (above, and §6): compute cannot spend it, host load cannot spend it, and
`stopped: "network"` means the wire really did take a second. The injected
`Bounds` seam stays, because it is still what lets the golden suite meter its
in-process fixtures on a clock it owns and what lets `golden_bounds_tests` and
`js/tests/netbudget.rs` drive both bounds by hand — including a "slow origin"
that is a `mockito` handler advancing the run's own clock, so the proof of a
timing bug neither sleeps nor spins.

That "~20 ms fits in 1000 ms" is no longer taken on trust (`bl-18df`): each of
the four golden bundles runs through `run_guarded`, which measures the run on a
`Clock::cpu` and fails — naming the fixture — if it costs `EXEC_CPU_MS / 2` or
more. The fraction is computed from the constant, so there is no second number
to drift, and the assertion is exactly as load-sensitive as `settled` is.

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
  > **Superseded in part 2026-07-22 (`bl-017a`), proposal awaiting sign-off.**
  > The bullet above is the shipped behaviour and stays accurate until the
  > challenge design lands. Under `docs/design/challenge.md` §3 the rule becomes:
  > **GET only, except a POST the page's own script initiated through
  > `fetch`/XHR** — capped at a new `REQ_BODY_BYTES` request-body constant,
  > excluded from the once-then-frozen cache (a POST is not idempotent and the
  > invocation cache is keyed by URL alone), and available on `Intent::FetchXhr`
  > only. The **navigation stays GET permanently** — that is the frottage rule,
  > and it is unmoved. What "frot does not submit to it" now means precisely:
  > frot *originates* nothing; a page script that POSTs is the browser acting,
  > and frot cannot construct such a request from the host side by
  > construction (`challenge.md` §2).
- **Once-then-frozen.** Each absolute URL is fetched at most once per call and
  its response cached for the call's lifetime. Deterministic within the call,
  nothing persists past it.
- **Same rules as stylesheet subfetches:** resolved against the final page
  URL, `-H` headers ride only same-origin (scheme+host+port), redirects
  followed, 16 MiB cap, remote→local blocked (`file:` targets from an http(s)
  page are refused). `file://` pages may fetch remote resources, as with CSS.
- **Bounds — one per resource, none per request count (bl-c7e9; unit corrected
  `bl-8dc0`, scope corrected `bl-79dc`).** Network is bounded by *elapsed* time,
  and by elapsed time **on the wire only**: each dispatch — a serial `get`, and
  the concurrent warm wave alike, served or failed, since a timeout is the
  slowest origin there is — is bracketed on the observable clock and charged to
  the §5 `NET_BUDGET_MS` budget, which is this seam's **own** bound rather than a
  share of the compute budget (§5: a blocked socket burns no CPU, so the two
  resources cannot be priced in one unit). It is enforced **at this seam**: the
  cache consults what is left of the budget before dispatching, and refuses once
  it is spent — and that refusal, as a recorded fact, is what clears `settled`
  and sets `stopped: "network"` (§5/§10), replacing the old re-read of a deadline
  at conclusion. Metering the wire rather than the run is what makes that name
  honest: the earlier armed-once wall window spanned frot's compute too, so a
  heavy page's own execution spent the *network* budget. The seam check exists
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
  quiescence within the run's bounds) and each resource has exactly one bound
  **in its own unit** (`bl-8dc0` split the first): compute — `EXEC_CPU_MS`
  (CPU time); network — `NET_BUDGET_MS` (wire time); engine heap —
  `JS_MEM_LIMIT`; fetched bytes — `SUBFETCH_BYTES`. A page that genuinely needs
  more than a budget allows now dies honestly of the resource it actually
  exhausted (`settled: false`, plus the `stopped` name, §10), not of an
  arbitrary count masquerading as completion.
- **Concurrent initial-script warm (landed, `bl-08f6`).** *(Supersedes the
  "fetches sequentially… deliberately not built" note below — the principled
  lever it named is now built.)* Before the source-ordered script queue drains,
  the run **preload-scans** the parsed document for its initial external
  `<script src>` (classic + module) and warms the frozen cache for all of them
  **concurrently** — `Subfetch::warm` → the one `fetch::fetch_many` primitive the
  CSS gather also rides, at most `POOL_PER_HOST` in flight, folded into the cache
  in one single-threaded pass. It is *parallel dispatch under the same deadline
  and byte pool*: each request is timed out at the §5 deadline's remainder and
  the wave is capped at the `SUBFETCH_BYTES` remainder, so neither bound is
  weakened — only the *latency* is, N serial round trips collapsing toward one
  h2-multiplexed wave. Execution order is untouched: the queue still runs each
  script in document order (a warmed URL is a plain cache hit, served even past
  the deadline since no network is left), so a slow first script and a fast
  second still evaluate first-then-second. Only *statically present* scripts are
  warmed — a script a running script inserts is discovered and fetched when it
  appears, never speculatively prefetched. Transitive module imports stay a
  serial loader chain (they are unknown until their importer is parsed); the
  preload-scanner set is the top-level externals, exactly as in a browser.
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
- **`data:` URLs are decoded, not fetched (landed, `bl-91bf`).** A `data:` URL
  *is* its own response, so it never reaches the transport: `src/js/subfetch/
  data.rs` decodes it (percent-decode, then forgiving-base64 when the media type
  ends `;base64`, then the shared `decode_body` charset path — a `;charset=`
  parameter means exactly what a `Content-Type` header means) and returns a
  frozen 200 whose `Content-Type` is the declared media type. Every §6 consumer
  gets it from the one seam: an external `<script src="data:…">` runs in its
  queue position, and `fetch`/XHR/`import` read the same bytes. **The network
  deadline does not apply** — there is no dispatch to bound, and a decode is not
  a refusal, so an inline script never makes a run `stopped: "network"` and never
  appears in resource timings. **The byte pool does** — the decoded body charges
  `SUBFETCH_BYTES` exactly as a fetched body would, so a page cannot inline its
  way past the bound. It is deliberately *not* cached: the cache key would be the
  payload itself, so freezing would store the bytes twice to make a pure decode
  repeatable, and `warm` skips data URLs because they have no round trip to hide.
  An undecodable payload is one counted §10 error, as a failed fetch is.
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
- `indexedDB`: **present, but stateless (`bl-8dde`, LANDED).** The §10 coherence
  bar makes absence a louder tell than a costume (Firefox has it), so `indexedDB`
  is a Firefox-shaped `IDBFactory` (branded native, the `IDB*` interface zoo with
  `[object …]` tags and "Illegal constructor" throws; `IDBVersionChangeEvent` an
  `Event` subtype) and feature detection passes. frot persists nothing (§7), so
  there is no backing store: `open()`/`deleteDatabase()` return a real,
  permanently-`pending` `IDBOpenDBRequest` whose `onsuccess`/`onupgradeneeded`/
  `onerror` **never fire** — the declared residual (`identity.md` §11). That
  silence is *less* detectable than an error, because a real fresh `open()`
  succeeds, so an error callback would contradict the persona. `databases()`
  honestly resolves to `[]`; `cmp()` is a real synchronous key comparison. Apps
  that gate on *existence* work; apps that *await* an open fail into the §10
  outcome story.
- `navigator.permissions` / `Notification`: **present, but no grant (`bl-1548`,
  LANDED).** Firefox exposes both; the §10 coherence bar makes absence a louder
  tell than a costume, so `navigator.permissions` is a branded `Permissions`
  whose `query({name})` returns a `Promise<PermissionStatus>` (branded, `[object
  PermissionStatus]`, `state`/`name`/`onchange`, EventTarget), and
  `window.Notification` is a branded, constructable interface (static
  `permission`/`maxActions`/`requestPermission`). Every value is fixed / never
  random: on a FRESH profile nothing is granted or denied, so `query()` resolves
  `state: 'prompt'` for every recognised name (the Firefox 140esr `PermissionName`
  enum; an unrecognised name **rejects** with the coherent `TypeError`),
  `Notification.permission` is `'default'`, and `requestPermission()` resolves an
  honest `'default'`. frot raises no prompt and shows no notification — a
  constructed `Notification` fires no event; `onchange`/`onclick` **never fire** —
  the declared residual (`identity.md` §11). This is exactly what a real,
  un-prompted page sees, so it is coherent, not a wrong value: asking for a grant
  frot cannot make would be the louder tell. Pure JS over `brand.js`, no syscall.
- `navigator` / `location`: `location` *assignment* is navigation — a counted
  no-op; frot takes an impression of one document, it does not browse.
  **The `navigator` identity surface derives from the profile (`bl-3972` LANDED,
  `docs/design/identity.md` §4/§8).** Every fact — `userAgent`/`appVersion`/
  `appName`/`appCodeName`/`product`/`productSub`/`vendor`/`vendorSub`/`platform`/
  `oscpu`/`language`/`languages`/`doNotTrack`/`buildID`/`hardwareConcurrency`/
  `maxTouchPoints`/`cookieEnabled`/`onLine`/`pdfViewerEnabled` and the
  `plugins`/`mimeTypes` PDF-viewer arrays — flows through **one** syscall,
  `__frot_env_profile()`, which returns a JSON payload the prelude parses once
  (`src/js/prelude/navigator.js`). No identity literal lives in the prelude (I1).
  `userAgent`/`appVersion` and `language`/`languages` come from the *effective*
  UA and `Accept-Language` (the `-H`-overridable strings on `Env`), the **same
  source** as the HTTP headers, so wire and JS cannot disagree — closing the
  measured incoherences (`languages: ['en-US']` vs Firefox's `['en-US','en']`,
  `doNotTrack: null` vs `'unspecified'`, the old UA/JS-UA gap, missing
  `oscpu`/`vendorSub`/`buildID`/`pdfViewerEnabled`). `webdriver: false` stays —
  *truthful*, not a costume. The shim is **Firefox-shaped**: `navigator` is a
  branded `Navigator` instance (`[object Navigator]`, `instanceof Navigator`) with
  every fact an enumerable accessor on `Navigator.prototype` (not an own data
  prop), matching Firefox's descriptor shape — so brand/prototype/descriptor
  probes pass, not just values.
- **Focus** (`bl-3a36`, found live: TodoMVC's deployed React shell dies in the
  commit phase on `autoFocus && stateNode.focus()` — React catches the throw
  via `captureCommitPhaseError` and, with no error boundary, unmounts the whole
  root: a settled run, one `report` message "not a function", an empty shell
  still reading needs-js. The vendored react19 golden carries the identical
  commit code but its app never sets `autoFocus`, which is exactly why the
  golden passed while every deployed bundle that autofocuses died). One
  document-level fact — `document.activeElement`, initially the body — that
  `focus()`/`blur()` move, firing `blur` on the loser then `focus` on the
  gainer through the ordinary listener registry. Focusability follows Firefox
  (form controls except `input[type=hidden]`, unless disabled; a/area with
  `href`; anything with `tabindex`/`contenteditable`; all else a silent
  no-op). **Page-driven only**: §11's "the lifecycle events are the only events
  the host ever dispatches" holds — frot never focuses anything by itself
  (native `autofocus` processing at load is deliberately not performed), so a
  focus event exists only because a page script called `focus()`, exactly like
  `dispatchEvent`.
- **EventTarget + abort surface (`bl-e81b`, found live: the Vite/React-Router
  template died "You cannot render a `<Router>` inside another `<Router>`" —
  an error about frot, not the app; the deployed bundle renders exactly one
  Router).** The measured chain: Radix-style libraries park closed popover
  content in `createPortal(children, new DocumentFragment())`, React listens
  on every portal container (`listenToAllSupportedEvents`), and the staging
  fragment's missing `addEventListener` threw mid-render; React's unwind then
  misaligned its shared context cursor stack, and the retried render read
  another provider's value out of react-router's LocationContext — the
  nested-Router invariant was the corruption's symptom, three causes
  downstream of the divergence. The surface, Firefox 140esr's base contract:
  the staging fragment carries Node's own EventTarget trio (one loop.js
  listener registry, keyed by a unique negative `_id` no arena node can
  carry); **`EventTarget`** is a global, constructable interface whose
  `instanceof` matches by the trio's shape, so every event-bearing surface
  answers true without a class hierarchy; and
  **`AbortController`/`AbortSignal`** (`prelude/abort.js`:
  `abort`/`reason`/`onabort`/`throwIfAborted`, statics
  `abort`/`timeout`/`any`, timeout on the §5 virtual clock) exist with real
  state-and-event semantics. Deliberate residual: no subfetch integration —
  §6's once-then-frozen network has nothing cancellable, so aborting only
  marks the signal, which is exactly the fact these libraries read back. The
  env-contract fixture pins the surface; the `vite-react-tailwind` field
  fixture replays the whole page end-to-end.
- `screen` / `devicePixelRatio` / visibility (`bl-1cb7`, LANDED): a branded
  `Screen` (`screen.width`/`height`/`availWidth`/`availHeight` = the 1280×720
  layout viewport, **not** a second constant, identity.md §8), `colorDepth`/
  `pixelDepth` = 24 and `devicePixelRatio` = 1 from the profile, `screenX`/`Y` and
  the scroll offsets 0, `document.visibilityState: 'visible'` / `hidden: false` /
  `hasFocus(): true` (one foreground impression). Coherent by construction: screen
  == avail == window viewport.
- `Intl` (`bl-ac8d`, LANDED): quickjs-ng ships without `Intl`, whose absence is a
  loud tell, so the prelude provides the low-entropy subset a page reads to detect
  locale/timezone — `Intl.DateTimeFormat().resolvedOptions()` returns `{locale,
  calendar:'gregory', numberingSystem:'latn', timeZone, …}` with `locale` the
  profile locale and `timeZone` pinned `UTC` (§9 determinism; a declared residual,
  §11). Full `NumberFormat`/`Collator`/relative-time formatting is a residual.
- `crypto` (`bl-cf3a`, LANDED): `crypto.getRandomValues`/`randomUUID` from **real
  OS randomness** (`/dev/urandom` via `__frot_random_bytes`), with the browser
  argument/quota/error contract (integer-typed view or `TypeMismatchError`, >65536
  bytes → `QuotaExceededError`) enforced in JS. Not deterministic and not pinned;
  separate calls share no state (OQ-2). `crypto.subtle` stays a residual (§11).
- **Native-code branding** (`bl-3926`, LANDED): frot's web APIs are JS over
  `__frot_*` syscalls, so an un-branded `fetch.toString()` would leak prelude
  source and the name `frot`. **One** `Function.prototype.toString` wrapper backed
  by **one** WeakMap registry (`src/js/prelude/brand.js`) makes every registered
  web API read `function name() { [native code] }`; a non-enumerable
  `__frot_brand`/`__frot_iface` pair is the registry the persona surface and the
  six later capability balls extend — no second wrapper, no per-call-site patch.
  `nativebrand.js` sweeps the whole existing surface last. *(Residual: the raw
  `__frot_*` syscall **names** remain enumerable on `globalThis`; hiding them is a
  separate concern, tracked apart from this branding of `toString`.)*
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
  understood; anything unknown matches never. `getContext('2d')` on canvas is a
  **coherent, deterministic fingerprint masquerade** (`bl-05e6`, identity.md §11):
  a branded `CanvasRenderingContext2D` whose draws fold into a profile-seeded
  digest, so `toDataURL`/`getImageData` are stable across invocations and vary with
  content; `getContext('webgl')`/`'webgl2'` are the **same masquerade** (`bl-f624`,
  §11): branded `WebGL`/`WebGL2RenderingContext` contexts with masked
  `VENDOR`/`RENDERER` (`"Mozilla"`), coherent Mesa/llvmpipe `UNMASKED_*` via
  `WEBGL_debug_renderer_info`, and deterministic `readPixels`/`toDataURL`.
  `Worker`/`SharedWorker`
  are **present as coherent,
  non-executing constructors** (`bl-342a`, §11): feature detection sees a
  Firefox-shaped surface, but no thread is spawned. `WebAssembly`,
  `serviceWorker`: absent.
- **Observers (`bl-07ab`) — the decision lives HERE; the env-contract fixture
  derives from it.** All three were previously pinned absent only in a fixture
  comment — a loud persona tell (Firefox 140esr has all three) and a crash
  class: bundles construct them unguarded at module top level (the Vite
  modulepreload polyfill every Vite production build inlines reaches
  `new MutationObserver` whenever its relList early-return misses), and the
  whole module dies as a count-only §10 error `--js-errors` cannot even name.
  - **`MutationObserver` is GENUINE**, implemented at the mutation-syscall
    seam. Every tree/attribute/text mutation reaches the arena through exactly
    five prelude-visible syscalls (§2/§3: `__frot_insert_child`,
    `__frot_detach`, `__frot_set_attr`, `__frot_remove_attr`,
    `__frot_set_text`), so ONE wrapper set over those five
    (`src/js/prelude/observer.js` captures the raw functions and republishes
    the names wrapped) yields real `MutationRecord`s with **no new Rust
    surface** and no per-API hooks: childList (added/removed NodeLists with
    true siblings; the arena auto-unlinks on insert, so re-parenting is
    honestly a *move* — a removal record then an addition record), attributes
    (`attributeName`, `oldValue` captured before the write), characterData
    (`oldValue`). Subtree matching is a parent-chain walk — O(depth) per
    mutation, never a subtree scan — with observations keyed by `NodeId` and a
    `document` sentinel matched only for connected nodes. Delivery is a
    microtask on the engine job queue (§5), the spec's own timing; a callback
    that itself mutates schedules a fresh round (re-entrancy = a new batch,
    terminated by the page going quiet or the §5 CPU budget); a *throwing*
    callback routes through the §10 `reportError` channel (counted, message
    captured). A page that never constructs an observer pays one guard per
    mutation — measured inside the §5 CPU-guard margin. Granularity residual:
    `innerHTML`/`textContent` replacement emits one record per detach/insert
    the seam actually sees, where a browser coalesces a "replace all" into one
    record — the same mutations, finer sliced.
  - **`IntersectionObserver`/`ResizeObserver` do NOT ride MutationObserver's
    coattails** — presence-but-never-firing (the Worker/indexedDB costume) was
    argued and **rejected** for both. The costume defends where real Firefox
    is also silent (an un-messaged Worker, a pending IDB open on a fresh
    profile); here it is not: a real browser **always delivers an initial
    batch** — IO's first update-intersection pass queues an entry per target
    (the previous-threshold index starts at −1), RO always reports the initial
    size — so a never-firing observer contradicts every real Firefox on the
    API's *first* use. And the blast radius inverts versus absence: absent IO
    makes lazy-load libraries fall back to eager loading (a good impression);
    a present-but-dead IO makes them wait forever — content silently never
    renders, a degraded result that looks complete, exactly what VISION
    principle 5 forbids. So both are implemented with a **genuine initial
    delivery** (`src/js/prelude/viewobserver.js`), computed from the same
    per-generation §8 geometry `getBoundingClientRect` serves (structural
    estimates, layout.md §6): IO entries carry the real target box, `rootBounds`
    = the 1280×720 viewport (or the given `root`'s box, `rootMargin` in px/%),
    and `intersectionRect`/`intersectionRatio`/`isIntersecting` computed from
    them at scroll offset 0 — truthful, because frot genuinely never scrolls
    (§11); RO entries where `contentBoxSize == borderBoxSize` is the §8
    borderless model's own contract, and `devicePixelContentBoxSize` matches at
    the profile's devicePixelRatio 1. Delivery is a task on the existing §5
    timer queue (browsers deliver observer batches from the rendering steps —
    a task, not a microtask; frot's one task source is its timer queue, no
    second scheduler). **The declared residual (identity.md §11): after the
    initial delivery, later DOM-mutation-driven geometry changes produce no
    further entries.** That is the one behavioral gap against an idle real
    browser — whose own post-initial deliveries are driven by scroll, resize,
    and animation, none of which frot ever produces — and closing it would
    mean re-diffing layout per observed target per generation: real §5 CPU
    cost for a trigger frot structurally never fires. `unobserve()` then
    re-`observe()` honestly re-delivers the then-current state; an emptied
    delivery wave never invokes the callback with zero entries.

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
  = the §5 loop reached quiescence within its bounds. Unhandled-rejection counting is wired at the engine
  seam via quickjs's host rejection tracker (a running net that a late `.catch`
  un-counts), read once after the settle loop. This is the honesty channel for
  *partial*
  execution that outcome detection can't see — a budget-killed run that still
  rendered something must not look complete. Emitted only when `--js` is on.
- **`stopped` names *which* bound ended a run (`bl-8dc0`).** Additive, and present
  **only** when `settled` is `false` — a settled run has no bound to name, so the
  field's absence is its own record (the §5/`bl-c7e9` posture: don't store what
  the other field already says):

  ```json
  "js": { "scripts": 9, "errors": 0, "settled": false, "stopped": "budget" }
  ```

  Two values, because §5 now bounds two different resources and they are two
  different facts about the world:
  - `"budget"` — the `EXEC_CPU_MS` compute budget was exhausted. **The page is
    heavier than frot underwrites.** Host-independent: a retry on a quieter box
    returns the same verdict, so the actionable answer is a real browser, not a
    retry.
  - `"network"` — the run spent its whole `NET_BUDGET_MS` of wire time and the §6
    seam refused a further dispatch. **The transport was slow, not the page.**
    A retry may legitimately differ. True by construction since `bl-79dc`: only
    real dispatch can spend that budget, so this name can no longer be reached by
    a page that merely computed for a second.

  This is the answer to "a page that needs JS frot cannot run is not the same
  fact as a page frot ran out of time on." `status: "needs"` stays outcome-based
  (§10 first bullet) and is unchanged; `settled` says whether the impression is
  complete; `stopped` says what truncated it. Three fields, three questions, no
  overlap — and no new flag: the block is already emitted under `--js`.
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
  page keeps climbing `errors` while the detail array stops at the first 32. Each
  message's *text* is bounded too, at `TEXT_MAX` (**200** characters, ellipsis
  marking the cut, `bl-91bf`): a diagnostic names a failure, and a message whose
  text is itself a payload — a `data:` script source, a thrown megastring — must
  not copy the page into the report. Two bounds, one per dimension: how many, and
  how big each. Three
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

  One channel is invisible by the framework's own choice, not frot's (bl-79db,
  measured live): **Vue 3 production builds route caught render/scheduler
  errors to `console.error`** — where React ≥16 uses the counted
  `reportError`/window channel above — so a dead Vue app reads `errors: 0`,
  `messages: []` even under `--js-errors`. frot's console capture records the
  line; it is not counted, because promoting console text to an app failure
  would be a string heuristic over log noise. The honest signal for such a
  page stays the outcome detector: the impression is still a shell, so
  `needs: ["js"]` survives (first bullet).
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
  > **Reaffirmed and load-bearing as of 2026-07-22 (`bl-017a`).** This bullet is
  > **unchanged** by the challenge-boundary movement, and it is now the *whole
  > enforcement* of that boundary's human clause: a browser never produces a
  > click or a `mousemove` unattended — the person does, through it. A
  > behavioural interstitial that wants a mouse-move therefore stays out for
  > exactly the reason it always did. `form.submit()`/`requestSubmit()` remain
  > no-ops even once the transport can carry a verb (`challenge.md` §2, §6).
- **No navigation.** `location` writes, `history` pushes, meta-refresh:
  no-ops (counted where a page could observe the lie). One URL, one document,
  one impression.
  > **Clarified 2026-07-22 (`bl-017a`).** Still true as written: frot never
  > follows a page to a *different* URL. The proposed challenge round trip
  > re-issues **the caller's own URL** after a server-posed challenge minted new
  > state — not a script-chosen navigation. "One URL, one impression" holds; "one
  > request" does not, and never governed redirects either (`challenge.md` §4).
- **No persistence** across calls, of any kind (§7).
- **No iframes.** Frame documents are not fetched or executed;
  `contentWindow`/`contentDocument` are `null`.
- **No canvas *rasterisation*** (`bl-05e6`, §7). `getContext('2d')` is a coherent
  masquerade: a branded `CanvasRenderingContext2D` accepts the drawing API and
  folds each call into a fixed-`canvas_seed`-seeded digest, so `toDataURL()`
  (a well-formed `image/png`) and `getImageData()` are DETERMINISTIC across
  invocations and vary with content — never random per call (the privacy-tool
  tell). **The residual is pixel realism:** the bitmap is a digest expansion, not
  a glyph raster (identity.md §11). No syscall.
- **No WebGL *rendering*** (`bl-f624`, §7). `getContext('webgl')`/`'webgl2')` (and
  `'experimental-webgl'`) return branded `WebGL`/`WebGL2RenderingContext` contexts:
  `getParameter(VENDOR)`/`(RENDERER)` are Firefox's masked `"Mozilla"`, the real GPU
  strings surface only through `WEBGL_debug_renderer_info` and name **Mesa llvmpipe**
  (software — coherent for headless Linux Firefox, never over-claiming hardware),
  and the limit/extension/precision set is one real llvmpipe build's (the `webgl`
  SSOT, `src/fetch/webgl.rs`, via `__frot_env_profile`). `readPixels`/`toDataURL`
  are the same deterministic digest expansion as 2D (`webglpix.js`), stable across
  invocations and content-varying. **The residual is pixel realism** (no GL runs);
  a canvas binds one context type for life, so a cross-type `getContext` is null. No
  syscall.
- **No worker *execution*, no WASM, no media** (§7). The
  `Worker`/`SharedWorker` *constructors* are present as a coherent masquerade
  (`bl-342a`, §7): `typeof Worker === 'function'`, `new Worker(url)` returns a
  Firefox-shaped instance with native `postMessage`/`terminate`/`onmessage`, but
  no thread runs — a page that constructs a worker and awaits a reply **gets
  none**. That non-delivery is the one declared residual (identity.md §11), not a
  bug: presence is the coherence requirement (§10), execution stays out of scope
  (GET-only, bounded, stateless all hold — the shim spawns nothing).
- **No permission grant, no notification prompt** (`bl-1548`, §7). The
  `navigator.permissions` / `Notification` *surfaces* are present as a coherent
  masquerade: `navigator.permissions.query({name})` resolves a Firefox-shaped
  `PermissionStatus`, `typeof Notification === 'function'`. But frot raises no
  prompt and shows no notification, so every `query()` resolves `'prompt'`,
  `Notification.permission` is `'default'`, `requestPermission()` resolves
  `'default'`, and a constructed notification fires no event. That no-grant /
  no-fire is a declared residual (identity.md §11), not a bug: a real un-prompted
  page sees exactly this, so it is coherent — inventing a `'granted'` frot cannot
  back would be the louder tell (§10).
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
  - **Out — a different axis, redrawn 2026-07-22 (`bl-017a`):** *doing what the
    human does* — CAPTCHA and every interactive gate, and any synthesized click,
    input event, or form submission — plus evasion loops of any kind (no UA
    rotation, no retry-until-allowed). The authority is
    `docs/design/challenge.md`; the line is **automatable vs. human-requiring**,
    and its test is **origination**.
    > **Superseded 2026-07-22 (`bl-017a`).** This sub-bullet previously read, and
    > this is the `bl-abe5` hard boundary verbatim: *"**Out — a different axis,
    > unchanged:** executing or solving a challenge — CAPTCHA, JS proof-of-work,
    > behavioural interstitials — and evasion loops of any kind (no UA rotation,
    > no retry-until-allowed). This is the `bl-abe5` hard boundary and the
    > 2026-07-20 ruling did not touch it."* Mark superseded it (*"It's time for
    > that scope creep … 'the content at this url, as a human browser would get
    > it' … okay to do multiple round trips"*). Kept as superseded, not deleted.
    > Note the incoherence the movement fixed: frot **already** executed
    > challenge scripts under `--js` (that is how `bl-bd4e` learned Amazon's
    > probe set); only a pre-parse *header* check stopped it, and it fired on a
    > declaration, not on the work. **Proposal awaiting sign-off — until it lands,
    > the superseded text is the shipped behaviour.**
  - **Unchanged — the honest outcome.** If a challenge script defeats the shim,
    the outcome is an honest `needs-js` or `error.kind: http.403`.
    > **Amended 2026-07-22 (`bl-017a`).** The rest of this bullet previously read:
    > *"and a *declared* challenge (`needs.md` §3, e.g. `Retry-After` on a 200) is
    > reported as `needs: ["human"]` **before its scripts are ever executed**.
    > Detection is refusal to pretend, not a step toward evasion."* Under the new
    > line, a declared challenge with `--js` **off** is `needs:["js"]` (a recipe
    > change genuinely helps); with `--js` on, its script runs, and
    > `needs:["human"]` is reported only **after** the automatable route was tried
    > and failed — where it is finally true (`needs.md` §1/§3, `challenge.md`
    > §5.2). Refusal to pretend is unchanged as the principle; what changed is
    > that asserting "a person is required" before trying was itself the pretence.

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
  and bounding each real resource once — see the further amendment below.
  **Amended again (bl-8dc0, 2026-07-22): the margin was measured in the wrong
  unit.** The ">30× margin" above is *wall* time, which contention inflates
  4–11×, so it was never a 30× margin on a busy host — Adduce watched three
  golden tests flip their envelope with zero errors. `EXEC_BUDGET_MS` is
  therefore split by unit, not resized: `EXEC_CPU_MS = 1_000` (CPU) for compute
  and `NET_BUDGET_MS = 1_000` (wall) for network, keeping both numbers. Re-measured
  in CPU time the fixtures cost **18–23 ms release / 48–78 ms under `llvm-cov`**,
  inflating only 1.0–2.7× under 4× CPU oversubscription — a 12× worst-case margin
  under the gate. §5 carries the full measurement and the rejected alternative
  (interpreter-step counting). The original resolution — no constant grew — still
  holds; the constants are the same numbers in honest units.
  **Amended once more (bl-18df, 2026-07-24): the margin is now an enforced
  invariant, not a remembered measurement.** The failure mode of this entry was
  never a wrong number — it was that a *recorded* margin stops being true and
  nothing notices. Each golden bundle is therefore driven through
  `run_guarded` (`src/run/golden_tests.rs`), which measures the run on a
  `Clock::cpu` and fails, naming the fixture, if it costs **half of
  `EXEC_CPU_MS` or more**. The fraction is computed from the constant, so the
  guard tracks any resize of the budget and no second number exists to drift.
  Half is deliberately loose: re-measured 2026-07-24 on 16 cores the whole-run
  cost is 32–59 ms CPU release and 41–61 ms under `llvm-cov`, rising only to
  74–130 ms under `llvm-cov` at 4× CPU oversubscription — so the gate keeps a
  ≥3.8× worst-case margin while still catching prelude bloat or an accidental
  O(n²) syscall. A wall-clock reading here would reintroduce exactly the
  host-load dependence bl-8dc0 removed.
  The bl-c7e9 mechanics: the §5 deadline is enforced at the
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
     **(`bl-8dc0` made this reason literally true rather than aspirational: the
     compute second is now a CPU second, so "1 s of genuine work" no longer
     silently means "1 s of a host that may have been giving frot a tenth of a
     core." The resolution is unchanged and strengthened — the reason OQ-5 gives
     for refusing a bigger number is exactly the reason the unit had to change.)**
  3. The trial's unsettled sites are canvas/WebGL/audio apps that exceed the
     *shim*, not the clock — no budget renders them, and two of three already
     emit `needs-js` correctly. `settled: false` is the honesty contract
     working, not a defect.
  4. The chunk-heavy conversion (refused-chunks → `settled: false`) is a
     *latency* shape, not a budget-size shape. **Update (`bl-08f6`, landed):**
     the principled lever this reason named — subfetch concurrency (parallel
     dispatch under the same deadline and byte pool) — is now built: the
     initial external scripts are warmed concurrently through `fetch_many`
     (§6), so a wave of chunks costs roughly one h2-multiplexed round trip
     instead of N serial ones. The budget still stands at 1 s; concurrency,
     not more time, is what buys the chunk-heavy case its latency back. (A
     transitive module-import *chain* remains serial — unknown until parsed —
     so a deep import graph can still exhaust the budget honestly.)
