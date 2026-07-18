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
