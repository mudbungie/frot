# frot

[![CI](https://github.com/mudbungie/frot/actions/workflows/ci.yml/badge.svg)](https://github.com/mudbungie/frot/actions/workflows/ci.yml)

Take an impression of a web page — structure, text, accessibility tree — without rendering or executing it. Like a gravestone rubbing for the web.

`frot` is the curl that renders: a stateless, single-binary CLI (~7 MB, no runtime deps — the embedded JS engine and a browser-matching TLS stack are compiled in unconditionally) that sits in the gap between `curl` and a headless browser. You give it a URL, a capability recipe, and one output view; it gives you back a machine-parseable JSON envelope. Built for harnesses that need to look at pages programmatically without standing up a browser pool.

## Usage

```
frot <url> [-H "Name: value"] [--css] [--js] [--js-errors] --out <dom|text|ax|links|forms|bboxes|meta>
```

```console
$ frot https://example.com --out text
{"frot":"0","url":{"requested":"https://example.com","final":"https://example.com/"},"view":"text","status":"ok","out":"Example Domain\nExample Domain\nThis domain is for use in documentation..."}

$ frot https://a-spa-shell.example --out text
{"frot":"0","url":{...},"view":"text","status":"needs","needs":["js"]}
```

Exit codes: `0` for an `ok` or `needs` envelope, `1` for an `error` envelope (still JSON on stdout), `2` for a usage error (no envelope; message on stderr). This sentence derives from the authoritative CLI contract — `docs/design/posix.md`, the POSIX profile stating exactly what frot claims (and does not) about argv syntax, streams, exit statuses, signals, and lifecycle; `scripts/posix-suite.sh` gates it (`make posix`).

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
- `--js` — execute the page's scripts in an embedded engine (`rquickjs`/quickjs-ng) against the *real* DOM, then let the rest of the pipeline consume the post-JS document. Execution is bounded: a virtual-clock event loop (timers/`requestAnimationFrame` self-terminate at a 10 s virtual horizon — no timers past load), two separate bounds — 1 s of **CPU** time for script execution (`EXEC_CPU_MS`, enforced at the engine interrupt) and 1 s of **wall** time for network (`NET_BUDGET_MS`, enforced at the subfetch seam) — and a 64 MiB engine heap. Network reads (`fetch`/`XMLHttpRequest`) are GET-only and served once-then-frozen, under the `--css` same-origin header rules. No interaction (no synthetic clicks/input), no navigation, no persistence — see the non-goals below. **ES-module `import` resolution is built** (`bl-1b98`): a `type="module"` script — inline or external — evaluates as a real module; relative/absolute-URL `import` specifiers resolve and load through the same once-then-frozen subfetch cache, top-level `await` and dynamic `import()` settle inside those same bounds. Bare specifiers (`import x from 'react'`) have no import map, so they are unresolvable exactly as in a browser without one — a counted error, the page still yielding the honest `needs-js` signal.
- `--js-errors` — requires `--js`; adds a bounded `js.messages` array (`[{kind, text}]`, first 32) to the `js` block, the *what* behind the `js.errors` count. Three classes carry a message — `throw` (a script/module exception), `report` (`reportError`/`window.onerror`/a dispatched window `'error'` — the channel a caught React render crash uses, so a dead app's messages are visible not just counted), and `subfetch` (a failed external `<script src>`, named by its spec). Rejections, refused navigations, and timer/lifecycle throws stay count-only. Bare (without `--js`) it is a usage error. The `js.errors` count is emitted with or without the flag.
- `-H "Name: value"` / `--header` — send a request header, repeatable, curl-style. A caller-supplied `User-Agent` replaces the default. Headers ride `--css` stylesheet subfetches only when the sheet shares the page's origin — credentials never leak cross-origin. No cookie jar, no sessions: headers are per-call input.

When a page can't be rendered faithfully under the current recipe — e.g. an empty SPA shell with `--js` off — frot emits a `needs` envelope rather than a silently degraded `out`.

The `--css` engine is a deliberately small, dependency-free subset (type/`.class`/`#id`/`*`/`[attr]` selectors, descendant and child combinators, `::before`/`::after`). External stylesheets are fetched best-effort — a failed fetch is skipped, not fatal. `@media` width/screen queries are evaluated against the fixed 1280×720 viewport (`@supports` is not); the CSS engine computes visibility and generated content only — geometry is a separate on-demand layout derivation (Phase 3, `bboxes`).

## Where it stands

Phases 0–5 (plus the Phase 2.5 impression-fidelity pass) have landed and are verified against real pages: fetch (HTTP/2 with HTTP/1.1 fallback by ALPN, redirects, gzip/brotli, charset detection) + HTML5 parse; all seven views, `bboxes` included; the `needs-X` taxonomy with the SPA-shell `needs-js` heuristic; the `--css` capability; Phase 3's on-demand layout; Phase 4's bounded `--js`; and Phase 5's coherent Firefox-140-esr network identity (one browser persona derives TLS, ALPN/h2, headers, cookies, `navigator` and clocks — `docs/design/identity.md`). Sub-second on real pages; the static binary is ~7.6 MB (the JS engine and TLS/h2 stack are compiled in unconditionally). The identity is **coherent, not byte-exact**: the capstone field trial (`bl-d66b`, 2026-07-21) confirmed it removes the wire self-contradictions a WAF fingerprints, but measured **no** access-gate change — the honest justification is coherence and TLS security maintenance, not bot-gate bypass, and frot still stops honestly at declared challenges (`needs:["human"]`).

Phase 2.5 (impression fidelity — the honest-capability-signals principle catching up with the shipped surface) closed the gaps the 2026-07 arch pass found:

- **HTTP status and headers are in the envelope.** Every network response carries an `http` block (`"http":{"status":200,"headers":[{"name":"server","value":"…"}]}`); a non-2xx response (status ≥ 400) flips the envelope to `status:"error"` (exit 1) with `error.kind:"http.<code>"` — so a 404/500/bot-challenge page is no longer reported as a successful impression. `http.headers` is a bounded, decision-relevant allowlist (`retry-after`, `cf-mitigated`, `server`, `x-datadome`, `content-type`) in wire order — the evidence that tells a bot-defence refusal from a genuine response; `set-cookie` and volatile per-request headers are never surfaced. (`file://` reads have no HTTP response, so no `http` block.)
- **Unimplemented surface fails honestly.** Accepted-but-unbuilt flags are rejected as usage errors (exit 2, message on stderr) naming the phase that delivers them, never silently accepted — this gated `--js` until Phase 4 built it, and `--out bboxes` until Phase 3 made it a real view (both below).
- **AX names are role-correct.** Name-from-content is restricted to WAI-ARIA `nameFrom:contents` roles (`link`, `heading`, `button`, `cell`, …); container roles (`table`, `rowgroup`, `list`, …) are no longer named from their full subtree text.
- **Layout tables collapse in `ax`.** A `<table>` with no data-table semantics (no `<caption>`/`<th>`/`summary`/`role`/`aria-label`) is demoted to `presentation`, so table-layout sites (e.g. Hacker News) surface content without `table`/`row`/`cell` scaffolding noise.

Phase 3 (on-demand layout) added element geometry with no new flag:

- **`--out bboxes` is a real view.** It returns a flat array in reading order, one entry per rendered element — `{ "i": <source-order index>, "tag", "rect": {"x","y","w","h"}, "text": <the element's direct text | null> }`. `display:none` elements are omitted; `visibility:hidden` are kept. `i` is source order, so it is non-monotonic wherever flex `order`/`flex-direction: *-reverse` shuffles reading order.
- **Layout is on-demand, not a flag.** `bboxes` always builds it; `--out ax` builds it only under `--css`, so flex `order`/`*-reverse` surface as visual reading order in the AX tree. Every other view skips layout entirely.
- **Viewport is 1280px, hard-coded** (no `--viewport`/`--width` knob).

Phase 4 (bounded `--js`) added the one capability that mutates the DOM:

- **Scripts run against the real arena, not a mirror.** JS holds opaque node handles; every read/write goes through a narrow (~20-op) host syscall table to the *same* `Document` every other view reads (single source of truth). `--js` runs first — before needs detection, CSS, layout, and the view — so the whole pipeline consumes the post-JS document exactly as it consumes a static one.
- **Engine: `rquickjs` (quickjs-ng), vendored C via `cc`** — no cmake/bindgen/system dep; the static musl binary holds. It runs real framework bundles (React 17 UMD + `ReactDOM.render`, Vue 3 global build, jQuery 3.7.1 settle end-to-end with zero errors).
- **Bounded and honest.** A budget-killed or partial run is reported: the envelope gains a `js` block, emitted only under `--js` — `"js":{"scripts":<executed>,"errors":<throws + unhandled rejections + refused fetches>,"settled":<loop reached quiescence within its bounds>}`. An unsettled run also names the bound that ended it — `"stopped":"budget"` (the compute budget was spent: the page is heavier than frot underwrites, so the answer is a real browser) or `"stopped":"network"` (the network deadline passed and a fetch was refused: the transport was slow, not the page). Present only when `settled` is `false`; a settled run has no bound to name. `--js-errors` adds a bounded `messages` array of `{kind, text}` detail behind that count (opt-in, additive). Post-JS the `needs-js` heuristic re-runs: a page still a shell after JS gets `needs-js` (now "needs more JS than the shim gives"). Script throws are normal web weather and do not flip `status`; only the needs detector and the transport/parse error taxonomy do.

Known limitations — honest signals, not silent failures:

- **`--js` runs page scripts, ES modules included.** A `type="module"` script evaluates as a real module: relative/absolute-URL `import`s resolve and load through the once-then-frozen subfetch cache (same GET-only, same-origin-header, deadline/byte-pool policy as `fetch`/XHR — there is no request-count cap; work is bounded by the 1 s CPU compute budget, the 1 s wall network deadline, and a pooled 64 MiB of response bytes), and top-level `await`/dynamic `import()` settle inside those bounds. Bare specifiers have no import map, so they are unresolvable (a counted `js.errors` failure), like a browser without one — module resolution rides `rquickjs`'s `loader` feature (the `relative-path` dep it adds). `--js` is also deliberately non-interactive: no synthetic clicks/input/scroll, no navigation (`location`/`history` writes are counted no-ops), no persistence across calls (`localStorage`/`sessionStorage`/`cookie` are in-memory and born empty; `indexedDB` is a coherent but stateless masquerade — present for feature detection, but with no backing store an `open()` never completes), no iframes/workers/WASM/canvas rendering.
- **Layout is an approximation, not pixel truth.** Geometry is a structural estimate: fixed font metrics (16px font, 8px average glyph advance, 20px line-height, greedy word-wrap), the fixed 1280px viewport, and block + inline + basic flex only. Grid, floats, `position:absolute/fixed/sticky/relative` offsets, and table-layout are coerced to in-flow block/inline; box-model widths/margins/padding and flex wrap/grow/shrink and precise justify/align/gap are out (`@media` width/screen queries *are* evaluated, against the fixed viewport). There is **no `needs:["layout"]` signal** — the published approximation contract (`docs/design/layout.md` §6) is the honesty mechanism.

Next: Phase 5+ is TBD (VISION.md) — push toward more Web APIs if Phase 4 holds, or accept the ceiling and defer the long tail to a real browser.

Docs: `VISION.md` is the why and the roadmap; `ARCHITECTURE.md` is the as-built how (pipeline, contracts, decision log).

## Building

```
make setup           # install dev tooling (cargo-llvm-cov)
make precommit-install
make build
make test
make cov             # 100% line coverage gate (matches the pre-commit hook)
```

## CI/CD

GitHub Actions runs the same gates as the local pre-commit hook, so nothing
lands ungated even if the hook is skipped or absent:

- **`.github/workflows/ci.yml`** (every push + pull request) — `cargo fmt
  --check`, `make lint` (clippy `-D warnings`), `make cov` (100% lines +
  regions), the source-file line cap, `make posix` (the POSIX conformance
  suite gating `docs/design/posix.md` at the process boundary), and `make
  package` (`cargo publish --dry-run --locked` — the crates.io packaging gate;
  it never publishes). The gate values are never re-typed in CI: coverage
  delegates to `make cov`, the 300-line limit is enforced by
  `scripts/line-limit.sh` (the same script the pre-commit hook calls), and the
  conformance checks live in `scripts/posix-suite.sh` alone.
- **`.github/workflows/release-plz.yml`** — the release pipeline. Every push to
  main refreshes a single "release PR" that bumps the version and stages the
  changelog; merging that PR is the human control point. Once CI concludes green
  on main, the release job tags `v<version>`, cuts the GitHub Release, and runs
  `cargo publish` to crates.io (needs the `CARGO_REGISTRY_TOKEN` repo secret),
  then a dependent job builds the stripped, static
  `x86_64-unknown-linux-musl` binary, asserts its size sits inside the
  documented 5-15 MiB envelope (`scripts/check-size.sh`), runs the POSIX
  conformance suite against that exact stripped artifact (the second target
  `docs/design/posix.md` §7 claims; locally, `make posix-musl`), and uploads
  it as a Release asset.

`rust-toolchain.toml` pins the compiler (and the `llvm-tools-preview` component
and musl target) so local and CI agree.

## License

Dual-licensed under **MIT OR Apache-2.0**, at your option — the declaration
`Cargo.toml` carries and the one published with the crate.

The full terms are in the repository root and ship in the published package:
[`LICENSE-MIT`](LICENSE-MIT) and [`LICENSE-APACHE`](LICENSE-APACHE).
