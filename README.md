# frot

Take an impression of a web page — structure, text, accessibility tree — without rendering or executing it. Like a gravestone rubbing for the web.

`frot` is the curl that renders: a stateless, single-binary CLI (~3.5 MB, no runtime deps) that sits in the gap between `curl` and a headless browser. You give it a URL, a capability recipe, and one output view; it gives you back a machine-parseable JSON envelope. Built for harnesses that need to look at pages programmatically without standing up a browser pool.

## Usage

```
frot <url> [-H "Name: value"] [--css] [--js] --out <dom|text|ax|links|forms|bboxes|meta>
```

```console
$ frot https://example.com --out text
{"frot":"0","url":{"requested":"https://example.com","final":"https://example.com/"},"view":"text","status":"ok","out":"Example Domain\nExample Domain\nThis domain is for use in documentation..."}

$ frot https://a-spa-shell.example --out text
{"frot":"0","url":{...},"view":"text","status":"needs","needs":["js"]}
```

Exit codes: `0` for an `ok` or `needs` envelope, `1` for an `error` envelope (still JSON on stdout), `2` for a usage error (no envelope; message on stderr).

The URL may also be `file://` — take an impression of a document you already have (a saved page, a crawl artifact, a cached fetch) without refetching it. Relative hrefs resolve against the file URL; under `--css` a local page may fetch its remote stylesheets, but a remote page can never read a `file:` one. Note that `file://` reaches local disk: validate schemes yourself before passing untrusted URLs, same as with curl.

## Output views

Each invocation returns exactly one view, selected with `--out`:

- `dom` — the parsed document tree, as JSON nodes (`element`, `text`, `comment`, `doctype`)
- `text` — readable content in source order; whitespace collapsed, `<pre>` preserved
- `links` — every `<a>`, `<area>`, and `<link>`, with absolute hrefs and `rel` tokens
- `forms` — `<form>` actions, methods, enctypes, and structured field lists
- `meta` — `<title>`, `<html lang>`, charset, canonical link, and `<meta>` entries
- `ax` — accessibility tree (roles, accessible names, levels)
- `bboxes` — per-element geometry: a flat array in reading order, one entry per rendered element (`{i, tag, rect:{x,y,w,h}, text}`), laid out at a fixed 1280px viewport

Every run emits the same envelope shape — `frot`, `url`, `view`, `status`, plus `out` (for `ok`), `needs` (for `needs`), or `error` (for `error`). Fields are only ever added, never renamed.

## Capability flags

The capability recipe is orthogonal to the view: it picks what to do to the document before producing output.

- `--css` — parse `<style>`, inline `style=`, and external `<link rel=stylesheet>` CSS; `display:none` and inherited `visibility` filter the `text` and `ax` views, and `::before`/`::after` generated content is folded into them
- `--js` (Phase 4, **not implemented** — currently rejected as a usage error) — execute scripts against a partial DOM under bounded execution
- `-H "Name: value"` / `--header` — send a request header, repeatable, curl-style. A caller-supplied `User-Agent` replaces the default. Headers ride `--css` stylesheet subfetches only when the sheet shares the page's origin — credentials never leak cross-origin. No cookie jar, no sessions: headers are per-call input.

When a page can't be rendered faithfully under the current recipe — e.g. an empty SPA shell with `--js` off — frot emits a `needs` envelope rather than a silently degraded `out`.

The `--css` engine is a deliberately small, dependency-free subset (type/`.class`/`#id`/`*`/`[attr]` selectors, descendant and child combinators, `::before`/`::after`). External stylesheets are fetched best-effort — a failed fetch is skipped, not fatal. No `@media`/`@supports`; the CSS engine computes visibility and generated content only — geometry is a separate on-demand layout derivation (Phase 3, `bboxes`).

## Where it stands

Phases 0–3 (plus the Phase 2.5 impression-fidelity pass) have landed and are verified against real pages: fetch (HTTP/1.1+2, redirects, gzip/brotli, charset detection, browser-ish UA) + HTML5 parse; all seven views, `bboxes` included; the `needs-X` taxonomy with the SPA-shell `needs-js` heuristic; the `--css` capability; and Phase 3's on-demand layout. Sub-second on real pages; the binary is ~3.5 MB.

Phase 2.5 (impression fidelity — the honest-capability-signals principle catching up with the shipped surface) closed the gaps the 2026-07 arch pass found:

- **HTTP status is in the envelope.** Every network response carries an `http` block (`"http":{"status":200}`); a non-2xx response (status ≥ 400) flips the envelope to `status:"error"` (exit 1) with `error.kind:"http.<code>"` — so a 404/500/bot-challenge page is no longer reported as a successful impression. (`file://` reads have no HTTP response, so no `http` block.)
- **Unimplemented surface fails honestly.** Accepted-but-unbuilt flags are rejected as usage errors (exit 2, message on stderr) naming the phase that delivers them, never silently accepted — `--js` today, and `--out bboxes` until Phase 3 made it a real view (below).
- **AX names are role-correct.** Name-from-content is restricted to WAI-ARIA `nameFrom:contents` roles (`link`, `heading`, `button`, `cell`, …); container roles (`table`, `rowgroup`, `list`, …) are no longer named from their full subtree text.
- **Layout tables collapse in `ax`.** A `<table>` with no data-table semantics (no `<caption>`/`<th>`/`summary`/`role`/`aria-label`) is demoted to `presentation`, so table-layout sites (e.g. Hacker News) surface content without `table`/`row`/`cell` scaffolding noise.

Phase 3 (on-demand layout) added element geometry with no new flag:

- **`--out bboxes` is a real view.** It returns a flat array in reading order, one entry per rendered element — `{ "i": <source-order index>, "tag", "rect": {"x","y","w","h"}, "text": <the element's direct text | null> }`. `display:none` elements are omitted; `visibility:hidden` are kept. `i` is source order, so it is non-monotonic wherever flex `order`/`flex-direction: *-reverse` shuffles reading order.
- **Layout is on-demand, not a flag.** `bboxes` always builds it; `--out ax` builds it only under `--css`, so flex `order`/`*-reverse` surface as visual reading order in the AX tree. Every other view skips layout entirely.
- **Viewport is 1280px, hard-coded** (no `--viewport`/`--width` knob).

Known limitations — honest signals, not silent failures:

- **`--js` is not built yet.** It is a usage error until Phase 4 (JS) lands.
- **Layout is an approximation, not pixel truth.** Geometry is a structural estimate: fixed font metrics (16px font, 8px average glyph advance, 20px line-height, greedy word-wrap), the fixed 1280px viewport, and block + inline + basic flex only. Grid, floats, `position:absolute/fixed/sticky/relative` offsets, and table-layout are coerced to in-flow block/inline; box-model widths/margins/padding, flex wrap/grow/shrink and precise justify/align/gap, and `@media` are out. There is **no `needs:["layout"]` signal** — the published approximation contract (`docs/design/layout.md` §6) is the honesty mechanism.

Next capability phase: Phase 4 (bounded `--js`).

Docs: `VISION.md` is the why and the roadmap; `ARCHITECTURE.md` is the as-built how (pipeline, contracts, decision log).

## Building

```
make setup           # install dev tooling (cargo-llvm-cov)
make precommit-install
make build
make test
make cov             # 100% line coverage gate (matches the pre-commit hook)
```

## License

TBD.
