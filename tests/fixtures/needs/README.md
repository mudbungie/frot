# `needs/` — synthetic pages for the needs-js heuristic

Every file here is **hand-authored by this project**. No third-party markup,
prose, branding, or logo is reproduced — see `../NOTICE.md`. They carry frot's
own `MIT OR Apache-2.0`.

That is deliberate and it costs nothing. These fixtures test a *shape*
question — "does the body carry rendered content behind its static chrome?"
(`src/needs.rs`, `docs/design/needs.md` §4) — so the exact bytes never mattered;
only the structure does. The `js/` fixtures are the opposite case and are
verbatim for the opposite reason: they replay the exact bytes that broke a
run.

Each file is modelled on a real page's *structure*, recorded here so the
lineage of the shape is not lost. Naming a page as the model is a statement
about where an observation came from; none of its content is present.

| File | Verdict under test | Shape it models |
| --- | --- | --- |
| `todo-spa-shell.html` | `needs:["js"]` | A todo SPA: an empty mount `<section>` under a static `footer.info` — the TodoMVC deployments' layout |
| `canvas-app-shell.html` | `needs:["js"]` | A canvas app: a `<header>` masthead with an SEO `<h1>` around an empty `#root` — Excalidraw's landing shape |
| `chat-app-scaffold.html` | `needs:["js"]` | A messaging app: dozens of nested empty `<div>`s plus an offscreen `<defs>`-only SVG sprite sheet, zero text, zero labels. Modelled on Telegram Web's scaffold (field trial 2026-07-19, `bl-e22e`), the page that killed the retired element-count guard: big *and* empty, which that guard read as a rendered page |
| `encyclopedia-article.html` | `ok` | A reference-encyclopedia article: `h1` + prose + a see-also list, chrome above and below |
| `news-article.html` | `ok` | A newspaper article: `<article>` with a headline and three paragraphs |
| `link-aggregator-listing.html` | `ok` | A table-layout link listing — the `<table>`-as-layout shape that must collapse to `presentation` in the `ax` view |

Adding one? Keep it synthetic. If a test genuinely needs the real bytes, it
belongs in `js/` with a row in `../NOTICE.md` and a vendored licence.
