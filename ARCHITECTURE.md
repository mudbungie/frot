# Architecture (as-built)

What the code actually is, as of the 2026-07 arch pass. `VISION.md` says why and where; this says how. When they disagree, the code wins — fix whichever doc is wrong.

## The pipeline

One process, one pass, one envelope:

```
argv ──cli::parse──► Args { url, css, js, out }
url ──fetch::fetch──► FetchResult { final_url, status, headers, body, charset }
body ──dom::Document::parse──► Document (arena)
(view, doc) ──needs::detect──► [NeedsKind]     — non-empty short-circuits to a `needs` envelope
(doc, sheets) ──css::compute_with──► Styles    — only when --css; sheets = <style> + style= + fetched <link>
(doc, styles) ──views::* / ax::ax_tree──► JSON payload
payload ──Envelope──► one JSON line on stdout + exit code
```

Orchestration lives in `src/run.rs` (~100 lines); everything below it is a pure function of its inputs. There is no global state anywhere — statelessness is enforced by signature.

## Core data structures

**`Document` (`src/dom.rs`)** — html5ever parses into `RcDom`, which is immediately *absorbed* into a flat `Vec<NodeEntry>` arena indexed by `NodeId` (`u32`). The facade exposes `walk` (preorder `Enter`/`Exit` events), `find_by_tag`, `text_content`; no `markup5ever` type leaks past this module, so parser/storage can be swapped without touching views. Tag and attribute names are lowercased at absorption.

**`Styles` (`src/css.rs`)** — a `Vec<ComputedStyle>` parallel to the arena, indexed by the same `NodeId`. Capabilities do not mutate the DOM; they produce side tables that views consult. **This is the extension pattern**: Phase 3 layout output should be another `NodeId`-indexed side table, and `--js` (which genuinely mutates) is the deliberate exception that will have to say so.

`ComputedStyle` carries exactly what visibility and generated content need: `display_none`, inherited `visibility`, `::before`/`::after` content strings. Nothing else is computed.

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
| `src/ax/{role,name,tree}.rs` | role mapping, accname subset, AX tree builder |
| `src/views/{text,dom,links,forms,meta}.rs` | pure view functions |
| `src/envelope.rs` | output contract |
| `src/run.rs` | glue: pipeline + exit codes |

Every module has a colocated `tests.rs`; `tests/binary.rs` drives the compiled binary end-to-end against a mockito server. Gates: 100% line coverage, ≤300 lines per source file, clippy `-D warnings` (pre-commit).

## Fetch behavior

`ureq` 3 with rustls (no C TLS), Firefox-desktop User-Agent, 15 s global timeout, 16 MiB body cap, redirects followed (final URL reported in `url.final`), gzip/brotli decoding. Charset: `Content-Type` header, else `<meta charset>` sniff in the first 1 KiB, else UTF-8, decoded via `encoding_rs`. Non-2xx responses are **not** errors (`http_status_as_error(false)`) — the body is still the impression; surfacing the status code in the envelope is bl-d1c6 (see gaps).

Under `--css`, external `<link rel=stylesheet>` hrefs are resolved against the final URL and fetched best-effort — a failed sheet is skipped, never fatal (CSS is an enhancement to the impression, not a precondition).

`file://` URLs take the read path instead: `std::fs` read, same size cap, charset from the `<meta>` sniff (there is no Content-Type), `status: None`, empty headers. Read failures map to the additive error kind `fetch.file`.

## Design decisions (log)

- **No `clap`** — argv parsing is ~120 hand-rolled lines; binary size wins (release binary ≈ 3.5 MB with `lto=fat`, `strip`, `panic=abort`).
- **No `cssparser`/`selectors`** — VISION named them, the implementation rejected them: the visibility/content need is narrow, and the dependency-escalation cost outweighed fidelity. Revisitable if the subset proves too thin.
- **Blocking I/O, no async runtime** — a single-shot process fetching a handful of resources gains nothing from tokio and pays startup + size for it.
- **RcDom absorbed, not wrapped** — rcdom's `Rc<RefCell<…>>` graph is copied once into the arena so the rest of the crate gets `Copy` node ids, cheap parallel side tables, and no interior mutability.
- **Views are pure functions** of `(Document, Option<&Styles>)` — capabilities compose by annotation, views by selection. No view knows which capabilities ran.
- **`-H` header passthrough, same-origin subfetch scope** (bl-f446, decided 2026-07-07) — repeatable curl-parity `-H "Name: value"`; caller's `User-Agent` replaces the default; `--css` sheet subfetches carry the headers only when same-origin with the page (scheme+host+port), else they go anonymous. `Authorization` never rides redirects (ureq default, not overridden). `-H` with `file://` is a usage error — under the same-origin rule the headers could never be sent, and an accepted flag must do something.
- **`file://` input rides the URL positional** (bl-ce6f, decided 2026-07-07) — no new flags, no bare-path sugar, no stdin (a stream has no base URL). A file read has no HTTP status (`FetchResult.status` is `Option`), no headers, and the same 16 MiB cap. Direction rule: local→remote is allowed (a saved page may fetch its CDN stylesheets under `--css`), remote→local is blocked (an http(s) page's `file:` stylesheet href is skipped — remote content never causes local reads).

## Known gaps (tracked in bl — run `bl ready`)

| Task | Gap |
|---|---|
| bl-d1c6 | HTTP status fetched but dropped from the envelope; 404/500/bot-challenge bodies report `status:"ok"`, exit 0 |
| bl-957a | `--js` parses but is never read (silent no-op); `--out bboxes` errors as kind `internal`/exit 1 instead of usage/exit 2 |
| bl-a864 | accname falls through to full-subtree text for every non-form element; containers (`table`, `rowgroup`, …) carry the whole page text as their name |
| bl-ab78 | no layout-table demotion: caption-less layout tables keep full `table` AX structure (noise browsers suppress) |

These four are the Phase 2.5 fidelity epic (bl-2a9e), which gates Phase 3 (bl-4cb4).

## Open questions (decision tasks in bl)

- **`NeedsKind::Css` is dormant** — the enum variant exists, nothing emits it. Reserved taxonomy slot, not dead code to delete: a future detector (content invisible without CSS?) inhabits it or Phase 5 review removes it.
- **Layout viewport convention** — `bboxes` needs a width; settle it in the Phase 3 design doc (bl-6077) before layout code exists.
