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
