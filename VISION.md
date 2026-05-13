# Vision

## The problem

There is a gap between `curl` (raw bytes, no semantics) and a full headless browser (Chromium, hundreds of MB, slow start, stateful, fragile). Harnesses live in this gap. They need to look at web pages programmatically — fetch a doc, read its structure, find a link, extract a table, see what an accessibility tree would expose — without standing up a browser pool.

The available options force a binary choice: too little (curl + regex, hope the page is SSR'd, brittle), or too much (Playwright, browser pool, container, anti-bot proxy infrastructure). Nothing sits cleanly in the middle as a small, fast, stateless tool that produces *machine-readable impressions* of the web.

## The name

`frottage` is an impression technique: press a surface against paper, rub graphite or charcoal across it, and capture the topology — outlines, raised features, structure — **without altering the original**. Archaeologists use it for inscriptions. Artists use it for surface texture. The metaphor maps to this tool with uncanny precision:

- No JS execution → no state mutation, no side effects, no interaction with the page beyond reading.
- Output is a *structural impression* (DOM, text, AX tree), not a faithful pixel render.
- The technique is fast, deterministic, and non-destructive.

The name scales with scope: even when we eventually add layout and JS, we're still taking impressions. We're not building a browser; we're building a way to lift the structure off a page.

## Principles

1. **Stateless interface.** A request is `(url, parameters) → outputs`. No sessions, no cookies-by-default, no hidden state. Calls are cacheable, reproducible, parallelizable.
2. **Single static binary.** No container, no runtime deps, no Chromium install. Drop on a machine, run.
3. **Minimal footprint.** Optimize for binary size, startup time, and memory floor. The interesting envelope is "smaller than a curl wrapper, more useful than one."
4. **Machine-first output.** Every output format is designed to be parsed, not displayed. AX tree is a first-class citizen, not an afterthought.
5. **Honest capability signals.** When a page can't be rendered faithfully at the current scope, return a clear, structured `needs-X` signal. Never silently produce a degraded result that looks complete.
6. **Elegance over completeness.** It's better to do less, well, with a clean boundary, than to half-implement the whole web. Where compromise is forced, push back.
7. **Testability is non-negotiable.** Every capability ships with golden-fixture tests against pinned real-world pages. If it can't be tested, it isn't built.

## The scope arc

frot is built in phases. Each phase delivers a usable tool; later phases extend the capability envelope without changing the interface.

### Phase 0 — Fetch + parse

- HTTP/1.1 + HTTP/2 client with sane defaults (redirects, content-encoding, character set detection).
- HTML5 parsing (via `html5ever`).
- Stable output: raw HTML, parsed DOM, final URL, response metadata.
- Realistic browser-ish User-Agent and TLS defaults; not anti-bot-grade, but not obviously a bot either.

### Phase 1 — Semantic impressions

- **AX tree** from semantic HTML + ARIA, computed without layout. The headline output.
- Readable text extraction, structure-preserving.
- Link and form extraction, structured.
- `needs-js` detector: heuristic signal that the page is an SPA shell and the impression is empty.

### Phase 2 — CSS-aware extraction

- Parse and apply CSS (via `cssparser` / `selectors`) for visibility and content semantics: `display:none`, `visibility:hidden`, pseudo-content, generated lists.
- Text output reflects what's *visible*, not what's in the markup.
- Still no layout. Still no JS.

### Phase 3 — Layout (partial)

- Block flow, inline flow, basic flex. Enough to compute reading order and reasonable bounding boxes for elements.
- Not pixel-perfect; serves AX tree refinement and reading-order computation, not screenshots.

### Phase 4 — JavaScript (cautious)

- A JS engine with a shimmed, partial DOM. Scope: enough to handle hydration of SSR'd content that progressively enhances after load. Not a full SPA host.
- Sites that exceed the shim get the `needs-js` signal, same as before. The boundary moves; it doesn't disappear.

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

A 5–15 MB static binary that returns a faithful structural impression of a web page in single-digit milliseconds, exposes a clean AX-tree output that LLM agents prefer over raw DOM, fails honestly when the page demands more, and has a roadmap that can be ignored without the tool feeling incomplete.
