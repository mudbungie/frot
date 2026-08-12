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
The UA sheet also carries two attribute rules, applied at the same step and
overridden by any author or inline declaration: `[hidden] { display: none }`
(HTML §15.3.1 — `hidden` is a boolean attribute, so presence is the fact and
`until-found` hides too, `bl-eeb4`) and, once `--js` ran, `noscript` (js.md §4).
The same UA block's third rule, `input[type=hidden i] { display: none !important }`,
is *not* overridable — `!important` in the UA origin outranks the author origin —
so it is decided before the cascade is consulted (`bl-a189`). Its AX twin is a
separate fact in a separate home: the HTML-AAM gives `type=hidden` no role at
all, which is what keeps it out of `--out ax` when no styles are computed.

Two further UA rules are **structural** rather than declarations — the parent's
box swallows the child — so they land *after* the cascade rather than inside it
(`css::cascade::conceal`, `dom::Document::concealed`, `dom/conceal.rs`). Every
child they hit — element **or text node** — generates no box:

- a `<details>` without `open` renders only its first `<summary>` child
  (`bl-74a6`);
- a **fallback-content element** paints **none** of its children, which HTML
  defines as content for a UA that does not implement the element
  (`tags::renders_children`: `<video>`, `<audio>`, `<iframe>`, `<canvas>`;
  `bl-0f83`, `bl-e79a`).

Neither is author-overridable, because the box the UA skips is the parent's —
`<details>`'s `::details-content`, the element's replaced box — not the child's,
and frot has no anonymous boxes to give the child one. Text nodes carry it too:
`Styles` is parallel to the whole arena, so
`<details><summary>Q</summary>A</details>` drops the `A` with everything else.

### 2.1 Two structural queries, because `<canvas>` splits them (`bl-e79a`)

"Not painted" and "not in the accessibility tree" were one fact until `<canvas>`
was measured. HTML makes canvas fallback content the element's **accessible
sub-tree**: Chrome 139 headless at 1280×720 gives `<canvas>x</canvas>` zero
`getClientRects()` and no `innerText`, and still exposes a live
`StaticText "x"` in `Accessibility.getFullAXTree`. Chrome does not model this as
`display:none` either — the fallback computes `display:block`,
`visibility:visible`, and simply generates no box. So `dom/conceal.rs` carries
two queries over one tag table:

| query | means | read by |
|---|---|---|
| `Document::unpainted` | generates no box | `layout::renders`, `needs::not_rendered` |
| `Document::concealed` | unpainted **and** absent from the AX tree | `Document::ax_children`, `css::cascade::conceal` → `display:none` |

`concealed` is the strict subset; they differ on `<canvas>` alone. Canvas
fallback must stay out of the cascade's `display:none` precisely because
`ax::tree` prunes there, and pruning it would drop the sub-tree HTML says a
screen-reader user gets — the VISION §5 violation in the opposite direction from
emitting text nobody sees. The exception is confined to the tag table:
authored `display:none` inside canvas fallback still hides it from AX, in Chrome
and here alike, because that arrives through the cascade.

`layout::renders` is therefore the render-tree gate, and it sits **above the
element/text split** — the box the UA skips is the parent's, so a `<video>`'s or
`<canvas>`'s raw text is no more measurable than its markup. `views::bboxes`
reads the same predicate for an element's `text` field, so a replaced box is
never sized or captioned from copy nothing paints.

Neither half is a UA *stylesheet* declaration, so neither waits for a cascade:
the fallback half is a content-model fact and the `<details>` half is document
state. Both therefore hold in every recipe, which §2.2 is about.

Its block-level tag set is *the same list* `views::text.rs` already uses for
block breaks (`BLOCK_TAGS`) — extract it to one place so the two never drift
(single source of truth). `li` → `ListItem`; everything else → `Inline`. It is
not an HTML-only set: SVG `<text>`/`<foreignObject>` and the MathML elements
that carry text are in it too, on the computed `display` Chrome reports for them
(`block` and `block math`), which is the same fact both consumers want
(`bl-c0a4`, needs.md §4.2).

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

### 2.2 Accessors, not remembered checks (`bl-0aaf`, `bl-d470`)

`bl-0f83` claimed the fallback rule "holds in every recipe" on the strength of
the views' skip sets. It did not, in three places, and each was invisible under
`--css` because the cascade spelled the same fact a second time as
`display:none` — which those walks *did* check:

- `ax::tree` asked `concealed`; `ax::name::contents` (accname §2F) did not. So
  `<a><video>Sorry, your browser does not support embedded videos</video></a>`
  gave `link` a `null` name under `--css` and named it from the fallback prose
  without: the node cut from the tree, its text still naming the tree
  (`bl-0aaf`).
- `views::text` asked only the *tag* half and left the closed-`<details>` half
  to the cascade, so `<details><summary>Options</summary><p>body` emitted
  "Options\nbody" without `--css` and "Options" with it (`bl-d470`).

Three walks over one arena, each with its own idea of what was excluded. The
fix is not a fourth caller of the predicates. Each query gets an **accessor**
built from the same `keeps`, and every child traversal goes through one:

```
views::text::emit           ────► Document::painted_children ─► unpainted
ax::tree::ordered_children  ─┐
                             ├───► Document::ax_children     ─► concealed
ax::name::contents          ─┘
```

`ordered_children` keeps only what it alone knows — flex reading order. The
predicates stay for the compound gates that glue this fact to a cascade fact
(`layout::renders`, `needs::not_rendered`, `css::cascade::conceal`), which are
not traversals and cannot forget a child they never iterate.

Three consequences follow, all of them subtractions:

- The `<details>` half is no longer cascade-gated for any *content* view.
  Chrome 139 agrees in both directions: `body.innerText` for a closed
  disclosure is its summary alone, and a link wrapping
  `<details><summary>SUMM</summary><p>SECRET2` is named "SUMM" with no AX node
  under it.
- Neither the §2F recursion nor `views::text` checks a *text* node for
  hiddenness any more. Neither can reach a hidden one: a hidden element returns
  above it, and a swallowed one never comes out of an accessor.
- `views::text` no longer reads `tags::renders_children` at all — the fallback
  skip it used to keep locally is the accessor's job now.

### 2.3 Which recipes each rule holds in

The claim that needed correcting twice, stated once, for all four rules:

| rule | kind | holds without `--css`? |
|---|---|---|
| `[hidden]` attribute | UA *stylesheet* declaration, author-overridable | **no** — it is a cascade rule, and `--css` is what "apply CSS" means. `needs` is the exception: it reads the attribute directly, because a starved-page verdict must not turn on the recipe |
| closed `<details>` | document **state**, structural | yes, everywhere |
| media/`<iframe>` fallback | **content model**, structural | yes, everywhere |
| `<canvas>` fallback | content model, structural, **paint only** | yes, everywhere — dropped from `text`/`bboxes`/`needs`, kept in `ax`, in either recipe (§2.1) |

The rule of thumb the accessors enforce: a *declaration* needs the cascade; a
*structure* does not. The recipe decides how a fact was computed, never which
content it covers.

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

**Generated content is inline content** (bl-6fcb). `--css` capability model:
computed `::before`/`::after` content propagates to *all* subsequent outputs, so
it must reach layout too — an icon span whose whole content is
`::before{content:"\e609"}` has a real box in a browser, and the inline content
after it is displaced by it. The rule is a reframe, not a special case: the
computed generated string is tokenized by the *same* tokenizer as a text child
(`src/layout/inline.rs`), at the position a browser puts it — `::before` ahead of
the element's children, `::after` behind them — so it wraps, advances the line
cursor, and unions into the originating element's rect and its inline ancestors'
exactly like text. Consequences, all of them inherited rather than invented:

- A pseudo with `display:none` computes no string in the cascade, so it
  contributes no geometry — no layout-side check.
- `attr()` content re-tokenizes whenever the tables are recomputed, so a
  post-`--js` attribute change moves the geometry (js.md §8's per-generation
  cache) with no extra mechanism.
- Generated content on a container that *also* has block-level children is
  dropped, exactly as a stray text child in that position is: Phase 3 promotes no
  anonymous block boxes (§1). Same gap, same reason, for both.
- Widths stay estimates: the glyph advance is the same 8px, so an icon glyph is
  8px wide here where a browser's font gives it ~21px.

**One coordinate space, composed on both axes** (bl-2161). Every rect in the
table is in viewport coordinates; there is no local/absolute distinction to get
wrong at a boundary, because a nested pass never *returns* local coordinates.
Concretely: inline line-breaking works in block-local coordinates (`x = 0` means
"line start", and the wrap test is against the content width), and the containing
block's content origin is composed onto a fragment at the moment it becomes a
`Rect` — on `x` exactly as on `y`. The consequences are invariants, not extra
mechanism:

- **A box never starts before its containing box's origin on either axis.**
  Composition only translates right and down (no floats, no negative margins, no
  RTL — all §6 non-goals), so `child.x >= container.x` and `child.y >=
  container.y` hold for every descendant. Boxes may still extend *past* a
  container's far edge: an over-long word overflows its block. It is an origin
  invariant, not a containment one.
- **An element that renders no word still has a position.** Its placeholder is
  the empty box `{x, y, 0, 0}` at its containing block's content origin, not a
  zero-size box at the viewport origin — an icon `<i>` whose content is all
  border/background is honestly "here, with no measurable extent" rather than
  falsely at `(0, 0)`. Widths are estimates (see the inline metrics above); the
  origin is not.

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
