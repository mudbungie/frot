# Third-party notices for `tests/fixtures/`

frot itself is `MIT OR Apache-2.0`. **Nothing under this directory is.** These
files are *data* — read only by `#[cfg(test)]` code, never compiled into the
library or the binary, and excluded from the published crate
(`Cargo.toml: exclude = ["tests/fixtures/"]`). They are redistributed here under
their own upstream licenses, which are reproduced verbatim in `licenses/`.

Three populations, three rules.

| Directory | What it holds | Rule |
| --- | --- | --- |
| `needs/` | Shells and content pages for the `needs-js` heuristic | **Synthetic.** Hand-authored by this project, no third-party content — see `needs/README.md`. |
| `js/` | Real framework distributions and real deployed bundles | **Verbatim third-party.** Licensed below. |
| `fidelity/` | Pages for the fidelity oracle, and a browser's recorded answers about them | **Synthetic.** Hand-authored by this project; each `.json` beside a page is what a named Chrome version answered about that page (`scripts/fidelity-record`) — measurements of our own markup, no third-party content. |
| `fidelity/saved/` | Real pages saved for the fidelity oracle, with their stylesheets, and the recorded answers about them | **Third-party, saved with the listed edits.** Licensed in §4 below. |

`js/` is verbatim on purpose: the golden field-trial suite replays the exact
bytes that broke frot (`js/VERSIONS.md` pins every file by source URL, retrieval
date, and sha256). A synthetic stand-in would not reproduce the failures, so
these are licensed rather than replaced. Two files are *derived* rather than
verbatim; both are marked below and in `VERSIONS.md`.

---

## 1. Framework distributions

Unmodified, byte-for-byte as published. Each carries its own upstream `@license`
banner inline, preserved here.

| Fixture | Upstream | Version | License | Text |
| --- | --- | --- | --- | --- |
| `js/react.production.min.js` | [facebook/react](https://github.com/facebook/react) | 17.0.2 | MIT — © Facebook, Inc. and its affiliates | `licenses/react-17.LICENSE` |
| `js/react-dom.production.min.js` | facebook/react | 17.0.2 | MIT — © Facebook, Inc. and its affiliates | `licenses/react-17.LICENSE` |
| `js/vue.global.prod.js` | [vuejs/core](https://github.com/vuejs/core) | 3.4.21 | MIT — © 2018-present, Yuxi (Evan) You and Vue contributors | `licenses/vue-3.4.21.LICENSE` |
| `js/jquery.min.js` | [jquery/jquery](https://github.com/jquery/jquery) | 3.7.1 | MIT — © OpenJS Foundation and other contributors | `licenses/jquery-3.7.1.LICENSE` |

## 2. Deployed pages and their bundles

Retrieved from live deployments on the dates `js/VERSIONS.md` records, verbatim
unless the Modifications column says otherwise. A production bundle contains its
dependencies, so each row licenses the *app* and the runtimes compiled into it.

### 2.1 TodoMVC (`todomvc.com`)

App code: **MIT** — © Addy Osmani, Sindre Sorhus, Pascal Hartig, Stephen Sawchuk
([tastejs/todomvc](https://github.com/tastejs/todomvc)), `licenses/todomvc.LICENSE`.
It covers `todomvc-common`'s `base.js` in all three rows.

| Fixture | Deployment | Also contains | Modifications |
| --- | --- | --- | --- |
| `js/todomvc-es6.html`, `js/todomvc-es6.bundle.js`, `js/todomvc-es6.base.js` | `/examples/javascript-es6/dist/` | — (framework-free ES6 + webpack runtime) | none — verbatim |
| `js/react-todomvc.html`, `js/react-todomvc.bundle.js`, `js/react-todomvc.base.js` | `/examples/react/dist/` | React 18-era react/react-dom/scheduler/jsx-runtime (MIT, © Meta Platforms, Inc. and affiliates, `licenses/react-19.LICENSE` — same text and copyright line as the 19.x tree) and `classnames` (MIT, © 2018 Jed Watson) | none — verbatim |
| `js/vue-todomvc.html`, `js/vue-todomvc.bundle.js`, `js/vue-todomvc.base.js` | `/examples/vue/dist/` | Vue 3.5 (MIT, `licenses/vue-3.4.21.LICENSE` — unchanged text across 3.x) and vue-router 4 (MIT, © 2019-present Eduardo San Martin Morote, `licenses/vue-router.LICENSE`) | none — verbatim |

**`js/react-todomvc.bundle.js.LICENSE.txt`** is the upstream file the bundle's
first line points at (`/*! For license information please see
app.bundle.js.LICENSE.txt */`) — the notices webpack stripped out of the code.
It was missing until 2026-08-11, which made that first line a dangling
reference and the bundle's own attribution unreadable. Retrieved
2026-08-11 from `https://todomvc.com/examples/react/dist/app.bundle.js.LICENSE.txt`,
verbatim, sha256 `2fde10fedaa9ebac195ff484ddb08969f5d04ab284ba65000b3cbf0992703640`;
only the filename is prefixed, to match the renamed bundle it belongs to. The
deployed `app.bundle.js` was re-hashed on that date and still matched the
pinned fixture byte-for-byte, so this sidecar is the one that bundle points at,
not a later revision of it. The Vite-built deployments (`vue-todomvc`,
`vite-react-tailwind`) emit no such sidecar — their notices are the inline
banners in the bundle, or this file.

### 2.2 `vite-react-tailwind-template.pages.dev`

App code: **MIT** — © 2025 Innei
([innei-template/smart-webapp-template](https://github.com/innei-template/smart-webapp-template)),
`licenses/innei-template.LICENSE`.

| Fixture | Live path | Contains | Modifications |
| --- | --- | --- | --- |
| `js/vite-preload.html` | `index.html` | — | none — verbatim |
| `js/vite-react-tailwind.bundle.js` | `assets/index-D22riwjH.js` | React 19 + react-dom + scheduler + jsx-runtime (MIT, © Meta Platforms, Inc. and affiliates), react-router 7.6.2 (MIT, © React Training LLC / Remix Software Inc., `licenses/react-router.LICENSE`), Radix primitives (MIT, © 2022 WorkOS, `licenses/radix.LICENSE`), lucide-react 0.518.0 (**ISC**, © Lucide Icons and Contributors, `licenses/lucide.LICENSE`) | none — verbatim; the 26 upstream `@license` banners are preserved in the file |
| `js/vite-react-tailwind.framer.js` | `assets/framer-lazy-feature-Cs1hsT7r.js` | framer-motion / Motion (MIT, © Motion B.V., `licenses/framer-motion.LICENSE`) | none — verbatim |
| `js/vite-preload.mjs` | derived from `assets/index-D22riwjH.js` | Vite's `vite:build-import-analysis` modulepreload polyfill (MIT, [vitejs/vite](https://github.com/vitejs/vite)) | **DERIVED, NOT VERBATIM** — the polyfill IIFE cut byte-for-byte out of the bundle above, plus one render line appended by this project so the outcome detector has app text to see (`VERSIONS.md`) |

## 3. Built by this project

| Fixture | Origin | License |
| --- | --- | --- |
| `js/react19-todo.src.js` | Hand-authored here | frot's own `MIT OR Apache-2.0` |
| `js/react19-todo.bundle.js` | **DERIVED** — `react19-todo.src.js` bundled with esbuild 0.25.5 (recipe in `VERSIONS.md`) | app code as above; the react + react-dom 19.1.1 runtime it embeds is MIT, © Meta Platforms, Inc. and affiliates, `licenses/react-19.LICENSE` |
| `js/esm-greeter.mjs` | Hand-authored here | frot's own `MIT OR Apache-2.0` |
| `js/env-contract.html`, `js/persona-navigator.html`, `js/script-mode.html` | Hand-authored here | frot's own `MIT OR Apache-2.0` |
| `js/dom-move.html` | Hand-authored here — a minimized DOM-move reproduction; it models the *pattern* a live page hit, and reproduces none of its markup (`VERSIONS.md`) | frot's own `MIT OR Apache-2.0` |
| `js/dom-interfaces.json` | **DERIVED** — a measurement recorded by this project: the DOM interface names, member names and descriptor shapes a Firefox 153.0esr binary reports for its own objects (`docs/design/identity.md` §3.21). Names and facts only; no Mozilla source text | frot's own `MIT OR Apache-2.0` |
| `js/streams-probe.js` | Hand-authored here — the calls whose Firefox answers `streams-firefox153.json` records (`docs/design/js.md` §16) | frot's own `MIT OR Apache-2.0` |
| `js/streams-firefox153.json` | **DERIVED** — a measurement recorded by this project: what Firefox 153.0esr answers to each call in `streams-probe.js` (`docs/design/js.md` §16). Values and messages only; no Mozilla source text | frot's own `MIT OR Apache-2.0` |
| `js/script-probe.js` | Hand-authored here — the calls whose Firefox answers `script-firefox153.json` records (`docs/design/js.md` §4.7) | frot's own `MIT OR Apache-2.0` |
| `js/script-firefox153.json` | **DERIVED** — a measurement recorded by this project: what Firefox 153.0esr answers to each call in `script-probe.js` (`docs/design/js.md` §4.7). Values and messages only; no Mozilla source text | frot's own `MIT OR Apache-2.0` |
| `js/perf-probe.js` | Hand-authored here — the calls whose Firefox answers `perf-firefox153.json` records (`docs/design/js.md` §5, "Resource timing") | frot's own `MIT OR Apache-2.0` |
| `js/perf-firefox153.json` | **DERIVED** — a measurement recorded by this project: what Firefox 153.0esr answers to each call in `perf-probe.js` (`docs/design/js.md` §5, "Resource timing"). Values and messages only; no Mozilla source text | frot's own `MIT OR Apache-2.0` |
| `needs/*.html` | Hand-authored here (`needs/README.md`) | frot's own `MIT OR Apache-2.0` |
| `fidelity/*.html`, `fidelity/*.css`, `fidelity/*.js` | Hand-authored here, one page per defect class (a `.js` is a script a page loads) (`docs/design/fidelity.md` §4) | frot's own `MIT OR Apache-2.0` |
| `fidelity/*.json` | **DERIVED** — recorded from the pages above by `scripts/fidelity-record`; the stamp in each names the Chrome version and date | frot's own `MIT OR Apache-2.0` |
| `fidelity/saved/*.json` | **DERIVED** — recorded the same way from the saved real pages of §4 | the licence of the page each one sits beside (§4) |

## 4. Saved real pages (`fidelity/saved/`)

Real pages, saved once so that frot and Chrome read the same bytes with no
network (`docs/design/fidelity.md` §4). Each was chosen because every byte kept
is under a licence that allows redistribution with attribution and nothing
more. The first four were retrieved on 2026-11-21, the W3C page on
2026-11-22. The sha256 of every saved file is
in the `stamp.bytes` of the capture beside the page, and `tests/fidelity.rs`
re-hashes them.

**Edits made to every page**, and nothing else — no element is removed except
`<script>`, no text is changed:

- every `<script>` element is removed (the pages are compared without `--js`);
- each stylesheet `href` points at the saved copy in the page's own directory
  (a cache-busting `?v=…` query goes with the old address), and
  `integrity`/`crossorigin` are dropped from that `<link>`; an `@import`
  between two saved sheets is kept as written;
- no image, icon or font is redistributed: every `<img src>`, every
  `<link rel=icon|apple-touch-icon|mask-icon|manifest|preconnect>` `href`, and
  every non-`data:` `url()` in a stylesheet or `<style>` is replaced by one
  empty inline SVG (`data:image/svg+xml,…`), and `srcset` is removed. The
  elements, their `alt` text and their sizes stay.

| Fixture | Source | Licence and attribution | Further edits |
| --- | --- | --- | --- |
| `fidelity/saved/go-getting-started.html` | `https://go.dev/doc/tutorial/getting-started` | **CC BY 4.0**, `licenses/CC-BY-4.0.txt` — © The Go Authors. go.dev: "Except as noted, the contents of this site are licensed under the Creative Commons Attribution 4.0 License, and code is licensed under a BSD license." | the `<link>` to the Google Fonts "Material Icons" stylesheet is removed (not ours to copy) |
| `fidelity/saved/go-getting-started/styles.css` | `https://go.dev/css/styles.css` | **BSD-3-Clause** — © The Go Authors ([golang/website](https://github.com/golang/website)), `licenses/go-website.LICENSE` | 9 `url()` image references replaced as above; otherwise verbatim |
| `fidelity/saved/bootstrap-forms.html` | `https://getbootstrap.com/docs/5.3/forms/overview/` (v5.3.9) | **CC BY 3.0**, `licenses/CC-BY-3.0.txt` — © The Bootstrap Authors. Page footer: "Code licensed MIT, docs CC BY 3.0." | none beyond the common edits |
| `fidelity/saved/bootstrap-forms/bootstrap.min.css` | `https://getbootstrap.com/docs/5.3/dist/css/bootstrap.min.css` (v5.3.9) | **MIT** — © 2011-2026 The Bootstrap Authors ([twbs/bootstrap](https://github.com/twbs/bootstrap)), `licenses/bootstrap-5.3.LICENSE`; banner preserved | none — verbatim (its images are already `data:` URIs) |
| `fidelity/saved/bootstrap-forms/docs.css` | `https://getbootstrap.com/docs/5.3/assets/_slug_.ZhhCqGDD.css` | **CC BY 3.0**, `licenses/CC-BY-3.0.txt` — © 2011-2026 The Bootstrap Authors; banner preserved | none — verbatim; renamed |
| `fidelity/saved/wikipedia-brass-rubbing.html` | English Wikipedia, "Brass rubbing", revision 1308487280 — `https://en.wikipedia.org/w/index.php?title=Brass_rubbing&oldid=1308487280` (fetched as `action=render`, legacy parser) | **CC BY-SA 4.0**, `licenses/CC-BY-SA-4.0.txt` — Wikipedia contributors; authors are listed at `https://en.wikipedia.org/w/index.php?title=Brass_rubbing&action=history`. This modified copy is offered under the same licence. | the rendered article body is wrapped in a minimal document (`<!doctype>`, `<title>`, and a `<base>` at the article's URL so its protocol-relative links resolve as they do on the site). The site skin and its stylesheet (MediaWiki, GPL-2.0-or-later) are **not** included; the article's own inline TemplateStyles are |
| `fidelity/saved/python-webbrowser.html` | `https://docs.python.org/3.13/library/webbrowser.html` (Python 3.13.18 documentation) | **PSF-2.0**, and **0BSD** for the code examples — © 2001-2026 Python Software Foundation. Page footer: "This page is licensed under the Python Software Foundation License Version 2. Examples, recipes, and other code in the documentation are additionally licensed under the Zero Clause BSD License." Both texts are in `licenses/cpython-3.13.LICENSE` (CPython's `LICENSE`, verbatim) | none beyond the common edits (11 `<script>` elements removed; its `../` links and form actions are left pointing where they pointed) |
| `fidelity/saved/python-webbrowser/classic.css`, `fidelity/saved/python-webbrowser/basic.css` | `https://docs.python.org/3.13/_static/classic.css`, `…/basic.css` | **BSD-2-Clause** — © 2007-2026 the Sphinx developers ([sphinx-doc/sphinx](https://github.com/sphinx-doc/sphinx)), `licenses/sphinx.LICENSE` | none — verbatim; `classic.css` reaches `basic.css` through its `@import` |
| `fidelity/saved/python-webbrowser/pygments.css`, `fidelity/saved/python-webbrowser/pygments_dark.css` | `https://docs.python.org/3.13/_static/pygments.css`, `…/pygments_dark.css` | **BSD-2-Clause** — © 2006-2022 the Pygments authors ([pygments/pygments](https://github.com/pygments/pygments)), `licenses/pygments.LICENSE`: the highlighting styles Pygments emits, written out by the docs build | none — verbatim |
| `fidelity/saved/python-webbrowser/pydoctheme.css`, `fidelity/saved/python-webbrowser/pydoctheme_dark.css` | `https://docs.python.org/3.13/_static/pydoctheme.css`, `…/pydoctheme_dark.css` | **PSF-2.0** — © Python Software Foundation ([python/python-docs-theme](https://github.com/python/python-docs-theme)), `licenses/python-docs-theme.LICENSE` | `pydoctheme.css`: 1 `url()` font reference replaced as above; `pydoctheme_dark.css`: none — verbatim |
| `fidelity/saved/web-share.html` | `https://www.w3.org/TR/web-share/` (Web Share API, W3C Recommendation, dated version `https://www.w3.org/TR/2023/REC-web-share-20230530/`) | **W3C Software and Document License** (2015), `licenses/w3c-software-and-document-2015.LICENSE` — Copyright © 2023 World Wide Web Consortium. Page footer: "W3C liability, trademark and permissive document license rules apply." This copy includes material copied from that document, modified as listed. | 3 `<script>` elements removed; the W3C logo `<img>` (W3C logos are not under the licence) and the 8 browser-logo `url()`s in the page's `<style>` replaced as above, its one `data:` GIF kept; the `crossorigin` on the logo dropped |
| `fidelity/saved/web-share/W3C-REC.css`, `fidelity/saved/web-share/base.css` | `https://www.w3.org/StyleSheets/TR/2021/W3C-REC`, `…/base.css` | **W3C Software Notice and License** (2002) — W3C ([w3c/tr-design](https://github.com/w3c/tr-design)), `licenses/w3c-tr-design.LICENSE` (the repository's `LICENSE` and the licence text it names) | `W3C-REC` saved as `W3C-REC.css` (the recorder types files by extension), reaching `base.css` through its `@import`; their 3 `url()` logo references (1 + 2) replaced as above; otherwise verbatim |

The `.json` beside each saved page is derived (§3), and it quotes the page:
it reproduces the page's text and structure as Chrome reported them, so it is
offered under that page's licence, not frot's.

The W3C page was tried twice and left out (223 lines from an absolutely
positioned box in `text` and `text-transform`, both missing then); it went in
once `bl-346e` and `bl-e48d` landed, with no ledger row (`bl-3248`). MDN and the full
Wikipedia skin were not tried: their site stylesheets are MPL-2.0 and
GPL-2.0-or-later, which ask for more than attribution.

---

## Keeping this honest

`js/VERSIONS.md` is the single source of truth for *what* each fixture is and
where it came from; this file adds only the licence layer over those same rows.
Adding a fixture to `js/` means adding its row here **and** vendoring its licence
text into `licenses/` — a third-party file with no row is a compliance bug, not a
formatting one. Prefer a synthetic fixture in `needs/` whenever the test is about
shape rather than exact bytes.

`tests/hygiene/licensing.rs` holds both ends. Every tracked file under
`tests/fixtures/` must be named in this file — by its own path, or by a
`dir/*.ext` row for a family this project authored, which covers that directory
only and never a subdirectory. And no tracked file *outside* `tests/fixtures/`
may carry a licence banner other than frot's own two `LICENSE-*` texts: this
directory is the only home for someone else's bytes. A React bundle vendored
under `src/` sat outside the old one-directory sweep, with no row here, and
shipped in the published crate (`bl-41e5`); a test under `src/` that needs such
a file reads it from here (`include_str!("…/tests/fixtures/js/…")`).
