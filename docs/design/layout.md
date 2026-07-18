# Layout engine design (Phase 3)

Living design for frot's on-demand layout. Scope-setting document: it decides
the data model, the display subset, the trigger, the viewport, the AX
refinement, and the non-goals *before* any layout code exists. It fits inside
`VISION.md` ("Phase 3 — Layout (on-demand)", principle 5 "honest signals",
principle 6 "elegance over completeness") and extends the as-built
side-table pattern in `ARCHITECTURE.md`. When code and this doc disagree, fix
whichever is wrong.

## The one-sentence shape

Layout is a *derivation*, not a capability: a `NodeId`-keyed side table of
per-element boxes, computed only when a view demands geometry, mirroring
`css::Styles` exactly — no DOM mutation, no `--layout` flag.

## 1. Data model — the layout side table

`Styles` (`src/css.rs`) is a `Vec<ComputedStyle>` parallel to the arena,
indexed by `NodeId`; views consult it, capabilities never mutate the DOM.
Layout is **another side table of the same shape** (`ARCHITECTURE.md`: "Phase 3
layout output should be another `NodeId`-indexed side table").

```
// src/layout/  (new module)
struct Rect { x: i32, y: i32, w: i32, h: i32 }   // px, viewport coords, y-down
struct Layout { boxes: Vec<Option<Rect>> }        // parallel to the arena
impl Layout {
    fn rect(&self, id: NodeId) -> Option<Rect>;   // None => no box generated
    fn child_order(&self, id: NodeId) -> Vec<NodeId>;  // reading order (§5)
}
```

- **One entry per `NodeId`.** A rendered element gets `Some(Rect)`; a
  `display:none` subtree, a non-element node, and a document node get `None`.
- **Border-box, integer px.** Boxes are content-touching (we parse no
  margin/border/padding widths — see §6). Integer px because sub-pixel
  precision is meaningless under our approximate metrics; machine-first output
  wants no float noise.
- **Anonymous boxes and line boxes are transient.** Real layout invents
  anonymous block boxes and line boxes; these have no `NodeId` and never appear
  in output, so they live only on the layout algorithm's stack, never in the
  table. The table stores exactly what a view can name: elements.
- **Reading order is the flex layout's output, not a redundant per-node index**
  (single source of truth): the CSS `order` value lives once in the cascade
  (`ComputedStyle::order`); flex layout materializes each container's reordered
  child sequence during `compute` into a `NodeId`-keyed side table that
  `child_order(id)` reads (§5). `Layout` retains no `doc`/`styles`, so the
  sequence is computed at layout time rather than re-derived per query. For any
  non-flex id (no stored order) `child_order` returns an empty `Vec` — the AX
  refinement (§5) calls it only for `Flex`/`InlineFlex` nodes.

Layout is a pure function of its inputs, like every other view/capability:

```
layout::compute(doc: &Document, styles: &Styles, viewport_w: i32) -> Layout
```

Note it takes `&Styles`, not `Option`: layout always needs a computed
`display` per element (§2). The trigger (§3) guarantees a `Styles` exists on
every layout path, even without `--css`.

## 2. Display values — computed vs coerced

Layout branches on each element's **computed `display`**, which the CSS module
must expose. Today `ComputedStyle` carries only `display_none: bool`; that is
too thin. Decision: **the cascade computes a full `display` keyword** (author
rule if any, else the element's UA-implicit display), and `display_none`
becomes the derived query `display == Display::None` (don't store what you can
compute).

```
enum Display { None, Block, Inline, InlineBlock, ListItem, Flex, InlineFlex }
```

**Implicit (UA) display by tag** seeds the value when no rule sets `display`.
Its block-level tag set is *the same list* `views::text.rs` already uses for
block breaks (`BLOCK_TAGS`) — extract it to one place so the two never drift
(single source of truth). `li` → `ListItem`; everything else → `Inline`.

**Computed by Phase 3 layout:**

| Display | Handling |
|---|---|
| `block`, `list-item`, `flow-root` | block flow (§ block) |
| `inline` | inline flow (§ inline) |
| `inline-block`, `inline-flex` | inline-level atomic box; contents laid out as block/flex |
| `flex` | flex container (§ flex) |
| `none` | no box; subtree skipped (already the `display_none` behavior) |

**Coerced (accepted, approximated as block flow):** every other value maps to
`Block` so the page still lays out, just without the specialised algorithm:

| Input | Coerced to | Because |
|---|---|---|
| `grid`, `inline-grid` | block | grid is a non-goal (§6) |
| `table`, `table-*` | block | table layout is a non-goal (§6) |
| `contents` | block | box-elision special-casing not worth it in P3 |
| unknown / unparsed | block | safe, in-flow default |

Coercion is deterministic and documented; it is not a silent failure (§6, honest
signals).

## 3. "On-demand" — the exact trigger

VISION: "Layout runs implicitly when something needs geometry … Outside that,
it is skipped entirely." There is no `--layout` flag. Mechanically, `run.rs`
`build_payload` decides per view whether to build a `Layout`:

| View | Builds layout? |
|---|---|
| `bboxes` | **always** — its output *is* geometry |
| `ax` | **only under `--css`** — see below |
| `dom`, `text`, `links`, `forms`, `meta` | never |

- **`bboxes`** always forces layout, computing a `Styles` first from whatever
  sheets are available (author + external under `--css`; UA-implicit + inline
  `style=` without it). So `--out bboxes` runs the cascade even without
  `--css`: geometry needs `display`, and `display` comes from the cascade. This
  also replaces the Phase-2.5 stub (bl-957a) that returns a usage error for
  `bboxes` until Phase 3 lands.
- **`ax` under `--css`** forces layout because CSS is Phase 3's *only* source of
  source-order ↔ reading-order divergence (flex `order`, `flex-direction:
  *-reverse`; floats/absolute are coerced to flow, §6, so they never reorder).
  Without `--css` no reorder is possible, source order is already correct, and
  layout is skipped — honest, and zero change to today's `ax` path.
- **JS reading geometry** (`offsetWidth`, …) is the third trigger named by
  VISION; it is **Phase 4**, out of scope here, noted so the mechanism composes:
  when JS forces a `Layout`, whatever view follows (e.g. `ax`) refines against
  it for free.

`build_payload` gains an `Option<&Layout>` alongside `Option<&Styles>`; views
stay pure functions of their inputs. No view knows *why* layout ran.

## 4. Viewport convention

- **Width: 1280px, hard-coded constant.** Load-bearing (inline flow line-breaks
  against it; block widths derive from it). 1280 is a common desktop breakpoint
  and ARCHITECTURE's own worked example.
- **Height: unbounded.** Phase 3 lays out the full document flow; there is no
  fold logic, so viewport height is not an input.
- **Not configurable.** No `--viewport`/`--width` flag. New flags are a smell,
  and a width knob is exactly the parameter-schema growth VISION's non-goals
  discipline warns against. The value of `bboxes` is *relative* structure and
  reading order, which are largely width-insensitive. The 1280 constant lives in
  the layout capability, deletable without touching the core (severability). See
  OQ-2 for when a width flag would be justified.

## 5. AX reading-order refinement

Today `ax::build` emits children in `entry.children` (source) order. Refinement:
when a `Layout` is present, a **flex container** emits its children in flex
reading order; every other container is unaffected (block/inline flow order ==
source order, and non-goals are coerced to flow).

- `ax::ax_tree`/`build` gain `layout: Option<&Layout>`. For a node whose
  computed display is `Flex`/`InlineFlex`, iterate `layout.child_order(id)`
  instead of `entry.children`. When `layout` is `None`, source order — the
  current behavior, bit-for-bit.
- **`child_order` rule** (the whole reorder surface in P3): sort a flex
  container's in-flow children by `(order value, source index)`; if
  `flex-direction` is `row-reverse`/`column-reverse`, reverse the result.
- **What changes in the `ax` view once layout has run:** children of flex
  containers appear in visual reading order instead of markup order. Nesting,
  roles, names, and levels are untouched — nodes are reordered among siblings,
  never moved across containers. So `--css --out ax` on a `order:`-shuffled or
  `row-reverse` toolbar reads in the order a user/AT would traverse it; without
  `--css`, and on all non-flex pages, output is identical to Phase 2.

## 6. Non-goals and the honesty story

Phase 3 is block + inline + basic flex, "enough to compute reading order and
reasonable bounding boxes" (VISION). Explicitly **out of scope**, each coerced to
in-flow block/inline so the page still lays out approximately:

- **Floats** (`float`) — laid out in normal flow; no float shapes, no text wrap
  around them.
- **Positioning** (`position: absolute|fixed|sticky|relative`) — laid out as
  `static`; `top/left/right/bottom` and relative offsets ignored.
- **CSS Grid** (`display:grid`) — coerced to block.
- **Tables as layout** (`display:table*`, the table sizing algorithm) — coerced
  to block; cells stack as blocks. (Complements, but is separate from, the AX
  layout-table demotion in Phase 2.5 / bl-ab78.)
- **Basic flex is basic:** `flex-wrap` (single-line only), `flex-grow/shrink/
  basis` fine resolution, `justify-content`/`align-*` precise placement, and
  `gap` are minimal or ignored. Flex computes ordering (§5) + crude main-axis
  placement, not exact item sizing.
- **Box model & typography:** no loaded fonts / real font metrics (see inline
  metrics below), no margin/border/padding widths, no `box-sizing`, no margin
  collapsing, no `overflow`/scroll, no `z-index` stacking, no `transform`/`zoom`,
  no multi-column, no `writing-mode`/vertical text, no bidi/RTL reordering.
- **`@media`** width/height/screen queries are evaluated against the fixed
  1280×720 viewport (`src/css/media.rs` — the same evaluator JS `matchMedia`
  delegates to), so responsive rules apply as a 1280px browser would; unknown
  features (`orientation`, `prefers-*`, …) conservatively never match.
  **`@supports`** (and other conditional groups) remain out of the CSS engine.

**Inline metrics** are a deliberate, documented approximation: no font is
loaded. Text advances at a fixed rate — default font-size 16px, ~0.5em average
glyph advance (8px), line-height 1.25 (20px) — as named constants, with greedy
word-boundary line-breaking against the viewport width. Boxes are therefore
**structural estimates, not pixel truth.**

**The needs/error story (VISION principle 5).** frot must never "silently
produce a degraded result that looks complete." For layout the honest posture is:

- **No `needs: ["layout"]` signal in Phase 3.** Unlike the SPA-shell heuristic
  (`needs-js` fires on a *detectably* empty impression), "this page uses grid /
  absolute positioning" is not a clean binary — flow layout still yields *a*
  reading order and *approximate* boxes. Emitting `needs-layout` on any coerced
  display would be noise. Honesty is instead served by (a) this document's
  stable, published approximation contract — `bboxes` is flow layout with
  estimated metrics, and it says so — and (b) a *reserved* `NeedsKind::Layout`
  slot, built only if a future detector can cleanly flag a page as
  unrenderable-without-grid/absolute (mirroring the dormant `NeedsKind::Css`).
  Recommended default: don't build the detector now (OQ-3).
- We do **not** add an envelope field counting coerced elements — that is
  schema growth for a fact the documented coercion table already makes
  deterministic. (Considered and rejected.)

## 7. `bboxes` output shape

`--out bboxes` returns a **flat array in reading order**, one entry per rendered
element that generates a box (`display:none` omitted; `visibility:hidden`
included — it occupies space and has geometry, though the `ax` view drops it):

```json
[
  { "i": 12, "tag": "h1", "rect": { "x": 0, "y": 0, "w": 1280, "h": 40 },
    "text": "Welcome" },
  { "i": 15, "tag": "p",  "rect": { "x": 0, "y": 40, "w": 1280, "h": 60 },
    "text": "Body copy…" }
]
```

- **Flat, not nested.** Nesting/structure is the `dom`/`ax` job; `bboxes` is
  geometry + order. A flat list is trivially sortable/filterable (machine-first).
- **`i`** = source-order index — the stable cross-view correlation handle back to
  `dom`/`ax` (arena `NodeId` is an internal detail; a source index is
  reproducible and view-agnostic).
- **`text`** = the element's own normalized direct text (or `null`), a
  lightweight legibility label so the list reads without cross-referencing.
- **Order** = the refined reading order (§5), so the array *is* the reading
  order plus geometry in one view.

## 8. Decomposition (proposed subtasks for bl-4cb4)

**Status: all eight shipped — Phase 3 landed (2026-07).**

Proposals only — the dispatcher files these; this doc does not run `bl create`.
Each is self-contained. All inherit bl-4cb4's gate (blocked on the Phase-2.5
epic bl-2a9e + this design bl-6077).

1. **Compute `display` in the cascade** — extend `css::ComputedStyle` with a
   `Display` enum (author rule ∨ UA implicit-by-tag), re-express `display_none`
   as the `== None` query, share the block-tag set with `views::text.rs`. No
   behavior change to existing views. *Dep: none — foundational.*
2. **Layout side table + box-tree scaffold** — new `src/layout/`: `Rect`,
   `Layout` (`NodeId`-keyed), the 1280px viewport constant, and the box-tree
   derivation that maps computed display → box kind and applies the §2/§6
   coercions. Placeholder zero rects. *Dep: 1.*
3. **Block flow geometry** — vertical stacking of block-level boxes; width from
   containing block, height from in-flow children; seeds the viewport width.
   *Dep: 2.*
4. **Inline flow + approximate text metrics** — greedy line-breaking with the
   fixed-advance constants (§6); inline box rects = union of line fragments;
   block heights driven by inline content. *Dep: 3.*
5. **Basic flex** — single-line main-axis placement along `flex-direction`,
   item ordering by `order`, crude content-based sizing; expose `child_order`.
   Wrap/grow/shrink/justify/align out. *Dep: 3 (content sizing benefits from 4).*
6. **`--out bboxes` view** — `src/views/bboxes.rs`: the §7 flat reading-order
   array; wire `run.rs` to always build `Layout` (+ a `Styles`) for `bboxes`,
   replacing the bl-957a usage-error stub. Golden-fixture tests. *Dep: 2–3 min;
   4–5 for fidelity.*
7. **AX reading-order refinement** — thread `Option<&Layout>` through
   `ax::ax_tree`/`build`; flex containers emit `child_order`; `run.rs` builds
   `Layout` for `ax` when `--css` is set. Unchanged when layout absent.
   *Dep: 5 + 1.*
8. **Docs/README fidelity pass** — mark Phase 3 landed; document the `bboxes`
   shape, the 1280 viewport constant, and the approximation/non-goal contract in
   README "Where it stands" + ARCHITECTURE (honest-signals). *Dep: 6, 7.*

## 9. Open questions (with recommended defaults)

- **OQ-1 — gate `ax`-triggers-layout on "page actually has a flex container"?**
  Recommended: **no.** Compute layout under `--css` unconditionally; the reorder
  is a no-op when no flex exists, and detecting flex first means computing
  display anyway. Over-triggering is a perf nuance, not a correctness/interface
  issue.
- **OQ-2 — ever add a viewport-width flag?** Recommended: **no**, until a
  concrete multi-width need (e.g. responsive-breakpoint testing) is filed. Ship
  the 1280 constant; a flag then is a justified capability, not speculative
  schema growth.
- **OQ-3 — a coarse `needs: ["layout"]` heuristic for grid/absolute-dependent
  pages?** Recommended: **no.** Reserve `NeedsKind::Layout` as a dormant slot
  (as `NeedsKind::Css` is today); build a detector only if one can flag
  unrenderable-without-X as cleanly as the SPA-shell heuristic does for JS.
