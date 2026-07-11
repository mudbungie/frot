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
jQuery's full UMD build are all classic scripts — none is a module-graph build,
so the ES-module-import demotion (`js.md` §4.1 / bl-1b98) does not apply.
