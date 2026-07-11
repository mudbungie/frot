# Vision

## The problem

There is a gap between `curl` (raw bytes, no semantics) and a full headless browser (Chromium, hundreds of MB, slow start, stateful, fragile). Harnesses live in this gap. They need to look at web pages programmatically — fetch a doc, read its structure, find a link, extract a table, see what an accessibility tree would expose — without standing up a browser pool.

The available options force a binary choice: too little (curl + regex, hope the page is SSR'd, brittle), or too much (Playwright, browser pool, container, anti-bot proxy infrastructure). Nothing sits cleanly in the middle as a small, fast, stateless tool that produces *machine-readable impressions* of the web.

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
6. **Elegance over completeness.** It's better to do less, well, with a clean boundary, than to half-implement the whole web. Where compromise is forced, push back.
7. **Testability is non-negotiable.** Every capability ships with golden-fixture tests against pinned real-world pages. If it can't be tested, it isn't built.

## The model

frot is composed of two orthogonal axes:

- **Capability flags** describe *what to do* before producing output. They are transformations on the document:
  - `--css` — parse and apply stylesheets, annotating the DOM with computed styles. Visibility (`display:none`, `visibility:hidden`) and generated content propagate to all subsequent outputs.
  - `--js` — execute scripts against the current DOM. JS mutates the DOM the way it does in a real engine, within bounded, stateless execution semantics.
- **The output flag** picks *what to return*. Exactly one of: `dom`, `text`, `ax`, `links`, `forms`, `bboxes`, `meta`.

```
frot <url> [--css] [--js] --out <view>
```

The caller composes a recipe of capabilities; frot returns one output reflecting the document under that recipe. Internally, capabilities are composable functions over a shared artifact (DOM + side tables); they are not a strict pipeline. JS can run without CSS, and the DOM that downstream readers see is the DOM as the chosen capabilities have left it.

Layout (block / inline / flex) is a *derivation*, not a capability. There is no `--layout` flag. Layout runs implicitly when something needs geometry — an output like `bboxes`, or a JS script reading `offsetWidth`. Outside that, it is skipped entirely.

When the recipe is insufficient — e.g. the page is an empty SPA shell and `--js` is off — frot emits a structured `needs-X` signal rather than a misleadingly thin output.

## The scope arc

frot is built in phases. Each phase delivers a usable tool; later phases extend the capability envelope without changing the interface.

### Phase 0 — Fetch + parse + envelope — **landed**

- HTTP/1.1 + HTTP/2 client with sane defaults (redirects, content-encoding, character set detection).
- HTML5 parsing (via `html5ever`).
- CLI surface and the machine-first output envelope. Outputs at this phase: `dom`, `text` (raw, source-order, no visibility filter), `links`, `forms`, `meta`.
- Realistic browser-ish User-Agent and TLS defaults; not anti-bot-grade, but not obviously a bot either.
- The envelope shape, the `needs-X` signal shape, and the error taxonomy are load-bearing and do not change in later phases.

### Phase 1 — Semantic impression — **landed**

- AX tree from semantic HTML + ARIA, computed without styling. The headline output: `--out ax`.
- The `needs-X` taxonomy lands here; `needs-js` is its first inhabitant — a heuristic signal that the page is an SPA shell and the impression under the current recipe is empty.

### Phase 2 — CSS application — **landed**

- `--css` capability flag.
- Parse and apply CSS for visibility and content semantics: `display:none`, `visibility:hidden`, pseudo-content. Implemented as a small dependency-free subset engine rather than pulling in `cssparser`/`selectors` — the binary-size and dependency-discipline constraints outweighed full CSS fidelity for a narrow visibility/content need. External `<link>` stylesheets are fetched best-effort (failures skipped); `@media`/`@supports` are out of scope; layout (and thus list-marker generation) defers to Phase 3.
- Subsequent outputs (text, AX) reflect what's *visible*, not what's in the markup.

### Phase 2.5 — Impression fidelity — **landed**

Debt surfaced by the 2026-07 arch pass; it precedes new capability because it is the honesty principle catching up with the shipped surface:

- Surface the HTTP outcome in the envelope (additive `http` block). Today a 404/500/bot-challenge body is reported as an `ok` impression.
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
- Bounded execution as promised: a virtual-clock event loop under a single wall-clock deadline (1 s, covering script *and* network time), no timers firing past load (a 10 s virtual horizon self-terminates `setInterval`/`requestAnimationFrame`), a 64 MiB engine heap, no persistence (`localStorage`/`sessionStorage`/`cookie` in-memory + born-empty, `indexedDB` absent), and `fetch`/XHR served GET-only once-then-frozen.
- Engine chosen at first claim: **`rquickjs`** (Rust bindings to the maintained quickjs-ng fork) over Boa — Boa cannot *bound* untrusted execution (no interrupt hook, no memory limit), which was disqualifying. Vendored C via `cc`, static musl holds, +1.18 MiB. The engine sits behind a swappable seam (`src/js/engine.rs`); Boa remains the documented fallback.
- Sites that exceed the shim get `needs-js`, same as before — the post-JS document is re-tested, so `needs-js` now means "needs more JS than the shim gives." The boundary moved; it did not disappear. Partial runs are additionally reported through an envelope `js:{scripts, errors, settled}` block.
- **Non-goals held honest (`docs/design/js.md` §11).** No interaction (no synthetic clicks/input/scroll — the only events dispatched are the `DOMContentLoaded`/`load` lifecycle pair), no navigation, no persistence, no iframes/workers/WASM/canvas rendering, not a stealth runtime. **ES-module `import` resolution did not ship** — it needs a new dependency (`rquickjs`'s `loader` feature → the `relative-path` crate) held to the no-new-deps posture, so it is filed as a follow-up (`bl-1b98`); a `type="module"` script runs as a classic script until then.

### Phase 5+ — TBD

If Phase 4 holds, push toward more Web APIs. If it doesn't, accept the ceiling and ship a clean tool that defers to a real browser for the long tail.

## What this is not

- Not a browser. Not a competitor to Playwright or Browserless.
- Not anti-bot infrastructure. If a site is gated by Cloudflare, frot will fail honestly.
- Not a screenshot tool. Pixels are not the output.
- Not a business. This is a harness component, shared for portability.

## The non-goals discipline

Resist three temptations specifically:

- **Don't add conditionals to the parameter schema.** The moment frot needs to express "click X if Y is present," it becomes a browser-automation DSL, badly. The escape hatch for that case is *a different tool*, not a feature.
- **Don't grow a session model.** Statelessness is load-bearing. If a workflow needs sessions, the harness composes calls; frot does not maintain them.
- **Don't optimize for the long tail of broken sites.** Coverage of the easy 70% with a tiny binary is worth more than 95% with a heavy one.

## What success looks like

A 5–15 MB static binary that returns a faithful structural impression of a web page in single-digit milliseconds at the lower capability tiers (sub-second once JS is in play), exposes a clean AX-tree output that LLM agents prefer over raw DOM, fails honestly when the page demands more, and has a roadmap that can be ignored without the tool feeling incomplete.
