# frot

Take an impression of a web page — structure, text, accessibility tree — without rendering or executing it. Like a gravestone rubbing for the web.

`frot` is a stateless, single-binary tool for harness use. You give it a URL and a parameter set; it gives you back machine-parseable output. It starts small (HTML-only impressions of server-rendered pages) and progressively burns down the render tree toward fuller capability.

## What it produces

- **DOM** — the parsed document tree
- **Text** — readable content, with semantic structure preserved
- **AX tree** — accessibility-tree simulacrum derived from semantic HTML + ARIA; the thing an agent actually wants to consume
- **Links / forms** — extracted, structured
- **Final URL** — after redirects

## What it doesn't do (yet)

- Execute JavaScript
- Apply CSS layout
- Render pixels

When a page demands those, frot returns a clear `needs-js` signal so callers can escalate to a heavier tool. The boundary is intentional — see [VISION.md](VISION.md) for the philosophy and the roadmap.

## Status

Pre-alpha. Scope is being burned down deliberately. See `bl ready` for what's queued.

## Usage (planned)

```
frot https://example.com --out ax,text,dom
```

## Building

```
make setup           # install dev tooling (cargo-llvm-cov)
make precommit-install
make build
make test
```

## License

TBD.
