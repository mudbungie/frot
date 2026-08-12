# Third-party notices for `tests/fixtures/`

frot itself is `MIT OR Apache-2.0`. **Nothing under this directory is.** These
files are *data* — read only by `#[cfg(test)]` code, never compiled into the
library or the binary, and excluded from the published crate
(`Cargo.toml: exclude = ["tests/fixtures/"]`). They are redistributed here under
their own upstream licenses, which are reproduced verbatim in `licenses/`.

Two populations, two rules.

| Directory | What it holds | Rule |
| --- | --- | --- |
| `needs/` | Shells and content pages for the `needs-js` heuristic | **Synthetic.** Hand-authored by this project, no third-party content — see `needs/README.md`. |
| `js/` | Real framework distributions and real deployed bundles | **Verbatim third-party.** Licensed below. |

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
| `js/env-contract.html`, `js/persona-navigator.html` | Hand-authored here | frot's own `MIT OR Apache-2.0` |
| `js/dom-move.html` | Hand-authored here — a minimized DOM-move reproduction; it models the *pattern* a live page hit, and reproduces none of its markup (`VERSIONS.md`) | frot's own `MIT OR Apache-2.0` |
| `needs/*.html` | Hand-authored here (`needs/README.md`) | frot's own `MIT OR Apache-2.0` |

---

## Keeping this honest

`js/VERSIONS.md` is the single source of truth for *what* each fixture is and
where it came from; this file adds only the licence layer over those same rows.
Adding a fixture to `js/` means adding its row here **and** vendoring its licence
text into `licenses/` — a third-party file with no row is a compliance bug, not a
formatting one. Prefer a synthetic fixture in `needs/` whenever the test is about
shape rather than exact bytes.
