# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.7](https://github.com/mudbungie/frot/compare/v0.0.6...v0.0.7) - 2026-11-25

### Changes

- js.md §15.5 — component sites measured after ElementInternals, composed events, slotchange, assign, delegatesFocus, setHTMLUnsafe [bl-d865]
- delegatesFocus is recorded but focus() on a shadow host does not move focus into its shadow tree [bl-326a]
- manual slot assignment — slot.assign() is absent and a manual-assignment shadow root assigns nothing [bl-f432]
- the window's everyday members are absent — event, name, origin, opener, status, scroll/scrollTo/scrollBy, focus/blur/stop/print, alert/confirm/prompt, open/close, structuredClone, getSelection [bl-d8f7]
- setHTMLUnsafe / parseHTMLUnsafe are absent — no script path to a declarative shadow root [bl-c1a3]
- NodeIterator and document.createNodeIterator are absent; Firefox 153 has both [bl-33df]
- iframe windows: an external or module <script> inserted into a frame's document never runs (its pending queue is never drained) [bl-b34e]
- DOM getters and methods do not brand-check their receiver; Firefox throws 'get X' called on an object that does not implement interface Y. [bl-a1a1]
- crypto.subtle RSA residual — PSS saltLength other than the digest length, SHA-1 signing, keys under 2048 bits reject NotSupportedError (left by bl-2763) [bl-4ba0]
- a 0 ms timer set by a parser script fires after load in frot; Firefox fires it before DOMContentLoaded [bl-9f4c]
- OfflineAudioContext / AudioContext are not EventTargets — addEventListener('complete') throws 'not a function' [bl-bd3d]
- HTMLScriptElement reflects only src, type and text — async, defer, noModule, charset, crossOrigin, referrerPolicy, integrity, fetchPriority, event, htmlFor absent [bl-f9f2]
- PerformanceResourceTiming (and PerformanceEntry) are not interface objects — performance entries are plain objects; Cloudflare's challenge reads the global [bl-c378]
- a <style>/<script>/<title> an author displays (style="display:block") is rendered in Chrome; frot skips it in text, names and bboxes [bl-8f60]
- should --out dom serialize an ordinary <template>'s contents as its children? [bl-2a85]
- slotchange never fires [bl-352f]
- text inside an SVG element SVG does not know (<svg><foo>t</foo></svg>) is painted in frot; Chrome paints none [bl-01b2]
- autofocus leftovers — a candidate hidden only by a stylesheet, focus before the parse ends, a radio form= parser-history quirk [bl-4248]
- <img srcset>/sizes and <picture><source> ignored under --images; frot requests src where Firefox requests the chosen candidate [bl-0e4a]

## [0.0.6](https://github.com/mudbungie/frot/compare/v0.0.5...v0.0.6) - 2026-11-25

### Changes

- innerHTML on <title>/<textarea>/<style>/<script> parses markup into elements; Firefox's RCDATA/RAWTEXT context keeps it as text [bl-538f]
- Node.isEqualNode is absent [bl-d5dc]
- no Window interface — a window's constructor is Object, 'Window' is not defined, window instanceof Window throws [bl-226d]
- ReadableStream is absent — Cloudflare's managed challenge on medium.com and npmjs.com reads ReadableStream.prototype in a timer callback, throws, and never answers [bl-96f7]
- HTMLElement.title, document.URL/documentURI, document.forms, HTMLFormElement.elements/action, a control's .form and named access (window[id], document[name], form[name]) are absent [bl-09c8]
- events do not cross shadow boundaries — a composed event stops at the shadow root, no retargeting, composedPath() omits the host side [bl-9872]
- form-associated custom elements and ElementInternals are absent — attachInternals, static formAssociated, setFormValue/validity, :state() [bl-be13]
- firstElementChild / lastElementChild / childElementCount / nextElementSibling / previousElementSibling are absent on elements, documents and fragments [bl-59f7]
- a typed navigation through a 3xx sends Referer: <pre-redirect URL> on the landing request; Firefox 153 sends none [bl-b298]
- test flake: js::tests::netbudget::the_concurrent_warm_wave_is_charged_too fails under builder load (left (1, 1, false), right (2, 0, false)) [bl-946c]
- the top layer is partial — a modal dialog does not make the page inert, popovers do not light-dismiss each other [bl-2dcd]
- text-transform casing beyond Turkic lang and simple titlecase (Greek/Lithuanian/Dutch tailoring, multi-char titlecase, combining marks in capitalize) [bl-f058]
- q marks by lang and q::before content:none in names; a displayed <style> is silent in names (left from bl-a510) [bl-73ee]
- iframe srcdoc is never parsed or run — the frame stays on its initial about:blank [bl-7b86]
- table parts are separated by what the walk saw last, not by Chrome's anonymous table boxes [bl-34f1]
- a display:contents child of a flex container is one flex item; Chrome promotes its children [bl-da9d]
- document.createTreeWalker is absent — a Lit web-components bundle on Amazon's home page throws 'not a function' at load [bl-aff4]
- an unknown HTML element (HTMLUnknownElement) takes the autofocus/draggable group and list-item rules; frot applies them to a fixed tag list and custom elements only [bl-4bc5]
- an <input placeholder> with no other name source is named by its placeholder in Chrome; frot names it "" [bl-2d76]
- the flat tree — text/ax/bboxes/links/forms read shadow content in place, --out dom serializes shadow roots, shadow style scoping (:host, ::slotted, adoptedStyleSheets) [bl-00f5]
- display-derived listitem — an <li> outside any list and an inline list-item box are listitem in Chrome; frot gives no node [bl-90c1]
- form-control state a script writes is an attribute in frot — .checked/.selected leave a mark Chrome does not, radio exclusivity and .indeterminate unseen [bl-ff74]
- MathML math-auto italic — a one-letter <mi> paints as its Mathematical Italic codepoint [bl-9b22]
- the hidden attribute hides SVG elements; Chrome's UA rule is HTML-only [bl-7d99]
- data-table signals that need the cascade — CSS cell borders and backgrounds, empty-cells:hide, contenteditable [bl-e143]
- input type=date is Date and type=color ColorWell in Chrome's AX tree; frot textbox [bl-dd43]
- cloneNode() of a doctype returns a Text node; Firefox returns a DocumentType [bl-ad2d]
- iframe windows: cross-document adoption throws; window[i] is not indexed; removal keeps the window open [bl-2aaa]
- cascade facts ax needs and Styles does not expose — grid/table display (coerced to block), absolute vs float, list-style-type on a list item [bl-08c6]
- crypto.subtle RSA (RSA-OAEP, RSA-PSS, RSASSA-PKCS1-v1_5) rejects NotSupportedError; Firefox 153 implements all three [bl-2763]
- attribute selectors with an operator — [a^=v] [a$=v] [a*=v] [a~=v] [a|=v] and the i/s flags — are unsupported; querySelector throws, and a stylesheet rule using one never applies [bl-1519]
- SharedArrayBuffer is a global in frot; Firefox 153 has none on a non-isolated page [bl-93e9]
- shadow roots — attachShadow, ShadowRoot, slot assignment, declarative shadow DOM in the parser [bl-3787]
- scripts inserted after the script phase never run — Cloudflare's managed challenge inserts its next stage from a timer and stalls [bl-a86b]
- document.title and document.referrer are undefined (Firefox: strings) [bl-dc63]
- window.postMessage is absent (top window and every iframe window) [bl-0af7]
- name-from-contents leftovers after bl-d94b — UA shadow text (file input, media), q marks by lang and q::before content:none, a displayed <style> [bl-a510]
- screen is the 1280x720 viewport, but the window (outerHeight 806 since bl-9663) is taller than the screen [bl-ca20]
- fetch images under --js — script-created only, all, or none (recommend none: measured no text/ax change, beacons, budget) [bl-49f1]
- :valid / :invalid / :in-range / :out-of-range unsupported — constraint validation at load [bl-51e9]
- event-handler content attributes never run — <img onerror="…"> and every other on* attribute is inert; on* IDL properties are absent from HTMLElement.prototype [bl-93c8]
- an open popover is a group in Chrome; frot gives the div no node [bl-cf7b]
- customElements registry — define/get/whenDefined/upgrade, lifecycle callbacks, upgrade, :defined truthful, customized built-ins [bl-688d]
- dispatchEvent has no propagation — no capture or bubble phase, so a document-level capture listener never sees an <img> load/error [bl-810b]
- <template>.content is a parser-filled inert DocumentFragment; document.importNode; cloneNode copies template contents [bl-52bf]
- follow Chrome in exposing what is inside a display:none SVG <g>? [bl-1177]
- how the envelope reports a round a script-requested reload earned (no header declared it; challenge.declared is a header name) [bl-14f9]
- a closed <dialog>, <area> and <slot> are painted — the UA display:none (dialog:not([open]), area) and display:contents (slot) rules are missing [bl-9433]

## [0.0.5](https://github.com/mudbungie/frot/compare/v0.0.4...v0.0.5) - 2026-11-23

### Changes

- state pseudo-classes read the DOM only — script-moved focus, property writes (.checked, .selected), showModal()/showPopover() are invisible; document.activeElement ignores autofocus [bl-bc6d]
- generated-content leftovers after bl-082b — MathML containers, a broken img, object by load outcome [bl-aa2b]
- display leftovers after bl-e3a0 — table-column content painted, inline list-item has no listitem, display:inherit dropped, stray table parts by adjacency, li outside a list [bl-9dec]
- pseudo-classes still unsupported after bl-1508 — :indeterminate, :default, :valid/:invalid, :in-range/:out-of-range, :placeholder-shown, :open, :defined, :lang(), :dir(), :has(), :scope, media states [bl-2a85]
- images under --js — stay pending forever (today), fail like a Firefox with images blocked, or actually send the GET (beacons) [bl-91a4]
- name from contents reads descendants Chrome does not — container roles, range widgets, fieldset, input defaults, q by lang [bl-d94b]
- crypto.subtle residual: the public-key algorithms (RSA*, ECDSA, ECDH, Ed25519, X25519) Firefox 153 has and frot rejects [bl-0cae]
- SVG leftovers after bl-1c54 — [hidden] applied to SVG elements, display:none <g> still exposed in Chrome, svg <a> without href, links view lists a MathML <a href> [bl-b3db]
- paragraph leftovers after bl-5c70 — cases the cascade cannot see (pseudo display, grid/table, UA display:none tags, onclick) [bl-50ec]
- ruby, MathML, meter/progress and iframe diverge outside names — rp painted, fallback text in text, Chrome-only roles [bl-4991]
- group leftovers after bl-0725 — custom and unknown elements, onclick keeps a span, display:list-item is a listitem, a table-cell div [bl-824c]
- <img alt="" title=T> is named T; Chrome names it "" [bl-e88d]
- roles Chrome gives and frot does not — address is group, dir is list [bl-c837]
- window.innerHeight === outerHeight: frot has no browser chrome, and no real Firefox window looks like that [bl-9663]
- iframe.contentWindow / contentDocument are absent — Cloudflare's managed challenge evaluates code in an iframe's window and stops there [bl-54ac]
- honor a script-requested reload of the caller's own URL? Amazon's undeclared 200 is an Akamai interstitial a browser clears with one POST and a reload [bl-99b7]
- role leftovers after bl-2ea9 — explicit role=row in a layout table, <li> under role=<unknown>, role=directory [bl-d61c]
- document.createTreeWalker, TreeWalker and NodeFilter — a filtered pre-order walk over the arena, as Firefox 153 does it [bl-b8c0]
- title names a <p>; Chrome gives paragraph "" [bl-84fc]
- role=presentation/none on a table or list with aria-label is ignored by Chrome (the element keeps its native role); frot drops the node [bl-89a6]
- DOMRect and DOMRectReadOnly are not constructable — new DOMRect() throws 'Illegal constructor.' [bl-e65f]
- fidelity corpus: add the W3C Web Share Recommendation page (left out again by bl-8682) [bl-3248]
- DOM leftovers after the interface chain: comment.data writes ignored; doctype nodeName is #doctype (Firefox: html); Promise()/Map() without new give the engine's message [bl-3d15]
- a cookie set by a challenge page's subresource response earns the round though the script solved nothing (Cloudflare: rounds:1 every run) [bl-cc4c]
- :target is unsupported — the selector engine is not given the URL fragment [bl-5852]
- text-transform is not applied; Chrome's innerText and accessible names carry the transformed text [bl-e48d]
- data-table signals Chrome reads and frot does not (thead/tfoot/colgroup, headers/scope/abbr, rules, border) — and aria-label, which Chrome does not [bl-7f3a]

## [0.0.4](https://github.com/mudbungie/frot/compare/v0.0.3...v0.0.4) - 2026-11-22

### Changes

- a block-level box inside <pre> does not break the line [bl-2f42]
- ::before/::after on a replaced element (input) emits generated content; Chrome renders none [bl-082b]
- a legend and the control after it fuse into one token inside a fieldset [bl-bccf]
- a span or div carrying autofocus is role group in Chrome; frot gives it no node [bl-0725]
- the form= attribute does not associate a control with its form [bl-e8fa]
- the links channel reads document.links, which omits SVG <a>; record every hyperlink so the two state rows become an equality [bl-4650]
- display values coerced to block that are not block-level — contents, inline-grid, inline-table, ruby, run-in, table-cell — now break the line in text [bl-e3a0]
- an absolutely positioned box fuses with its neighbours; Chrome puts it on its own line [bl-346e]
- <wbr> gets a box; Chrome gives it no client rects [bl-1d51]
- the other structural pseudo-classes are unsupported — :only-child, :nth-child(), :nth-last-child(), :first-of-type family, :empty, :root (--css, querySelector) [bl-00d1]
- block breaks follow the tag, not the computed display; Chrome's innerText follows display [bl-fe2c]
- SVG leftovers after bl-07bd — CSS-hidden text still makes an svg a graphics-document; svg <a>/<image>/<text> titles [bl-1c54]
- :not(:hover) never matches; in an unattended browser :hover is known-false, so its negation matches [bl-1508]
- a hidden or display:none <option> is dropped under --css; Chrome's innerText still lists it [bl-240d]
- name-from-contents separators at atomic inline boxes still differ from Chrome (nested button, img without alt, float/absolute, ruby, q, media) [bl-3fc9]
- Amazon's delivered home page still throws once per run — 'Error: not a function' (the absent API is unnamed) [bl-cb8c]
- PerformanceObserver is absent — Cloudflare's managed challenge throws 'not a function' constructing one [bl-b129]
- document owns 37 properties; a real one owns exactly one (location) [bl-9f79]
- <details> and <summary> do not break the line; Chrome's innerText puts each on its own line [bl-788d]
- the staging DocumentFragment is an arena node, and inserting one moves its children in the host [bl-de06]
- name from contents includes the text of a <style> element in the body [bl-b124]
- a no-break space in an accessible name becomes a plain space; Chrome keeps U+00A0 [bl-3bd4]
- an interface called without new does not throw "<Name> constructor: 'new' is required" [bl-6404]
- the document is an arena node — its children are the roots, and a root's parent is the document [bl-196a]
- fidelity corpus: add the Python docs page and the W3C spec page left out of bl-5334 [bl-8682]
- table and caption leftovers after bl-2b09/bl-fe88 — empty caption shuts out title, caption title, CSS-only caption content, one-cell <th> tables [bl-dfc3]
- one prototype per node kind — Node, Element, CharacterData — instead of one flat Node.prototype [bl-78cb]
- :first-child / :last-child are unsupported, so list separators made with them come out wrong (--css) [bl-e987]
- a <p> holding only block-level children emits a paragraph node; Chrome ignores it [bl-5c70]
- the sibling combinators + and ~ are unsupported, so their rules never apply (--css) [bl-6363]
- DPUB-ARIA roles (doc-noteref, doc-footnote, doc-backlink) are ignored; Chrome exposes them [bl-fc83]
- :not() / :is() / :where() are unsupported, so a rule that needs one never applies (--css) [bl-011e]
- a table with role=presentation still emits its rows and cells; Chrome emits none [bl-2ea9]
- @import is skipped, so rules in an imported stylesheet never apply (--css) [bl-3aef]
- delete the leftover debug test tmp_dump2 (prints, asserts nothing) [bl-886c]
- fidelity corpus: add saved real pages (subresources localized); needs a licence ruling per page [bl-5334]
- the DOM interface chain — document as an arena node, and one prototype per node kind (the plan bl-9f79 is split by) [bl-fcdd]
- FileReader is absent — AWS WAF's challenge tries FileReader.readAsDataURL(blob) first and falls back to btoa [bl-8c46]
- an empty <dt title=T> is named from its title; Chrome names it "" [bl-0794]
- an unnamed explicit role=form / role=region keeps its role; Chrome demotes it to generic [bl-c5a6]
- an empty <caption> emits a caption node; Chrome leaves it out [bl-fe88]
- a <table summary> is unnamed; Chrome names the table from its summary attribute [bl-2b09]
- Image is absent — Amazon's delivered home page throws 'Image is not defined' (twice per run) [bl-1404]
- a childless <svg> is graphics-document; Chrome exposes it as img, and a <g> with a <title> as a named group [bl-07bd]
- vendored React 17 bundle in src/js/engine/ carries no licence attribution [bl-41e5]
- crypto.subtle residual: the WebCrypto algorithms and AES-GCM parameters Firefox 153 has and frot rejects [bl-db8f]
- www.amazon.com sometimes answers 200 with no challenge declaration and a script-only page — needs:[js] under --js [bl-eed9]
- a declared challenge at status >= 400 (Cloudflare: 403 + cf-mitigated: challenge) never enters the round trip [bl-a7c5]
- name-from-contents fuses block and table-cell boundaries (row "G110", button "blk1blk2"); Chrome separates them with a space [bl-fb69]
- options of a collapsed select get boxes; Chrome gives them none [bl-2932]
- fidelity recorder: capture what innerText and getClientRects miss, so residual rows become real checks [bl-7966]
- flex order / row-reverse reorder AX children; Chrome's AX tree stays in DOM order [bl-7430]
- Blob (and URL.createObjectURL) is absent — read by AWS WAF challenge.js [bl-78c5]
- document.scripts is absent — AWS WAF challenge.js throws reading document.scripts.length [bl-c296]
- requestIdleCallback is absent — read by AWS WAF challenge.js [bl-b584]
- TextEncoder/TextDecoder are absent — read by AWS WAF challenge.js [bl-53b7]
- FormData is absent — read by AWS WAF challenge.js [bl-6eb9]
- crypto.subtle is absent — AWS WAF challenge stops at importKey('raw', 32 bytes, {name:'AES-GCM'}, _, ['encrypt']) [bl-0647]
- decide whether frot publishes crypto.subtle (and isSecureContext with it) [bl-b0f8]

- **`--out text`: a block inside `<pre>` breaks the line.** `<pre>one
  <div>two</div>three</pre>` is three lines, as in Chrome; it was
  `onetwothree`. Preformatted white space is untouched, and a newline already
  written before the block is not doubled. [bl-2f42]

- **`--css`: `::before`/`::after` generate nothing on a box that takes no
  children**, as in Chrome, which builds no pseudo-element box there:
  `<img>`, `<br>`, `<wbr>`, `<video>`, `<audio>`, `<canvas>`, `<iframe>`,
  `<embed>`, `<textarea>`, a text or button `<input>` (a checkbox, radio,
  range, color, file or date input still has them), a menu-list `<select>`
  and its options, and `<svg>` with its contents down to a
  `<foreignObject>`. `input.auto::after { content: " unfocused" }` used to
  put `unfocused` into `--out text`, an accessible name and the layout.
  [bl-082b]

- **`<legend>` is block-level**, and with it the other tags Chrome's UA
  stylesheet makes block-level that frot had as inline: `<center>`, `<dir>`,
  `<hgroup>`, `<listing>`, `<menu>`, `<plaintext>`, `<search>`, `<xmp>`. In
  `--out text` each stands on its own line, with or without `--css`
  (`<legend>Off</legend><button>off</button>` was the fused token `Offoff`),
  an accessible name spaces one off, and `bboxes` lays it out as a block.
  [bl-bccf]

- **`--out ax`: `autofocus` or `draggable` makes a plain container a
  `group`**, as Chrome does. A `<div>`, a `<span>` or another element with no
  role of its own (`<b>`, `<i>`, `<pre>`, …) that carries either attribute —
  whatever its value — is a `group` node when Chrome keeps a node for it: it
  has an `id`, a `tabindex`, `contenteditable`, an `aria-*` attribute or a
  `title`, or (not a span) it is a block holding a line of text. A `role`
  attribute turns the rule off. `<div draggable="true">card</div>` was no
  node. [bl-0725]

- **`--out forms`: the `form=` attribute associates a control with its
  form.** A form's `fields` are now the controls it owns, in document order:
  a control anywhere in the page whose `form` attribute names the form's id
  is listed, and a control inside one form that names another is listed
  there instead. A `form` attribute that names no form (empty, an id nothing
  has, an id a non-form element has first) leaves the control in no form, as
  in Chrome's `form.elements`. `:checked` under `--css` reads radio groups by
  the same owner, so a radio with `form=f` and one inside `<form id=f>` are
  one group. [bl-e8fa]

- **`--css`: `display` values that are not block-level stop breaking the
  line**, a regression since `bl-fe2c`, where every value outside seven
  keywords was read as `block`. Measured per value in Chrome 158. [bl-e3a0]
  - `display: contents` is no box: no line break in `--out text`, no entry
    in `--out bboxes` (its children keep theirs and are laid out in its
    parent's flow), no `paragraph` for a `<p>`.
  - `inline-grid`, `inline-table` and `-webkit-inline-box` are inline-level
    boxes like `inline-block`; `ruby` and `ruby-text` are inline.
  - `table-cell` and `table-row` (and the row groups) are on no line of
    their own: a cell is set off from the cell just before it by a space and
    a row from the row before it by a line break, where Chrome's `innerText`
    writes a tab and a newline. `p<span style="display:table-cell">c1</span>
    <span style="display:table-cell">c2</span>q` is `pc1 c2q`.
  - A value Chrome rejects (`run-in`, a typo) is no declaration: the valid
    declaration it would have beaten wins, else the tag's own display. It
    used to make the element a block. `display: revert` gives the tag's own;
    `initial` and `unset` give `inline`.
  - The two-keyword forms (`inline flex`, `block flow`, `inline flow-root`,
    …) are read as their one-keyword twins.
  - Under `--js`, `getComputedStyle().display` answers `contents`,
    `table-cell` and `table-row` for those boxes.

- **`--css`: an absolutely positioned or floated box is block-level**, as
  CSS computes it. `position: absolute | fixed` and `float` blockify the
  `display` of an element or a `::before`/`::after`, so in `--out text` such
  a box stands on its own line (`Before<span class="tip">tip</span>after.`
  was the fused token `Beforetipafter.`), in `--out ax` it is spaced off
  inside a name, and a `::before`/`::after` with `display: block` is too.
  `bboxes` geometry now lays such a box out as a block in flow. [bl-346e]

- **`--out bboxes`: `<wbr>` has no entry.** It is a break opportunity, not a
  box — Chrome gives it no client rects — so the view no longer lists one. A
  `<br>` keeps its entry. [bl-1d51]
- **`--out text --css`: line breaks follow the computed `display`, not the
  tag**, as Chrome's `innerText` does. A `display:block`, `flex` or
  `list-item` span now stands on its own line (`x<span
  style="display:block">sb</span>y` was the fused token `xsby`), and a
  `display:inline` div, list item or table cell no longer breaks one. A
  `<p>` still does, whatever its box. Without `--css` nothing changes. Values
  the cascade coerces to block (`contents`, `inline-grid`, `inline-table`)
  break the line too, which Chrome does not. [bl-fe2c]

- **`--out ax`: SVG edges follow Chrome.** [bl-1c54]
  - Under `--css` an `<svg>` whose only content is hidden (`<text
    style="display:none">`, `visibility:hidden`) is an `img`, not a
    `graphics-document`.
  - An `<a>` inside an `<svg>` is named by its `<title>` child when it has
    one (`link "at"`, not the title run into the link text), and a `<text>`
    or `<tspan>` with a `<title>` contributes that title to a name around it.
  - A described `<image>` (a `<title>` child or an author name) is an `img`
    node inside its graphic.
- **`--css`: the rest of the structural pseudo-classes.** `:only-child`,
  `:nth-child()`, `:nth-last-child()`, `:first-of-type`, `:last-of-type`,
  `:only-of-type`, `:nth-of-type()` and `:nth-last-of-type()` now match, with
  the whole `An+B` grammar (`odd`, `even`, `2n+1`, `-n+3`) and
  `:nth-child(… of S)`, each checked against Chrome 158. Their rules were
  ignored and `querySelector` threw on them. [bl-00d1]

- **`--out text --css`: a `<select>` lists its hidden options**, as Chrome's
  `innerText` does. An `<option>` that is `hidden`, `display:none` or
  `visibility:hidden`, or sits in a hidden `<optgroup>`, used to be dropped;
  only hiding the select itself hides its options now. `--out ax` follows
  for a collapsed select, whose popup keeps those options in Chrome; a list
  box still drops them. [bl-240d]

- **`--out ax`: a name from contents is spaced as Chrome spaces it at
  controls and replaced elements.** [bl-3fc9]
  - A control inside a name is a word of its own: `v<button>inner</button>w`
    is "v inner w", and so is a `role=button`, `checkbox`, `radio`, `switch`,
    `tab`, `menuitem`, `textbox`, `searchbox`, `listbox` or `tree`.
  - `<img>` with no `alt` is a (silent) word, `a<img>c` is "a c"; a
    presentational one (`alt=""`, `role=presentation`) separates nothing,
    `a<img alt="">c` is "ac".
  - `<rt>`, `<rp>` and `<title>` say nothing; MathML is not read (`a<math>…
    </math>c` is "a c"); `<meter>` and `<progress>` say their value; an
    `<iframe>` its `title`; a `<canvas>` with fallback content is spaced off;
    an `<area>` no longer puts its `alt` into the name around its `<map>`.
  - Under `--css` a `<q>` carries its quote marks (“ ” outside, ‘ ’ nested —
    the English marks, whatever `lang` says).
  - A `<summary>` is a `button` only as a child of `<details>`; elsewhere it
    emits no node.
- **`PerformanceObserver` exists under `--js`.** An observer of `resource`
  receives an entry for each request the page really made, as
  `performance.getEntriesByType('resource')` already listed them; the other
  entry types Firefox names are accepted and never delivered (frot measures no
  navigation timing and paints nothing). Cloudflare's challenge script no
  longer throws constructing one. [bl-b129]

- **`--js`: `document` is a node.** `document.appendChild`, `.childNodes`,
  `.firstChild`, `.contains`, `.removeChild` and `.replaceChild` exist (they
  threw `not a function`), `document instanceof Node` is true, and
  `String(document)` is `[object HTMLDocument]`. Its members are on
  `Document.prototype`, in Firefox's order; the object itself owns only
  `location`, as a real one does. [bl-9f79]

- **`--css`: `:not(:hover)` matches, and the state pseudo-classes are real.**
  frot loads a page nobody is using, so `:hover`, `:active`, `:visited`,
  `:modal`, `:popover-open`, `:fullscreen`, `:picture-in-picture`,
  `:autofill` and `:user-valid` / `:user-invalid` are false — and their
  negations now match (`.menu:not(:hover){display:none}` hides the menu, as
  in Chrome; it was ignored). Answered from the document, as measured in
  Chrome 158: `:root`, `:empty`, `:link` / `:any-link`, `:checked`,
  `:disabled` / `:enabled`, `:required` / `:optional`, `:read-only` /
  `:read-write`, and `:focus` / `:focus-visible` / `:focus-within` from
  `autofocus`. `querySelector` and `matches` accept them all instead of
  throwing. `:target`, `:has()` and the rest stay unsupported. [bl-1508]

- **`<details>` and `<summary>` are block-level**, as in every browser. In
  `--out text` each starts and ends a line, so a summary no longer fuses
  with the text beside its `<details>` (`head<details><summary>four…` was
  `headfour`); in `--out ax` their contents are spaced off inside a name
  from contents; and layout gives them block boxes. [bl-788d]
- **`--js`: a `DocumentFragment` is a real node.** It inherits `Node`, so
  `querySelector`, `getElementById`, `cloneNode`, `contains` and `textContent`
  work on it (they were absent), a node appended to it has it as `parentNode`,
  and appending one fragment to another moves the children. Inserting a
  fragment is one operation in the tree, and is refused with Firefox's message
  when its contents do not fit (text or a second element into the document, a
  fragment into itself). [bl-de06]

- **`--out ax`: a `<style>` or `<script>` inside an element is no longer
  read into its name.** The text of `<script>`, `<style>`, `<template>` and
  `<noscript>` never contributes to an accessible name, as in Chrome — also
  when `aria-labelledby` points straight at one. A Wikipedia header cell
  carrying inline TemplateStyles was named with the stylesheet's source.
  [bl-b124]

- **`--out ax`: a name keeps its no-break spaces.** Only HTML's ASCII white
  space (space, tab, LF, FF, CR) is collapsed and trimmed in an accessible
  name, as in Chrome: `No&nbsp;break` keeps U+00A0 — it used to come out as a
  plain space — and so do U+2003, U+3000 and the rest. `aria-label` and
  `title` are now collapsed the same way (`aria-label="a  b"` is "a b"; they
  were only trimmed). [bl-3bd4]
- **`--js`: an interface called without `new` throws what Firefox throws.**
  `Event('x')`, `URL('…')`, `Blob()`, `XMLHttpRequest()`, `MutationObserver()`
  and every other constructable interface throw `TypeError: <Name>
  constructor: 'new' is required`; most used to return `undefined`.
  `AbortSignal()` and `new AbortSignal()` throw `Illegal constructor.`
  [bl-6404]

- **`--js`: the document is a node of the tree.** `html.parentNode` is the
  document (it was `null`), and the tree refuses what a browser's does, with
  Firefox's `HierarchyRequestError` messages: a node appended under its own
  descendant (which used to build a cycle), a second element under the
  document, a child under a text node. `el.matches()` and `querySelector` now
  answer for an element that is in no document; `outerHTML` on such an element
  is a no-op. `--out dom` and every other view are unchanged. [bl-196a]

- **The fidelity oracle reads a fourth saved real page** (dev-tree only;
  nothing shipped changes): a Python docs page with its six Sphinx
  stylesheets, one of them reached only through `@import`. It matches Chrome
  on every channel with no ledger row. Link and form references are now
  compared relative to the corpus root, so a saved page may link above its own
  directory. The W3C page stays out (223 differing lines, from absolute
  positioning and `text-transform`; `bl-3248`). [bl-8682]
- **`--out ax`: table and caption edges follow Chrome.** [bl-dfc3]
  - A table of one row holding one cell is a layout table (no `table`, `row`
    or cell node) unless it has a `<caption>`, a non-empty `summary` or a
    `role`; an empty `summary=""` no longer marks a data table.
  - A `<caption>` or `<legend>` that is present is the whole native name:
    `<table title=T><caption></caption>` and `<fieldset title=T><legend>
    </legend>` have no name. A hidden caption is skipped, so the next
    caption, `summary` or `title` names the table.
  - `title` does not name a `<caption>` or a `role=caption` (it still names a
    `<figcaption>`).
  - Whether an empty `<caption>` emits a node now reads the cascade: content
    that is all `display:none` does not keep it, `::before` text does, and a
    caption whose only content is a block-level child, or that is itself
    `display:inline`, has none.
  - `role=form`, `role=region`, `<section>` and a nested `<aside>` keep their
    landmark role when they carry a `title` attribute, even an empty one.
- **`--js`: DOM nodes have the interfaces a browser gives them.** An element
  is an `HTMLDivElement` / `HTMLInputElement` / … (all 71 HTML interfaces, by
  tag) under `HTMLElement`, `Element`, `Node` and `EventTarget`; a text node is
  a `Text` under `CharacterData`. Each member sits on the interface Firefox 153
  puts it on, so a text node has no `getAttribute` and a `<div>` no `value`,
  `href` or `getContext`. A node now has one JS object: `el === el` holds and a
  property a script sets on a node is still there at the next read. New on
  `Node`: `hasChildNodes`, `isSameNode`, `isConnected`, `parentElement` and the
  node-type constants; `new Text()` and `new Comment()` construct.
  `el.textContent = null` or `''` leaves no child. [bl-78cb]

- **`--css` and `querySelector` understand `:first-child` and
  `:last-child`.** List separators drawn with them come out as in a browser:
  `li::after { content: " ·" }` `li:last-child::after { content: none }` no
  longer leaves a trailing `·` (every Wikipedia navbox row). A pseudo-class
  written with two colons (`::first-child`, `::is(a)`) is invalid CSS and no
  longer matches. [bl-e987]

- **`--out ax`: a `<p>` with no line of its own emits no `paragraph`**, as in
  Chrome. That is an empty `<p>` (in every recipe), and under `--css` one
  with any block-level child (`<p><span style="display:block">…`) or one that
  is itself `display:inline` or `flex`; its contents surface in its place. A
  `<p>` carrying `id`, `title`, `tabindex`, `contenteditable`, `role` or any
  `aria-*` attribute keeps its node, as does a `role=paragraph` on another
  element. [bl-5c70]
- **`--css` and `querySelector` understand the sibling combinators `+` and
  `~`.** `h2 + a.self-link::before { content: "§" }` now paints its mark, and
  `document.querySelectorAll('dt ~ dd')` returns the siblings instead of
  throwing. Before, any rule with a sibling step was dropped. [bl-6363]

- **`--out ax`: the DPUB-ARIA roles are honoured.** An author `role` naming
  any of the 41 `doc-*` roles (`doc-noteref`, `doc-footnote`, `doc-chapter`,
  …) is emitted under that name, as in Chrome, instead of falling back to the
  element's own role. `doc-backlink`, `doc-biblioref`, `doc-glossref`,
  `doc-noteref` and `doc-subtitle` are named from their contents; the rest
  only by the author. [bl-fc83]
- **`--css` and `querySelector` understand `:is()`, `:where()` and `:not()`**,
  over lists of complex selectors, nested, with the spec's specificity
  (`:where()` zero; `:is()`/`:not()` their most specific argument). Before, a
  rule using one never applied — and when that rule was the one *revealing*
  content (`.label{display:none}` `.label:not([hidden]){display:inline}`,
  Bootstrap's docs header) the content was wrongly hidden. A `:not()` over
  something frot cannot evaluate (`:not(:hover)`) still never matches, and
  `querySelector` still throws on it. [bl-011e]

- **`--out ax`: a table or list the author gave another role takes its
  parts' roles with it**, as in Chrome. Under `<table role=presentation>` (or
  `none`, `group`, any role but `table`/`grid`/`treegrid`) a `<thead>`,
  `<tr>`, `<td>` or `<th>` emits no node, and under `<ul role=none>` (any
  role but `list`) neither does an `<li>`; their contents surface in place. A
  part with its own `role` keeps it, and a nested `<table>` starts over.
  [bl-2ea9]
- **`--css` follows `@import`.** A stylesheet's `@import "x"` / `@import
  url(x)` rules (in a linked sheet or a `<style>` block, nested imports too)
  are fetched in the same budgeted stylesheet gather and cascade ahead of the
  importing sheet, as in a browser. A media-qualified import is fetched only
  when its query matches the 1280×720 viewport; one carrying `layer` or
  `supports()` is not applied. Before, every imported rule was silently
  missing — a Sphinx docs page kept ~90 hidden pilcrows and headings. A `;`
  inside a quoted at-rule prelude (a Google Fonts URL) no longer swallows the
  rule after it. [bl-3aef]

- **The fidelity oracle now reads three saved real pages** (dev-tree only;
  nothing shipped changes): a go.dev tutorial, a Bootstrap docs page and a
  Wikipedia article, each with its stylesheets, licensed in
  `tests/fixtures/NOTICE.md`. The Go page matches Chrome on every channel;
  the other two surfaced ten divergences, each filed and pinned in the
  ledger. [bl-5334]

- **`FileReader` and `ProgressEvent` exist under `--js`.** A `FileReader`
  reads a `Blob` as a data URL, text, an `ArrayBuffer` or a binary string, and
  fires `loadstart`, `progress`, `load` and `loadend` (or `abort`) in the order
  and with the payloads Firefox does. Text is decoded as UTF-8 only: a legacy
  `encoding` argument or `charset` is ignored. AWS WAF's challenge no longer
  throws on its first `new FileReader()`. [bl-8c46]

- **`--out ax`: `title` no longer names a `term`.** An empty `<dt title=T>`,
  `<dfn title=T>` or `role=term` is `term` with no name, as in Chrome; other
  roles still fall back to `title`. [bl-0794]

- **`--out ax`: `role=form` and `role=region` stand only on a named
  element**, as in Chrome. Unnamed, the next `role` token applies, else the
  element's implicit role: `<div role=region>` is generic,
  `<div role="form button">` is a `button`, `<nav role=region>` is still a
  `navigation`. An unknown first token is likewise skipped rather than voiding
  the attribute (`role="foo button"` is a `button`). [bl-c5a6]

- **`--out ax`: an empty, unnamed table `<caption>` emits no node**, as in
  Chrome. A caption with any element child, any text, or an author name
  (`aria-label`, `aria-labelledby`, `title`) keeps its `caption` node, and so
  do an empty `<figcaption>` and an empty `role=caption`. [bl-fe88]

- **`--out ax`: a `<table>` with no `<caption>` is named from its `summary`
  attribute**, as in Chrome. `aria-label` and a caption outrank it; it
  outranks `title`. [bl-2b09]

- **`Image` and `HTMLImageElement` exist under `--js`.** `new Image()` is an
  `<img>` element outside the tree; `new Image(w, h)` sets its `width` and
  `height`. frot still fetches no images: assigning `src` sends no request (a
  tracking-pixel beacon is not sent), neither `load` nor `error` fires, and
  `complete` stays `false` with a natural size of 0 — for every `<img>`,
  parsed or constructed. A page that waits for an image to load waits until
  the run ends. [bl-1404]

- **`--out ax`: an inline `<svg>` with nothing exposed inside it is an `img`**
  (was always `graphics-document`), as in Chrome. It stays a
  `graphics-document` when it holds text in a `<text>` or a described part.
  A described part — a `<g>`, shape, `<use>` or nested `<svg>` with a
  `<title>`/`<desc>` child or an author name — now has a node of its own
  (`group`, `graphics-symbol`, `graphics-object`), named from its `<title>`;
  inside a link or button it contributes that title and not its contents. An
  SVG `<desc>` no longer leaks into names, and gets no box in `bboxes`.
  [bl-07bd]

- **A vendored React bundle no longer ships in the crate.**
  `src/js/engine/react.production.min.js` — a second, unattributed React
  distribution read by one engine smoke test — is deleted; the test reads the
  licensed React 17.0.2 fixture instead, which is excluded from the package.
  The licensing gate now sweeps the whole tracked tree: every file under
  `tests/fixtures/` needs a `NOTICE.md` row, and no file outside it may carry
  a third-party licence banner. [bl-41e5]

- **`crypto.subtle` now does every secret-key operation Firefox does.** New
  under `--js`: AES-CBC and AES-CTR `encrypt`/`decrypt`; AES-GCM with any IV
  length and all seven tag lengths (it was 96-bit IV, 128-bit tag only);
  AES-KW; PBKDF2 and HKDF through `deriveBits`/`deriveKey`; and
  `wrapKey`/`unwrapKey` under any of the four AES modes — each with
  `importKey`/`exportKey` (`raw`, `jwk`) and `generateKey`. Results, key
  shapes and every error (name, message and which check wins) match Firefox
  153.0esr on 444 captured calls. Two limits: PBKDF2 is refused
  (`OperationError`) past 2 000 000 HMAC iterations × output blocks, where a
  browser would keep computing; and the public-key algorithms (RSA, ECDSA,
  ECDH, Ed25519, X25519) still reject `NotSupportedError`. Amazon's delivered
  home page imports an AES-CBC key, which no longer fails. [bl-db8f]

- **A bot challenge declared at an error status now gets the same treatment as
  any other declared challenge.** A response with status 400 or above that
  carries `cf-mitigated: challenge` or `x-amzn-waf-action: challenge` —
  Cloudflare serves its challenge as a 403 — was `status:"error"`,
  `http.403`, exit 1. It is now `needs:["js"]` without `--js`, and with `--js`
  frot runs the challenge page's script and re-requests once if it changed the
  request, exactly as below 400; `http.status` still reports the 403 and the
  envelope carries the `challenge` block. A 4xx/5xx that declares no challenge
  (including a 429/503 with `retry-after`) is unchanged. Measured: this does
  **not** yet get a Cloudflare-challenged page — its script needs an iframe
  window frot does not have. [bl-a7c5]

- **`addEventListener` / `removeEventListener` / `dispatchEvent` called with
  no receiver act on the window under `--js`**, as in a browser; they threw.
  [bl-a7c5]

- **`--out bboxes` gives a collapsed `<select>`'s options no box.** A select
  is a list box when it has `multiple` (unless `size=1`) or `size` of 2 or
  more; otherwise it is a drop-down, and nothing inside it is laid out — as
  in Chrome, where those options have no client rects. The select keeps its
  own (now empty) box; `text` and `ax` still list every option. The `ax` role
  follows the same rule, so `<select multiple size=1>` is now a `combobox`
  and `size=" 4 "` / `size=2abc` a `listbox`. [bl-2932]

- **The fidelity oracle now checks what it used to excuse** (dev-tree only;
  nothing shipped changes). The recorder reads string `::before`/`::after`
  content and `<textarea>` values from Chrome in paint position, and counts a
  box only where `checkVisibility()` holds, so nine ledger residuals — seven
  `text`, two `bboxes` — became plain equalities and were deleted. Captures
  re-recorded on Chrome 158. [bl-7966]

- **An accessible name computed from contents keeps block boundaries.** The
  contents of any child that is not a plain inline box (a block, table cell,
  list item, inline-block, flex container) and the line breaks `<br>`/`<wbr>`
  are spaced off from their neighbours, as in Chrome:
  `<button><div>a</div><div>b</div></button>` is "a b" (was "ab"), a grid row
  of `<td>`s is "G1 10" (was "G110"). Inline children still join: two
  `<span>`s are "in1in2". Under `--css` the box is the computed `display`.
  [bl-fb69]

- **`--css --out ax` no longer reorders flex children.** The AX tree is in DOM
  order in every recipe, as Chrome's is: flex `order` and
  `flex-direction: *-reverse` do not move AX nodes (measured, Chrome 156).
  `--out ax` builds no layout any more. `--out bboxes` keeps its visual
  reading order. [bl-7430]

- **`Blob`, `URL.createObjectURL` and `URL.revokeObjectURL` exist under
  `--js`.** A `Blob` is real bytes with `size`, `type`, `slice`, `text`,
  `arrayBuffer` and `bytes`. A blob: URL is minted but cannot be loaded (no
  `fetch` of it, and a Blob-URL `Worker` still never runs); there is no
  `stream()` or `File`. [bl-78c5]

- **`document.scripts` exists under `--js`**: the `HTMLCollection` of the
  document's `<script>` elements, as of the moment it is read. [bl-c296]

- **`requestIdleCallback` / `cancelIdleCallback` exist under `--js`.** An idle
  callback runs on the virtual clock once every timer due at that instant has
  run, and receives a real `IdleDeadline`. [bl-b584]

- **`TextEncoder` and `TextDecoder` exist under `--js`**, for UTF-8:
  `encode`, `encodeInto`, and a spec-exact `decode` (replacement characters,
  BOM, `fatal`, `{stream: true}`). Any other `TextDecoder` label is refused
  with a `RangeError` rather than mis-decoded. [bl-53b7]

- **`FormData` exists under `--js`, and a `fetch`/XHR POST can send one.** It
  goes out as the `multipart/form-data` text Firefox produces (same boundary
  format, same escaping), under the existing 64 KiB request-body cap.
  `new FormData(formElement)` is refused — frot does not read a form's
  controls. **With this, frot passes Amazon's declared challenge**: measured
  2026-11-17, `--js` returns the home page with
  `challenge:{rounds:1, passed:true}` every time Amazon declared its
  challenge — 11 runs of 15; the other 4 got an undeclared, textless 200
  (`docs/design/challenge.md` §11.4). [bl-6eb9]

- **`crypto.subtle` exists under `--js`, and it computes real values.** On a
  secure page (`https:`, `file:`, loopback) `crypto.subtle` is a Firefox-shaped
  `SubtleCrypto` with all twelve methods and a real `CryptoKey`. Implemented:
  `digest` (SHA-1/256/384/512); AES-GCM (128/192/256-bit keys, 96-bit IV,
  128-bit tag) and HMAC — `importKey`/`exportKey` as `raw` or `jwk`,
  `generateKey`, `encrypt`/`decrypt`, `sign`/`verify`. Everything else Firefox
  has (RSA, ECDSA/ECDH, Ed25519/X25519, AES-CBC/CTR/KW, HKDF, PBKDF2, other
  AES-GCM IV/tag lengths, `wrapKey`/`unwrapKey`) rejects with
  `NotSupportedError` — never a made-up result. Amazon's challenge script now
  gets through its key import and encryption; it still does not pass (it stops
  at `FormData` next — `docs/design/challenge.md` §11.4). [bl-0647]

- **`window.isSecureContext` exists under `--js`**, as it does in every
  Firefox: `true` on `https:`, `file:` and loopback `http:`, `false` on any
  other `http:` page. `crypto.randomUUID` is now absent on an insecure page,
  which is what Firefox 153 does (`docs/design/identity.md` §3.19). [bl-b0f8]

## [0.0.3](https://github.com/mudbungie/frot/compare/v0.0.2...v0.0.3) - 2026-11-18

### Changes

- build the declared-challenge round trip (challenge.md, signed off by Mark 2026-11-17) [bl-15ff]
- release PR shows no checks: a dispatched CI run stays off the PR's status rollup; restate its verdict as a commit status [bl-4de7]
- make package: cargo publish --dry-run leaves no .crate to leak-scan; CI package step is red [bl-f28a]
- an unnamed <form> is omitted; Chrome exposes it as role form with an empty name [bl-2a3c]
- full CI/CD: fix release-plz PR CI, tags, GitHub Release + musl asset, crates.io publish path [bl-5cf3]
- an inline <svg> is not named from its <title> child (Chrome: "Chart title") [bl-0a0a]
- Adopt the leak-scan stack and wire store publication [bl-59dc]
- <dt> (term) has no name; Chrome names it from its contents (term "Term") [bl-8ad7]
- a row is named from its cells, fused with no separator ("Q110"); Chrome names a row "" [bl-3072]
- <tbody> emits a rowgroup node; Chrome ignores tbody (explicit or implied) and keeps only <thead> [bl-e760]
- a <table> is not named from its <caption> (Chrome: table "Quarterly totals") [bl-fd85]
- HTTP `Refresh:` response header is not followed (sibling of the meta refresh, bl-08d8) [bl-e52a]
- build the fidelity oracle (fidelity.md, signed off by Mark 2026-11-17) [bl-2e08]
- zero-delay meta refresh returns an empty ok impression [bl-08d8]
- renew the persona expiry pin (eol 2027-01-26) from Mozilla's published calendar [bl-d8f1]
- Collapse the pre-commit gate to exec bl-gate (rollout phase 2; ops bl-3166) [bl-8a8b]
- Remote gate: make check is the whole gate; pre-commit delegates to the noodlezoo builder [bl-254b]
- host objects with no interface own their whole surface as instance data [bl-643d]
- the 300-line cap does not see the JS prelude, and elem2.js is already over it [bl-6da7]
- instance backing slots are visible own properties; a real Firefox instance has none [bl-3bdc]

- **frot now answers a server-declared bot challenge the way a browser does
  unattended — and tells you it did.** When a response declares itself a
  challenge (`x-amzn-waf-action: challenge`, `cf-mitigated: challenge`, or
  `retry-after` on a success response), `--js` now runs the challenge page's own
  script. If that script changes the request frot would send (it mints a
  cookie), frot requests your URL **once** more and returns what comes back. If
  it changes nothing, or the answer is still a challenge, the result is
  `needs: ["human"]` as before. frot never clicks, types, submits a form or
  solves a CAPTCHA; the 1 s CPU and 1 s network budgets are unchanged and are
  shared by the challenge and the page behind it. (`[bl-15ff]`, design
  `docs/design/challenge.md`)
  - **New envelope block `challenge`**, present only when a challenge was
    declared: `"challenge":{"declared":"x-amzn-waf-action","rounds":1,"passed":true}`.
    `declared` is the response header that declared it, `rounds` the extra
    navigations taken (0 or 1), `passed` whether the last one was no longer a
    challenge. It rides `ok`, `needs` and `error` envelopes alike.
  - **Changed verdict:** a declared challenge fetched **without** `--js` is now
    `needs: ["js"]` (it was `needs: ["human"]`), because adding `--js` is the
    recipe that can answer it. With `--js`, an unanswered challenge is still
    `needs: ["human"]`, now with a `js` block describing the script's run.
  - **Page scripts can POST.** `fetch` and `XMLHttpRequest` accept method
    `POST` with a text body (a string or `URLSearchParams`) up to 64 KiB; it is
    sent with the script's `Content-Type` and an `Origin`, and its response is
    never cached. Every other method is still rejected, a non-text body fails
    the request, and frot's own navigation is always a GET.
  - **Pages that declare no challenge are unchanged**, byte for byte, and make
    no extra request.
  - **Measured, not promised:** on 2026-11-17 this did **not** get Amazon's
    page. The AWS WAF script cannot finish in frot's JS environment (it needs
    `crypto.subtle`), so no round is taken and the result is `needs: ["human"]`
    with `rounds: 0`. A Cloudflare challenge served with status 403 stays an
    `http.403` error and is not attempted.

- **`ax`: an unnamed `<form>` is now a `form` node.** `<form><input><button>
  Send</button></form>` used to put the textbox and button at the level of the
  form's parent; they are now children of a `form` with a null name, as in
  Chrome's tree — at top level, inside `<search>` or `<nav>`, or empty. This
  reverses the earlier named-only rule for forms: the HTML-AAM and ARIA make a
  form a *landmark* only when named, but the browser keeps the node either
  way, and frot reports the tree. Consumers counting `form` nodes as landmarks
  should now require a name. `<section>` is unchanged (unnamed, no node), and
  an author `role=` on a form still replaces the role. Found by the fidelity
  oracle. (`[bl-2a3c]`)

- **`ax`: an inline `<svg>` is now named from its `<title>`.** `<svg><title>
  Chart title</title>…</svg>` was `graphics-document` with a null name; it is
  `graphics-document "Chart title"`, as Chrome reports it. The title is the
  first `<title>` *child* — one nested under a `<g>` belongs to the `<g>` —
  and `aria-label`/`aria-labelledby` still win. The same rule now governs what
  a graphic contributes to the name of a link or button around it, with two
  consequences there: a `<title>` that is not a child no longer stands in for
  the whole graphic (its text is still read, along with the rest of the
  graphic's text), and an empty `<title>` no longer silences the graphic's
  text. Found by the fidelity oracle. (`[bl-0a0a]`)

- **`ax`: a `term` is now named from its contents.** `<dt>Term</dt>` was
  `term` with a null name; it is `term "Term"`, as Chrome reports it, and the
  same goes for `<dfn>` and `role=term`. ARIA makes `term` a name-from-author
  role; the browser names it from contents anyway, and frot follows the
  browser. `definition` (`<dd>`) is still unnamed, as in Chrome. Found by the
  fidelity oracle. (`[bl-8ad7]`)

- **`ax`: a row of a plain table no longer has a name.** `<tr><th>Q1</th>
  <td>10</td></tr>` was `row "Q110"` — its cells' text, fused; it is now a
  `row` with a null name, as in Chrome, and the cells keep their own names. A
  row is still named from its cells inside a `grid` or `treegrid`, where
  Chrome names it too, and `aria-label`/`aria-labelledby`/`title` on a row
  count everywhere. A `<td>`'s `cell`/`gridcell` choice now reads the same
  one fact — the nearest ancestor with a table role — which differs from
  before only for a table given a non-table `role=` nested inside a grid.
  Found by the fidelity oracle. (`[bl-3072]`)

- **`ax`: an unnamed `<tbody>` no longer emits a `rowgroup`.** The rows of a
  table body — written out or implied by the parser — are now direct children
  of the `table`, as in Chrome's tree; before, every data table carried a
  `rowgroup` wrapper Chrome does not have. `<thead>` and `<tfoot>` are still
  `rowgroup`s, and so is a `<tbody>` the author named (`aria-label`,
  `aria-labelledby`, `title`) or gave `role=rowgroup`. This departs from the
  HTML-AAM's flat `tbody` → `rowgroup` mapping in favour of what the browser
  exposes. Found by the fidelity oracle. (`[bl-e760]`)

- **`ax`: a `<table>` is now named from its `<caption>`.** `<table><caption>
  Quarterly totals</caption>…` was `table` with a null name; it is
  `table "Quarterly totals"`, as Chrome reports it. The first `<caption>` child
  is read the way any descendant is read into a name — its `alt`s and
  `aria-label` count, a hidden caption says nothing — and `aria-label` /
  `aria-labelledby` on the table still win. The `caption` node itself is
  unchanged. Found by the fidelity oracle. (`[bl-fd85]`)

- **An immediate `Refresh:` response header is now followed, as the
  `<meta http-equiv="refresh">` already was.** A response carrying
  `Refresh: 0; url=/real` sends a browser to `/real` unattended — the same HTML
  rule as the meta, spelled as an HTTP header — but frot ignored the header and
  returned the interim page (often an empty `ok`). It is now the same rule and
  the same hop: redirect budget, cookies, `Referer`, `url.final` all as for the
  meta. When a response declares both, the header is read first (it exists
  before the markup does), so an immediate header wins and a delayed header
  means nothing is followed. The header's relative target resolves against the
  page URL, not a `<base>`. No flag, field or exit code changed. (`[bl-e52a]`)

- **A page that is only an immediate `<meta http-equiv="refresh">` now returns
  the page it points at, not an empty `ok`.** `frot https://docs.astro.build/`
  returned `status: ok` with nothing in `out`: the root document is a
  zero-delay refresh to `/en/getting-started/`, which a browser follows with no
  script and no human. frot now follows it as it follows an HTTP redirect — same
  connection pool and cookies, the refreshing page as `Referer`, `url.final`
  reporting where it landed — and shares the ten-request redirect budget, so a
  refresh loop ends in the existing `fetch.redirect` error. A refresh with a
  delay above zero is not followed (that page is what a browser shows first),
  nor is one inside `<noscript>`, one a script inserts, or one whose target is
  not `http(s)`. Under `--js` the refresh is taken before any script runs. No
  flag, field or exit code changed. (`[bl-08d8]`, decided in `[bl-c262]`)

- **The Firefox 153 ESR persona's expiry moved from 2027-01-26 to 2027-09-07.**
  The build fails on the day the pinned browser line passes the last release
  Mozilla has scheduled for it. Mozilla's calendar now runs 153.x through 153.27
  on 2027-09-07 (read 2026-11-17), with ESR 175 beside it from 2027-06-29. No
  wire or script-visible value changed. (`[bl-d8f1]`)

- **frot's views are now checked against a real browser on every test run.**
  `tests/fidelity.rs` compares `text`, `ax`, `links`, `forms` and `bboxes`
  with what Chrome answered about the same pages, from captures recorded once
  by `scripts/fidelity-record` and checked in — so the comparison needs no
  browser and no network. Agreement is exact, not scored: each view is
  compared only on what frot claims to produce, and every known difference is
  listed with its reason or its open ball. No behaviour of the binary changes.
  (`[bl-2e08]`)

- **Under `--js`, a DOM object no longer carries frot's implementation on its
  face.** Every instance a page can reach — an element, a `Notification`, an
  audio node, a canvas or WebGL context, an observer, a `URL` — kept its state
  in `_`-prefixed own properties, which `Object.getOwnPropertyNames` reports
  whether or not they are enumerable. A real Firefox instance reports nothing
  there (measured); frot's `Notification` reported ten entries. The state moved
  behind the prototype accessors WebIDL puts it behind, so anything walking an
  instance now sees a browser's shape. (`[bl-3bdc]`)

## [0.0.2](https://github.com/mudbungie/frot/releases/tag/v0.0.2) - 2026-08-12

> **`0.0.0` and `0.0.1` are withdrawn — upgrade to `0.0.2`.**
> Both of those releases packaged design documents that recorded personal
> network identifiers belonging to the author: the egress address of a field
> trial and the geolocation derived from it. They were measurement notes; they
> should never have been published. The identifiers are gone from the tree, the
> affected versions are being yanked from crates.io, and `0.0.2` is the first
> clean release. A published crates.io version can never be edited or replaced,
> so the fix ships as a new version rather than a corrected `0.0.1`. If you have
> `0.0.0` or `0.0.1` pinned or vendored, move to `0.0.2` — the upgrade itself
> changes no behaviour, and everything below is a bug fix on top of it.

### Changes

- **Personal network identity is scrubbed, and gated against return** — the
  packaged design docs no longer carry an egress address or its geolocation
  gloss, and `tests/hygiene/` fails the build if a globally-routable IPv4
  literal or a bare autonomous-system number reappears in any tracked file.
  (`[bl-f521]`, `[bl-f90c]`, `[bl-5638]`)
- **Vendored test fixtures carry their licence and attribution**, checked by
  the same hygiene suite. (`[bl-11f5]`)
- **Public docs match the product** — corrected claims about cookies, phase
  status and challenge passing; removed directional refusal language.
  (`[bl-6f85]`, `[bl-0af9]`)
- **`--out text` and `--out ax` report what a browser actually exposes:**
  - a closed `<details>` body is no longer emitted as visible text, with or
    without `--css` (`[bl-74a6]`, `[bl-d470]`);
  - SVG and MathML text a browser paints is no longer omitted (`[bl-c0a4]`);
  - adjacent `<option>` texts no longer fuse into one token (`[bl-66ed]`);
  - fallback content for `<video>`, `<object>`, `<canvas>` and `<picture>` is
    no longer emitted as page content, and no longer names its ancestor
    (`[bl-0f83]`, `[bl-0aaf]`, `[bl-e79a]`);
  - hidden fallback text no longer masks a failed SPA as status `ok`
    (`[bl-eeb4]`);
  - accessible names: `title` is a name source, `aria-labelledby` resolves at
    an element that has its own alternative, `<optgroup label>` and
    `<option label>` name their elements, and name-from-contents descends into
    descendant text alternatives (`[bl-d8ff]`, `[bl-0482]`, `[bl-4093]`,
    `[bl-3d2e]`);
  - the AX tree honours `aria-hidden` and `inert` subtree exclusion, and
    implicit roles honour HTML context and conditions (`[bl-2fa6]`,
    `[bl-a189]`).
- **CSS and geometry** — inline descendant bboxes keep their containing `x`
  origin (`[bl-2161]`); a linked stylesheet's `media` is honoured, so print CSS
  no longer applies on screen (`[bl-4f0a]`); generated pseudo-content
  contributes layout geometry and its Unicode escapes are decoded instead of
  emitted literally (`[bl-6fcb]`, `[bl-e94a]`).
- **`--js` gets through more real-world pages:**
  - `document.currentScript` exists, so Turbopack chunk loaders work
    (`[bl-a19d]`);
  - classic scripts are no longer forced into strict mode (`[bl-0679]`);
  - `Element.id` and four further DOM properties are writable, as in a browser
    (`[bl-d313]`, `[bl-273b]`);
  - Astro pages no longer abort on DOM child insertion or in engine teardown
    (`[bl-ae88]`, `[bl-5249]`);
  - a starved heap no longer segfaults during parse (`[bl-c385]`);
  - `js.stopped` names the bound that actually ended an unsettled run — the
    compute budget or the network one (`[bl-79dc]`).
- **Browser-surface fidelity re-measured against a real Firefox** — interface
  prototypes and their own-property lists, `IDBKeyRange` range semantics and
  absent statics, `Worker.prototype`'s flattened `EventTarget`, an undeclared
  `crypto.subtle`, the 2D canvas surface, `navigator.appVersion`, and the
  remaining capability surfaces are re-pinned to the measured profile, and the
  profile's end-of-life date is corrected. (`[bl-706b]`, `[bl-6438]`,
  `[bl-d22f]`, `[bl-1ab7]`, `[bl-b128]`, `[bl-6491]`, `[bl-3595]`, `[bl-04ab]`,
  `[bl-7523]`)
- **Fetching and URL resolution** — a relative `<base href>` resolves links
  (`[bl-409e]`); a fetch with no redirect reports the resolved final URL rather
  than the raw spelling (`[bl-2832]`); a binary image response is no longer
  parsed as HTML and reported `ok` (`[bl-0c3e]`); `data:` script sources are no
  longer counted as failed subfetches (`[bl-91bf]`).
- **Repo gates** — the formatting and 300-line source caps are keyed on tracked
  files rather than the staged set, so neither can be skipped at delivery; a
  golden that measured real wall time is deterministic; dead code removed; the
  release-plz branch prune no longer fails open and deletes every release
  branch. (`[bl-0066]`, `[bl-a68b]`, `[bl-c81a]`, `[bl-d673]`, `[bl-a237]`)

## [0.0.1](https://github.com/mudbungie/frot/releases/tag/v0.0.1) - 2026-08-11

The first release meant to be installed. `0.0.0` existed only to register the
crate name and should not be used.

> **Changed behaviour — a lost impression is now a failed exit status.**
> Until now, if frot could not write its envelope to stdout, it still exited
> `0`: the product was gone and nothing said so. From `0.0.1`, a broken pipe
> kills frot with `SIGPIPE` (a shell reports `141`) and any other stdout write
> failure exits `3` with a diagnostic on stderr. If you pipe frot into a
> consumer that can close early (`frot … | head -1`) and you treat a zero exit
> as "impression taken", that assumption no longer holds — check the status.

### Changes

- **Exit status: stdout write failures are detected** — `EPIPE` dies by
  `SIGPIPE` (`141`), every other write failure exits `3`, diagnostic on stderr.
  Previously the envelope could be silently lost under a successful exit
  status. (`[bl-34fb]`)
- **Argv: the `--` end-of-options delimiter is accepted** — everything after
  `--` is an operand, so a URL or path that starts with `-` can be passed
  (POSIX XBD 12.2 Guideline 10). (`[bl-28d1]`)
- **The CLI contract is written down and gated** — `docs/design/posix.md` pins
  the profile (POSIX.1-2017) frot claims for argv syntax, streams, exit
  statuses, signals and lifecycle, and `scripts/posix-suite.sh` (`make posix`)
  tests the binary against it. (`[bl-5382]`)
- **`--js`: real client-rendered React and Vue apps now settle** instead of
  coming back `needs-js`. The environment gained the surface deployed bundles
  actually touch:
  - `NodeList` and `HTMLCollection` as named interfaces, returned by every DOM
    query (`[bl-e5c3]`);
  - element breadth — `focus`/`blur`/`document.activeElement`, `DOMTokenList`
    for `classList` and `relList` (including `supports()`), `createElementNS`,
    `closest`, sibling navigation (`[bl-3a36]`);
  - `MutationObserver`, implemented at the mutation seam so it observes real
    tree changes; `IntersectionObserver` and `ResizeObserver` deliver a genuine
    initial batch (`[bl-07ab]`);
  - comment nodes are real nodes in the tree, so frameworks that navigate from
    comment anchors (Vue's `RouterView`, `v-if`) work (`[bl-79db]`);
  - the event contract — `DocumentFragment` is an `EventTarget`, the global is
    an `EventTarget`, and `AbortController`/`AbortSignal` exist (`[bl-e81b]`).
- **Licence text ships with the crate** — `LICENSE-MIT` and `LICENSE-APACHE`
  are in the tree and in the package, matching the declared
  `MIT OR Apache-2.0`. (`[bl-cd36]`)
- **Crate description corrected** — the old tagline claimed frot never executes
  or renders, which `--js` and `--out bboxes` contradict; it now says scripts
  run only when you ask and frot never drives the page. (`[bl-419f]`)
- **Release pipeline publishes to crates.io** and no longer uses Node-20
  actions. (`[bl-a498]`)
