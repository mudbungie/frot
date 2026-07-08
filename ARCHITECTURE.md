# Architecture (as-built)

What the code actually is, as of the 2026-07 arch pass. `VISION.md` says why and where; this says how. When they disagree, the code wins — fix whichever doc is wrong.

## The pipeline

One process, one pass, one envelope:

```
argv ──cli::parse──► Args { url, css, js, out }
url ──fetch::fetch──► FetchResult { final_url, status, headers, body, charset }
body ──dom::Document::parse──► Document (arena)
(view, doc) ──needs::detect──► [NeedsKind]     — non-empty short-circuits to a `needs` envelope
(doc, sheets) ──css::compute_with──► Styles    — --css (bboxes: compute_bare even without it)
(doc, styles) ──layout::compute──► Layout      — on-demand: bboxes always, ax under --css; else skipped
(doc, styles, layout?) ──views::* / ax::ax_tree──► JSON payload
payload ──Envelope──► one JSON line on stdout + exit code
```

Orchestration lives in `src/run.rs` (~100 lines); everything below it is a pure function of its inputs. There is no global state anywhere — statelessness is enforced by signature.

## Core data structures

**`Document` (`src/dom.rs`)** — html5ever parses into `RcDom`, which is immediately *absorbed* into a flat `Vec<NodeEntry>` arena indexed by `NodeId` (`u32`). The facade exposes `walk` (preorder `Enter`/`Exit` events), `find_by_tag`, `text_content`; no `markup5ever` type leaks past this module, so parser/storage can be swapped without touching views. Tag and attribute names are lowercased at absorption.

**`Styles` (`src/css.rs`)** — a `Vec<ComputedStyle>` parallel to the arena, indexed by the same `NodeId`. Capabilities do not mutate the DOM; they produce side tables that views consult. **This is the extension pattern**: Phase 3 layout output is another `NodeId`-indexed side table (`Layout`, below); `--js` (which genuinely mutates) will be the deliberate exception that has to say so.

`ComputedStyle` carries what visibility, generated content, and layout need: a computed `display` keyword (the seven-variant `Display` enum — author/inline rule, else UA-implicit by tag; `display:none` is the derived `display == None` query, not a stored bool), the flex `order` and `flex-direction` reorder keys, inherited `visibility`, and `::before`/`::after` content strings. `Display::parse` folds every non-layout value (`grid`, `table*`, `contents`, unknown) to `Block` once, at compute time — the layout §2 coercion.

**`Layout` (`src/layout.rs`)** — Phase 3 realized that extension pattern: a second `NodeId`-keyed side table computed by `layout::compute(doc, &Styles, viewport_w)`. It holds `boxes: Vec<Option<Rect>>` (parallel to the arena) plus `orders` for flex reading order. `Rect { x, y, w, h }` is integer px in viewport coordinates, y-down; a rendered element carries `Some(Rect)`, a `display:none`/non-rendered/non-element node carries `None`. `child_order(id)` returns a flex container's in-flow children in reading order — sorted by `(order, source index)`, reversed for a `*-reverse` `flex-direction` — and an empty `Vec` for any non-flex node. The box *kind* is never stored: it is `Styles::display`, the single source of truth. Width is the hard-coded `VIEWPORT_WIDTH = 1280` constant (no `--viewport` flag); with no real fonts, geometry uses fixed metrics (16px font, 8px glyph advance, 20px line-height), so boxes are structural estimates, not pixel truth.

## Load-bearing contracts

- **Envelope** (`src/envelope.rs`, version `"0"`): `{frot, url:{requested, final?}, view, status}` plus exactly one of `out` (status `ok`), `needs` (status `needs`), `error:{kind, message}` (status `error`). Fields are only ever **added**, never renamed. Error kinds are the closed `kinds::` taxonomy (`fetch.dns`, `fetch.tls`, …, `parse`, `internal`).
- **Exit codes**: `0` = `ok`/`needs` envelope, `1` = `error` envelope (still JSON on stdout), `2` = usage error (no envelope; message on stderr).
- **`needs` beats `ok`**: capability-gap detection runs before view production, only for views whose output depends on rendered content (`text`, `ax`, `links`, `forms` — not `dom`, not `meta`). The one detector so far is the SPA-shell heuristic (`src/needs.rs`): body text empty + document has `<script>` + ≤3 real element descendants under `<body>` → `needs: ["js"]`.
- **One view per invocation**; `--out` is mandatory, capabilities are orthogonal flags.

## Module map

| Path | Role |
|---|---|
| `src/cli.rs` | hand-rolled argv parser → `Args` |
| `src/fetch.rs` | blocking ureq GET; redirects, gzip/brotli, charset decode |
| `src/dom.rs` | html5ever → arena facade |
| `src/needs.rs` | capability-gap detection |
| `src/css/{parse,selparse,selector,cascade}.rs` | dependency-free CSS subset engine |
| `src/layout/{,flex,inline}.rs` | on-demand block/inline/flex geometry → `NodeId`-keyed `Layout` side table |
| `src/ax/{role,name,tree}.rs` | role mapping, accname subset, AX tree builder |
| `src/views/{text,dom,links,forms,meta,bboxes}.rs` | pure view functions |
| `src/tags.rs` | block-level tag set, shared by `views::text` and `layout` (single source of truth) |
| `src/envelope.rs` | output contract |
| `src/run.rs` | glue: pipeline + exit codes |

Every module has a colocated `tests.rs`; `tests/binary.rs` drives the compiled binary end-to-end against a mockito server. Gates: 100% line coverage, ≤300 lines per source file, clippy `-D warnings` (pre-commit).

## Fetch behavior

`ureq` 3 with rustls (no C TLS), Firefox-desktop User-Agent, 15 s global timeout, 16 MiB body cap, redirects followed (final URL reported in `url.final`), gzip/brotli decoding. Charset: `Content-Type` header, else `<meta charset>` sniff in the first 1 KiB, else UTF-8, decoded via `encoding_rs`. Non-2xx responses do not error at the transport layer (`http_status_as_error(false)`) — the body is still read — but `run.rs` surfaces the code in an `http` block and flips status ≥ 400 to an `error` envelope (`error.kind: http.<code>`), so a 404/500/bot-challenge body is never reported as a successful impression.

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

**Honest signals.** Phase 3 is block + inline + basic flex at a 1280px viewport with fixed font metrics; grid, floats, `position` offsets, and table-layout are coerced to in-flow block/inline (deterministic, documented — `docs/design/layout.md` §6), and box-model sizing, flex wrap/grow/shrink/justify/align/gap, and `@media` are out. There is deliberately **no `needs:["layout"]` signal**: unlike the SPA-shell heuristic, "uses grid/absolute" is not a clean binary — flow layout still yields *a* reading order and *approximate* boxes. Honesty is served instead by that published approximation contract; `NeedsKind::Layout` stays a reserved, dormant slot (as `NeedsKind::Css` is).

## Design decisions (log)

- **No `clap`** — argv parsing is ~120 hand-rolled lines; binary size wins (release binary ≈ 3.5 MB with `lto=fat`, `strip`, `panic=abort`).
- **No `cssparser`/`selectors`** — VISION named them, the implementation rejected them: the visibility/content need is narrow, and the dependency-escalation cost outweighed fidelity. Revisitable if the subset proves too thin.
- **Blocking I/O, no async runtime** — a single-shot process fetching a handful of resources gains nothing from tokio and pays startup + size for it.
- **RcDom absorbed, not wrapped** — rcdom's `Rc<RefCell<…>>` graph is copied once into the arena so the rest of the crate gets `Copy` node ids, cheap parallel side tables, and no interior mutability.
- **Views are pure functions** of `(Document, Option<&Styles>)` — capabilities compose by annotation, views by selection. No view knows which capabilities ran.
- **`-H` header passthrough, same-origin subfetch scope** (bl-f446, decided 2026-07-07) — repeatable curl-parity `-H "Name: value"`; caller's `User-Agent` replaces the default; `--css` sheet subfetches carry the headers only when same-origin with the page (scheme+host+port), else they go anonymous. `Authorization` never rides redirects (ureq default, not overridden). `-H` with `file://` is a usage error — under the same-origin rule the headers could never be sent, and an accepted flag must do something.
- **`file://` input rides the URL positional** (bl-ce6f, decided 2026-07-07) — no new flags, no bare-path sugar, no stdin (a stream has no base URL). A file read has no HTTP status (`FetchResult.status` is `Option`), no headers, and the same 16 MiB cap. Direction rule: local→remote is allowed (a saved page may fetch its CDN stylesheets under `--css`), remote→local is blocked (an http(s) page's `file:` stylesheet href is skipped — remote content never causes local reads).
- **Layout is an on-demand side table, not a flag** (Phase 3, bl-4cb4) — `layout::compute` mirrors the `Styles` pattern; the box *kind* is `Styles::display` (single source of truth), stored nowhere twice. The trigger lives in `run.rs` (bboxes always, ax under `--css`), keeping views pure functions of `(doc, styles?, layout?)`. Viewport is a hard-coded 1280px constant, not a flag — `bboxes`'s value is relative structure + reading order, largely width-insensitive, and a width knob is exactly the schema growth the non-goals discipline resists. Non-goals (grid/floats/positioning/tables, box-model sizing, fine flex) are coerced to flow and documented rather than signalled: no `needs:layout`, because coerced layout still yields a usable approximation (`docs/design/layout.md` §6).

## Known gaps (tracked in bl — run `bl ready`)

The 2026-07 arch pass filed a Phase 2.5 impression-fidelity epic (bl-2a9e) — HTTP status in the envelope (bl-d1c6), rejecting the unimplemented `--js`/`--out bboxes` surface (bl-957a), `nameFrom:contents` accname (bl-a864), layout-table demotion (bl-ab78) — which gated Phase 3 layout (bl-4cb4). Both have since **landed** (see "Where it stands" in `README.md`; `--out bboxes` is now a real view, not a usage error). `bl ready` is the live backlog.

## Open questions (decision tasks in bl)

- **`NeedsKind::Css` and `NeedsKind::Layout` are dormant** — the enum variants exist, nothing emits them. Reserved taxonomy slots, not dead code to delete: a future detector (content invisible without CSS? unrenderable without grid/absolute?) inhabits them, or a later review removes them. (Viewport convention — an earlier open question — is settled: the hard-coded 1280px constant, `docs/design/layout.md` §4.)
