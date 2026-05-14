# frot

Take an impression of a web page — structure, text, accessibility tree — without rendering or executing it. Like a gravestone rubbing for the web.

`frot` is a stateless, single-binary tool for harness use. You give it a URL, a capability recipe, and one output view; it gives you back a machine-parseable envelope. It starts small (HTML-only impressions of server-rendered pages) and progressively burns down the render tree toward fuller capability.

## Output views

Each invocation returns exactly one view, selected with `--out`:

- `dom` — the parsed document tree, as JSON nodes (`element`, `text`, `comment`, `doctype`)
- `text` — readable content in source order; whitespace collapsed, `<pre>` preserved
- `links` — every `<a>`, `<area>`, and `<link>`, with absolute hrefs and `rel` tokens
- `forms` — `<form>` actions, methods, enctypes, and structured field lists
- `meta` — `<title>`, `<html lang>`, charset, canonical link, and `<meta>` entries
- `ax` — accessibility tree (planned for Phase 1)
- `bboxes` — element geometry (planned for Phase 3, when layout lands)

Every successful run emits the same envelope shape — `frot`, `url`, `view`, `status`, plus `out` (for `ok`), `needs` (for `needs`), or `error` (for `error`). The shape does not change as new capabilities ship.

## Capability flags

The capability recipe is orthogonal to the view: it picks what to do to the document before producing output.

- `--css` (Phase 2) — parse stylesheets, propagate visibility and generated content
- `--js` (Phase 4) — execute scripts against a partial DOM under bounded execution

Phase 0 ships neither flag. When a page can't be rendered faithfully under the current recipe — e.g. an empty SPA shell with `--js` off — frot will emit a `needs-X` envelope rather than a silently degraded `out`.

## Status

Phase 0 has landed: fetch + parse, the output envelope, and the `dom`/`text`/`links`/`forms`/`meta` views. Phase 1 (the `ax` view and the `needs-X` taxonomy) is queued; run `bl ready` for the live picture.

## Usage

```
frot <url> [--css] [--js] --out <dom|text|ax|links|forms|bboxes|meta>
```

Example:

```
frot https://example.com --out text
```

Exit codes: `0` for an `ok` or `needs` envelope, `1` for an `error` envelope (still emitted as JSON on stdout), `2` for a usage error (no envelope; message on stderr).

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
