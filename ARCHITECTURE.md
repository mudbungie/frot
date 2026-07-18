# Architecture (as-built)

What the code actually is, as of the 2026-07 arch pass. `VISION.md` says why and where; this says how. When they disagree, the code wins — fix whichever doc is wrong.

## The pipeline

One process, one pass, one envelope:

```
argv ──cli::parse──► Args { url, css, js, out }
url ──fetch::fetch──► FetchResult { final_url, status, headers, body, charset }
body ──dom::Document::parse──► Document (arena)
(doc) ──js::run──► Document', JsInfo             — --js ONLY: mutates in place, then the rest reads Document'
(view, doc) ──needs::detect──► [NeedsKind]     — non-empty short-circuits to a `needs` envelope
(doc, sheets) ──css::compute_with──► Styles    — --css (bboxes: compute_bare even without it)
(doc, styles) ──layout::compute──► Layout      — on-demand: bboxes always, ax under --css; else skipped
(doc, styles, layout?) ──views::* / ax::ax_tree──► JSON payload
payload ──Envelope──► one JSON line on stdout + exit code
```

**`--js` runs first, before needs/CSS/layout/views** (`docs/design/js.md` §9). It is the one capability that *mutates* the DOM rather than producing a side table, so the whole pipeline downstream — including `needs::detect` — must see the post-JS document: the needs question is "is the impression empty *under this recipe*", and the recipe now includes JS. JS-inserted `<style>`/`<link>` sheets are therefore collected naturally, because sheet collection reads `Document'`. Without `--js` the document passes straight through, bit-for-bit the earlier pipeline.

Orchestration lives in `src/run.rs` (~100 lines); everything below it is a pure function of its inputs. There is no global state anywhere — statelessness is enforced by signature. The `--js` phase mutates a `Document` through a bounded, single-threaded window that *closes at settle* — after settle every downstream reader still gets `&Document` (immutable), so the mutation exception does not leak into the pure-function layer.

## Core data structures

**`Document` (`src/dom.rs`)** — html5ever parses into `RcDom`, which is immediately *absorbed* into a flat `Vec<NodeEntry>` arena indexed by `NodeId` (`u32`). The facade exposes `walk` (preorder `Enter`/`Exit` events), `find_by_tag`, `text_content`; no `markup5ever` type leaks past this module, so parser/storage can be swapped without touching views. Tag and attribute names are lowercased at absorption.

**`Styles` (`src/css.rs`)** — a `Vec<ComputedStyle>` parallel to the arena, indexed by the same `NodeId`. Capabilities do not mutate the DOM; they produce side tables that views consult. **This is the extension pattern**: Phase 3 layout output is another `NodeId`-indexed side table (`Layout`, below); `--js` is the deliberate exception (Phase 4) — it *does* mutate, under the contained rules below.

**Document mutation — the `--js` exception (`src/dom/mutate.rs`)** — Phase 4 gives `Document` the append-plus-relink mutation ops JS needs: `create_element`/`create_text`, `set_attr`/`remove_attr`, `set_text`, `insert_child` (append is insert-at-end), `detach`. The arena `Vec` is **append-only** and **`NodeId`s are stable forever**, so `NodeId`-keyed side tables stay parallel and a detached subtree is simply never reached by `walk` (its entry stays in the arena, unlinked from any parent). A **generation counter** bumps on every mutation; it is the cache key for the per-generation `Styles`/`Layout` recomputed for mid-execution geometry reads (§8, below), invalidated by the next mutation. JS never holds a copy of the DOM — it holds opaque `NodeId` handles and every read/write routes through the syscall table to this one arena (single source of truth, no mirror DOM).

`ComputedStyle` carries what visibility, generated content, and layout need: a computed `display` keyword (the seven-variant `Display` enum — author/inline rule, else UA-implicit by tag; `display:none` is the derived `display == None` query, not a stored bool), the flex `order` and `flex-direction` reorder keys, inherited `visibility`, and `::before`/`::after` content strings. `Display::parse` folds every non-layout value (`grid`, `table*`, `contents`, unknown) to `Block` once, at compute time — the layout §2 coercion.

**`Layout` (`src/layout.rs`)** — Phase 3 realized that extension pattern: a second `NodeId`-keyed side table computed by `layout::compute(doc, &Styles, viewport_w)`. It holds `boxes: Vec<Option<Rect>>` (parallel to the arena) plus `orders` for flex reading order. `Rect { x, y, w, h }` is integer px in viewport coordinates, y-down; a rendered element carries `Some(Rect)`, a `display:none`/non-rendered/non-element node carries `None`. `child_order(id)` returns a flex container's in-flow children in reading order — sorted by `(order, source index)`, reversed for a `*-reverse` `flex-direction` — and an empty `Vec` for any non-flex node. The box *kind* is never stored: it is `Styles::display`, the single source of truth. Width is the hard-coded `VIEWPORT_WIDTH = 1280` constant (no `--viewport` flag); with no real fonts, geometry uses fixed metrics (16px font, 8px glyph advance, 20px line-height), so boxes are structural estimates, not pixel truth.

## Load-bearing contracts

- **Envelope** (`src/envelope.rs`, version `"0"`): `{frot, url:{requested, final?}, view, status}` plus exactly one of `out` (status `ok`), `needs` (status `needs`), `error:{kind, message}` (status `error`). Fields are only ever **added**, never renamed. Error kinds are the closed `kinds::` taxonomy (`fetch.dns`, `fetch.tls`, …, `parse`, `internal`). Two additive blocks ride any status: `http:{status}` (every network response; absent for `file://`), and — **only under `--js`** — `js:{scripts, errors, settled}` (`JsInfo`): `scripts` = scripts executed, `errors` = the page's own throws + unhandled promise rejections + refused/failed subfetches (all surfacing through the one §5 error channel), `settled` = the event loop reached quiescence within budget. It is the honesty channel for *partial* execution that outcome-based `needs` detection cannot see — a budget-killed run that still rendered must not look complete. Script errors do **not** flip `status`; only the needs detector and the transport/parse taxonomy do.
- **Exit codes**: `0` = `ok`/`needs` envelope, `1` = `error` envelope (still JSON on stdout), `2` = usage error (no envelope; message on stderr).
- **`needs` beats `ok`**: capability-gap detection runs before view production, only for views whose output depends on rendered content (`text`, `ax`, `links`, `forms` — not `dom`, not `meta`). The one detector so far is the SPA-shell heuristic (`src/needs.rs`): body text empty + document has `<script>` + ≤3 real element descendants under `<body>` → `needs: ["js"]`.
- **One view per invocation**; `--out` is mandatory, capabilities are orthogonal flags.

## Module map

| Path | Role |
|---|---|
| `src/cli.rs` | hand-rolled argv parser → `Args` |
| `src/fetch.rs` | blocking ureq GET; redirects, gzip/brotli, charset decode |
| `src/fetch/firefox_tls.rs` | ureq `Connector` that TLS-wraps HTTPS with a Firefox ClientHello (`craftls`) + Title-Case header names — the browser-identity transport seam |
| `src/dom.rs` | html5ever → arena facade |
| `src/dom/mutate.rs` | `--js` DOM mutation ops (append-plus-relink) + generation counter |
| `src/needs.rs` | capability-gap detection (post-JS under `--js`) |
| `src/css/{parse,selparse,selector,cascade}.rs` | dependency-free CSS subset engine |
| `src/layout/{,flex,inline}.rs` | on-demand block/inline/flex geometry → `NodeId`-keyed `Layout` side table |
| `src/ax/{role,name,tree}.rs` | role mapping, accname subset, AX tree builder |
| `src/views/{text,dom,links,forms,meta,bboxes}.rs` | pure view functions |
| `src/tags.rs` | block-level tag set, shared by `views::text` and `layout` (single source of truth) |
| `src/js.rs` | `--js` capability entry: run scripts, return post-JS `Document` + `JsInfo` |
| `src/js/engine.rs` | the swappable rquickjs seam (eval, ES-module eval, interrupt handler, memory cap, job queue, rejection tracking) — no `rquickjs` type escapes it |
| `src/js/loader.rs` | ES-module `Resolver`/`Loader` (`Runtime::set_loader`, `loader` feature): resolves `import` specifiers and loads module source through the §6 subfetch cache — no `rquickjs` type escapes it |
| `src/js/syscall{,.rs}`, `syscall/{env,net}.rs` | the ~20-op host↔JS table (node queries, mutations, `innerHTML`, selector match via `css::selector`, geometry, subfetch, env facts, console) |
| `src/js/prelude{,.rs}`, `prelude/*.js` | bundled JS web-API (`Node`/`Element`/`Document`, `querySelector`, timers, `fetch`/XHR, storage, `navigator`/`location`/`matchMedia`) built on the syscalls, `include_str!`-embedded |
| `src/js/session.rs` | event loop: virtual clock, horizon, timers/rAF, microtask drain, lifecycle events, wall-clock budget → `settled` |
| `src/js/subfetch.rs` | once-then-frozen GET-only network cache behind `__frot_subfetch` (`fetch`/XHR + external `<script src>` + ES-module loader) |
| `src/js/geometry.rs` | per-generation `Styles`/`Layout` cache for `getBoundingClientRect`/`offset*`/`getComputedStyle` |
| `src/envelope.rs` | output contract |
| `src/run.rs` | glue: pipeline + exit codes |

Every module has a colocated `tests.rs`; `tests/binary.rs` drives the compiled binary end-to-end against a mockito server. Gates: 100% line coverage, ≤300 lines per source file, clippy `-D warnings` (pre-commit).

## Fetch behavior

`ureq` 3 (HTTP/1.1 only — no h2), 15 s global timeout, 16 MiB body cap, redirects followed (final URL reported in `url.final`), gzip/brotli decoding. Charset: `Content-Type` header, else `<meta charset>` sniff in the first 1 KiB, else UTF-8, decoded via `encoding_rs`. Non-2xx responses do not error at the transport layer (`http_status_as_error(false)`) — the body is still read — but `run.rs` surfaces the code in an `http` block and flips status ≥ 400 to an `error` envelope (`error.kind: http.<code>`), so a 404/500/bot-challenge body is never reported as a successful impression.

**Browser-identity transport (`src/fetch/firefox_tls.rs`).** WAFs fingerprint the TLS ClientHello (JA3/JA4) and the on-wire header casing; a stock rustls handshake with lowercase header names 403s even behind perfect Firefox headers (proven on StackOverflow). So the HTTPS handshake is supplied by `craftls` — a fork of rustls with a craftable ClientHello — carrying the Firefox fingerprint (cipher/extension order, GREASE, key-share, padding), and the transport Title-Cases header names on the wire (ureq/`http` emit them lowercase). It plugs in as a `ureq` `Connector` (`ureq::unversioned::transport`, chained after `TcpConnector`), so ureq keeps HTTP/1.1, redirects, decompression, timeouts and the error taxonomy — only the handshake bytes and header casing change; no `craftls` type leaks past `firefox_tls.rs` (the same seam discipline as `js/engine.rs`). Crypto is `ring` (pure Rust, static-musl clean); the only C is an incidental `zstd-sys` for cert-compression. Cost: **+2.1 MiB → ≈7.06 MiB** (musl 7.25 MiB), inside the 5–15 MB envelope. Residual soft tells (accepted — they don't gate current targets): ALPN reads `http/1.1` not `h2`, and ureq's header *order/set* is not Firefox's (bl-28d6 owns the header set). Scope: fingerprint-matching only; CAPTCHA / JS-challenge solving is refused.

Under `--css`, external `<link rel=stylesheet>` hrefs are resolved against the final URL and fetched best-effort — a failed sheet is skipped, never fatal (CSS is an enhancement to the impression, not a precondition).

`file://` URLs take the read path instead: `std::fs` read, same size cap, charset from the `<meta>` sniff (there is no Content-Type), `status: None`, empty headers. Read failures map to the additive error kind `fetch.file`.

## Layout (on-demand)

Layout is a *derivation*, not a capability — no `--layout` flag (`docs/design/layout.md`). `run.rs` `build_envelope` builds a `Layout` iff the view demands geometry:

| View | Builds layout? |
|---|---|
| `bboxes` | **always** — its output *is* geometry |
| `ax` | **only under `--css`** — flex `order`/`*-reverse` is the sole source-order↔reading-order divergence, and only author CSS can express it |
| `dom`, `text`, `links`, `forms`, `meta` | never |

`bboxes` needs a `Styles` even without `--css` (layout branches on `display`), so `compute_styles` sources one *bare* (`css::compute_bare`: UA-implicit display + inline `style=`, no `<style>`/external) — `--css` keeps its "apply author CSS" meaning on every view. Once layout has run, `ax::ax_tree` emits a flex container's children in `child_order` instead of source order; absent layout it is bit-for-bit the Phase-2 path.

**`bboxes` output** (`src/views/bboxes.rs`) is a flat array in reading order, one entry per rendered element: `{ "i": <source-order index>, "tag", "rect": {"x","y","w","h"}, "text": <direct text | null> }`. `display:none` omitted, `visibility:hidden` kept; `i` is source order, so it is non-monotonic wherever flex reorders. Flat, not nested — structure is the `dom`/`ax` job.

**Honest signals.** Phase 3 is block + inline + basic flex at a 1280px viewport with fixed font metrics; grid, floats, `position` offsets, and table-layout are coerced to in-flow block/inline (deterministic, documented — `docs/design/layout.md` §6), and box-model sizing and flex wrap/grow/shrink/justify/align/gap are out (`@media` width/screen queries *are* evaluated against the fixed viewport — `src/css/media.rs`). There is deliberately **no `needs:["layout"]` signal**: unlike the SPA-shell heuristic, "uses grid/absolute" is not a clean binary — flow layout still yields *a* reading order and *approximate* boxes. Honesty is served instead by that published approximation contract; `NeedsKind::Layout` stays a reserved, dormant slot (as `NeedsKind::Css` is).

## JS execution (bounded, `--js`)

The full design is `docs/design/js.md`; the load-bearing as-built shape:

- **Bounded execution — one deadline (`src/js/session.rs`, §5).** Two clocks, three constants (constants, not flags — the 1280px-viewport severability posture). A **virtual clock**: timers schedule at virtual timestamps and the loop jumps to the earliest-due task; `Date.now()`/`performance.now()` read it. `VIRTUAL_HORIZON_MS = 10_000` drops any task due past the horizon, so `setInterval` pollers and `requestAnimationFrame` chains (rAF = a 16 ms virtual timer) self-terminate instead of needing per-API caps — "no timers past load". A single **wall-clock budget `EXEC_BUDGET_MS = 1_000`**, enforced by the engine's interrupt handler, covers script execution *and* subfetch time in one deadline (not two); hitting it stops the loop and marks the run unsettled. `JS_MEM_LIMIT = 64 MiB` caps the engine heap. Microtasks drain host-driven after every task; an unhandled exception aborts *that* task (counted) and the loop continues — a page that throws after rendering is still a good impression. **Settled** = queue empty and nothing due before the horizon.
- **Network — once-then-frozen, GET-only (`src/js/subfetch.rs`, §6).** `fetch` and `XMLHttpRequest` both wrap the one `__frot_subfetch` syscall (XHR is trivially "sync" — the whole loop is single-threaded and blocking), as do the external-`<script src>` runner and the ES-module loader. **GET only**: anything else rejects/throws (frot reads the web, it does not submit to it). Each absolute URL is fetched at most once per call and cached for the call's lifetime (deterministic within the call, nothing persists past it). Same rules as `--css` stylesheet subfetches: resolved against the final URL, `-H` headers ride same-origin only, redirects followed, 16 MiB cap, remote→local blocked. `SUBFETCH_MAX = 16` requests per call; network time spends the one wall-clock budget above.
- **ES modules — resolver/loader over subfetch (`src/js/loader.rs`, §4.1/§6).** A `type="module"` script (inline or external) evaluates as a real ES module via rquickjs's `loader` feature: a `Resolver`/`Loader` pair (`Runtime::set_loader`) resolves each `import` specifier against the importing module's URL and loads its source through the *same* once-then-frozen subfetch cache — one shared `SUBFETCH_MAX` budget, same header/remote→local rules, no side channel. An inline module's imports resolve against the page URL; an external module's against its own fetched URL, so nested graphs chain. **Bare** specifiers have no import map, so they are unresolvable exactly as in a browser without one — a counted §10 error; a failed/refused/non-2xx module fetch counts likewise, sibling scripts continuing. Module evaluation is async by spec — top-level `await` and dynamic `import()` settle through the microtask drain / settle loop under the one deadline; a rejected module promise is netted to a single count by a rejection watcher at the engine seam (quickjs can report a synchronous top-level throw twice).
- **Geometry — per-generation cache (`src/js/geometry.rs`, §8).** `getBoundingClientRect`/`offset*`/`getComputedStyle` route through two narrow syscalls served from a `Styles`+`Layout` pair computed for the DOM's *current generation* (authored CSS under `--css`, `css::compute_bare` without it — the same rule `bboxes` uses) and cached until the next mutation bumps the generation. `getComputedStyle` exposes only what the cascade actually computes (`display`, `visibility`, the flex reorder keys); unknown properties return `""`. Geometry answers inherit the layout approximation contract wholesale — structural estimates, not pixel truth.
- **`<noscript>` under `--js`.** The cascade is the single-source-of-truth home for display semantics, so when the JS capability ran it treats `<noscript>` as UA-implicit `display:none` (author-overridable) — browser-accurate, since scripting hides `<noscript>`. This is not inert: it is exactly what a page script reading `getComputedStyle(noscriptEl).display` observes (`"none"` when `--js` ran, vs `"inline"` otherwise). The structural views suppress `<noscript>` unconditionally regardless of `--js` (`text`/`ax` via `SKIP_TAGS`, `layout`/`bboxes` via `NON_RENDERED_TAGS`) — frot's denoising policy, which happens to agree with the cascade flip when JS ran, so it is an equivalent fast path there and is *not* collapsed into the cascade: doing so would make `<noscript>` render without `--js` (a regression) and force `text`/`ax` to consult a styles table they don't build without `--css`.

## Design decisions (log)

- **No `clap`** — argv parsing is ~120 hand-rolled lines; binary size wins (release binary ≈ 3.5 MB pre-JS — ≈ 4.7 MB with the Phase-4 engine compiled in — with `lto=fat`, `strip`, `panic=abort`).
- **No `cssparser`/`selectors`** — VISION named them, the implementation rejected them: the visibility/content need is narrow, and the dependency-escalation cost outweighed fidelity. Revisitable if the subset proves too thin.
- **Blocking I/O, no async runtime** — a single-shot process fetching a handful of resources gains nothing from tokio and pays startup + size for it.
- **RcDom absorbed, not wrapped** — rcdom's `Rc<RefCell<…>>` graph is copied once into the arena so the rest of the crate gets `Copy` node ids, cheap parallel side tables, and no interior mutability.
- **Views are pure functions** of `(Document, Option<&Styles>)` — capabilities compose by annotation, views by selection. No view knows which capabilities ran.
- **`-H` header passthrough, same-origin subfetch scope** (bl-f446, decided 2026-07-07) — repeatable curl-parity `-H "Name: value"`; caller's `User-Agent` replaces the default; `--css` sheet subfetches carry the headers only when same-origin with the page (scheme+host+port), else they go anonymous. `Authorization` never rides redirects (ureq default, not overridden). `-H` with `file://` is a usage error — under the same-origin rule the headers could never be sent, and an accepted flag must do something.
- **`file://` input rides the URL positional** (bl-ce6f, decided 2026-07-07) — no new flags, no bare-path sugar, no stdin (a stream has no base URL). A file read has no HTTP status (`FetchResult.status` is `Option`), no headers, and the same 16 MiB cap. Direction rule: local→remote is allowed (a saved page may fetch its CDN stylesheets under `--css`), remote→local is blocked (an http(s) page's `file:` stylesheet href is skipped — remote content never causes local reads).
- **Layout is an on-demand side table, not a flag** (Phase 3, bl-4cb4) — `layout::compute` mirrors the `Styles` pattern; the box *kind* is `Styles::display` (single source of truth), stored nowhere twice. The trigger lives in `run.rs` (bboxes always, ax under `--css`), keeping views pure functions of `(doc, styles?, layout?)`. Viewport is a hard-coded 1280px constant, not a flag — `bboxes`'s value is relative structure + reading order, largely width-insensitive, and a width knob is exactly the schema growth the non-goals discipline resists. Non-goals (grid/floats/positioning/tables, box-model sizing, fine flex) are coerced to flow and documented rather than signalled: no `needs:layout`, because coerced layout still yields a usable approximation (`docs/design/layout.md` §6).
- **`--js` engine: `rquickjs` (quickjs-ng), not Boa** (Phase 4, `docs/design/js.md` §1) — the disqualifier was bounding: rquickjs's sync `Runtime` has an interrupt handler (wall-clock budget), `set_memory_limit`, and a host-driven job queue; Boa has no interrupt hook and no memory limit, so untrusted page scripts cannot be bounded on it. quickjs-ng runs real framework bundles (React/Vue/jQuery); Boa has no such evidence. It is vendored C compiled by the `cc` crate (`default-features = false` — no `allocator` feature, so `set_memory_limit` is enforced on the default C allocator) with pregenerated bindings — no cmake, no bindgen, no system dep, no dynamic linkage; `*-linux-musl` is CI-tested, so the static binary holds. The C-dependency escalation (`~/AGENTS.md`) is discharged by that design doc + the subtask-4.1 spike's static-build proof. **Measured binary delta: +1.18 MiB → 4.59 MiB total**, comfortably inside the 5–15 MB VISION envelope. All rquickjs types stay behind `src/js/engine.rs` and `src/js/loader.rs` — the engine is swappable (Boa is the documented fallback if the C dep ever becomes untenable) without touching the syscall shim, mirroring how no `markup5ever` type leaks past `dom.rs`. **ES-module `import` resolution is built (bl-1b98):** it enables rquickjs's `loader` feature, which adds exactly one crate — `relative-path` (approved 2026-07-10; nothing else rides in) — driving a `Resolver`/`Loader` over the §6 subfetch cache. Measured cost is negligible: **+≈20 KiB → ≈4.73 MiB total** (the `loader` feature over the current baseline), still far inside the 5–15 MB VISION envelope.

## Known gaps (tracked in bl — run `bl ready`)

The 2026-07 arch pass filed a Phase 2.5 impression-fidelity epic (bl-2a9e) — HTTP status in the envelope (bl-d1c6), rejecting the unimplemented `--js`/`--out bboxes` surface (bl-957a), `nameFrom:contents` accname (bl-a864), layout-table demotion (bl-ab78) — which gated Phase 3 layout (bl-4cb4). Both have since **landed** (see "Where it stands" in `README.md`; `--out bboxes` is now a real view, not a usage error). `bl ready` is the live backlog.

## Open questions (decision tasks in bl)

- **`NeedsKind::Css` and `NeedsKind::Layout` are dormant** — the enum variants exist, nothing emits them. Reserved taxonomy slots, not dead code to delete: a future detector (content invisible without CSS? unrenderable without grid/absolute?) inhabits them, or a later review removes them. (Viewport convention — an earlier open question — is settled: the hard-coded 1280px constant, `docs/design/layout.md` §4.)
