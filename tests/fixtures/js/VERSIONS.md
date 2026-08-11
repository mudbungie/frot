# Golden fixture framework bundles (pinned, vendored)

These are the real-world, unmodified framework distributions the golden fixture
suite (`docs/design/js.md` §9, bl-6358) drives end-to-end through `--js`. They
are **data, not dependencies** — served from an in-process mock server so tests
never touch the live network. Pins are exact; do not float them.

| File | Package | Version | Source |
| --- | --- | --- | --- |
| `react.production.min.js` | react | 17.0.2 | `https://unpkg.com/react@17.0.2/umd/react.production.min.js` |
| `react-dom.production.min.js` | react-dom | 17.0.2 | `https://unpkg.com/react-dom@17.0.2/umd/react-dom.production.min.js` |
| `vue.global.prod.js` | vue | 3.4.21 | `https://unpkg.com/vue@3.4.21/dist/vue.global.prod.js` |
| `jquery.min.js` | jquery | 3.7.1 | `https://code.jquery.com/jquery-3.7.1.min.js` |

React 17 (classic `ReactDOM.render`, not the React 18 `createRoot`/scheduler
path) and the Vue 3 *global* build (compiler included, UMD, no module graph) and
jQuery's full UMD build are all classic scripts — none is a module-graph build.

## Bundled application fixtures

React 19 dropped the official UMD builds the 17.0.2 pin uses, so the modern-React
capstone (bl-4640) can't be a single `curl`ed distribution — it's a tiny genuine
todo app bundled *with* its React runtime into one classic IIFE.

| File | Package(s) | Version | Build |
| --- | --- | --- | --- |
| `react19-todo.bundle.js` | react + react-dom | 19.1.1 | esbuild 0.25.5, see below |
| `react19-todo.src.js` | (app source) | — | the readable input to the bundle |

`react19-todo.src.js` is the app: a controlled `<input>` + a seeded list mounted
via `ReactDOM.createRoot(...).render(...)` (the React 18/19 concurrent path, NOT
17's `ReactDOM.render`) into an empty `<div id='root'>`. Its init reads
`history.state` and `new URL(location.href).searchParams` — the SPA-router
surface the field trial's React 19 todomvc crashed on. It is vendored only so the
187 KB minified bundle stays auditable and reproducible; the golden test drives
the **bundle**. Reproduce (react/react-dom pinned to 19.1.1):

```
esbuild react19-todo.src.js --bundle --minify --format=iife \
  --define:process.env.NODE_ENV='"production"' --outfile=react19-todo.bundle.js
```

## Hand-authored fixtures

| File | Purpose |
| --- | --- |
| `esm-greeter.mjs` | A tiny, framework-free ES module the golden suite's module page imports by relative URL — exercises the as-built ESM path (resolver + loader over the §6 subfetch cache, live module linking; `js.md` §4.1/§6, bl-1b98). Data, not a dependency; hand-authored, so it carries no upstream version. |

## Field-trial fixtures (verbatim live pages)

Pages pinned byte-for-byte from live deployments the 2026-08-10 field trial
broke frot against, driven offline by `src/run/golden_field_tests.rs`. Carrying
no package version, they are pinned by source URL, retrieval date, and sha256 —
do not re-fetch; the point is replaying the exact bytes that failed.

Retrieved 2026-08-10 from `https://todomvc.com/examples/javascript-es6/dist/`
(the TodoMVC "JavaScript ES6 Webpack" deployment whose bundle runs
`NodeList.prototype.forEach = Array.prototype.forEach` at init, bl-e5c3):

| File | Live path | sha256 |
| --- | --- | --- |
| `todomvc-es6.html` | `index.html` | `3f5e2a1f370e7326c9523fdbaf07a3d98ab11e5fb0dd243ff83d276559f1d87a` |
| `todomvc-es6.bundle.js` | `app.bundle.js` | `01b56caf970328499b1ea12a405bd4c03e27bc4bad6d6e36d49884fe75159fac` |
| `todomvc-es6.base.js` | `base.js` | `12d217a42e7349e522ee100e833b734471aa2d14823defa5e1bd77802cf67a9d` |

Retrieved 2026-08-10 from `https://todomvc.com/examples/react/dist/` (the
TodoMVC "React" deployment whose React 18-era commit phase runs `autoFocus &&
stateNode.focus()` on the new-todo input, bl-3a36 — absent `Element#focus`,
React catches the throw and unmounts the root; its base.js differs byte-wise
from the es6 deployment's, so it is pinned separately):

| File | Live path | sha256 |
| --- | --- | --- |
| `react-todomvc.html` | `index.html` | `23b40a8b71f26e44f3e72e682188a3ab238d17fd375ff0ec06295af9971fa959` |
| `react-todomvc.bundle.js` | `app.bundle.js` | `6197ad9358985fb3f745aef3fca9abbe2fc7f0cd35cd4525cb8570107ae6b78a` |
| `react-todomvc.base.js` | `base.js` | `8cfbaa8d2bc03e2e52a8b7788e041efda231a17e2a25c8bc4bfe2659adc5bb90` |

Retrieved 2026-08-10 from `https://vite-react-tailwind-template.pages.dev/`
(the field trial's independent React reproduction, bl-3a36). The page is
verbatim; the module fixture is **derived, not verbatim**: the failing unit is
the `vite:build-import-analysis` modulepreload polyfill every Vite production
build inlines at module top level (`link.relList.supports("modulepreload")`,
falling through to `new MutationObserver` when relList is absent), so
`vite-preload.mjs` is that polyfill IIFE cut byte-for-byte from the deployed
`/assets/index-D22riwjH.js` (bundle sha256
`bf1e9611b4339cabd44f7681e3909ce308a3bc641ac2477f3b2c3df4c1ec1bdf`, polyfill
slice sha256
`1360fef05150c30f23bb0641a7cde33d78e3bf153a7e93ee1cf27f16444ac3ee`) plus one
appended render line so the outcome detector has app text to see. The remaining
~577 KB of that bundle is React 19 + the app, whose surface the react
fixtures above already pin.

| File | Source | sha256 |
| --- | --- | --- |
| `vite-preload.html` | `index.html`, verbatim | `590b4162216823aa2067ba712ad0689b88e3f5db18f5692b7c7098580de967ce` |
| `vite-preload.mjs` | derived (see above) | `d0bfe58d113b5ad5c8d1765b715fee7c9021aa96efc869ca46ef975fbffc962b` |
