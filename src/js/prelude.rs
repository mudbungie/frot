//! The bundled JS prelude (js.md §3) — the web-facing API, written in JS on top
//! of the syscall table so shim breadth grows without widening the Rust
//! interface. Files are concatenated and evaluated once, before any page
//! script, by [`super::syscall::install`].
//!
//! Coverage note (js.md §3): `cargo llvm-cov` cannot see these JS lines; the
//! prelude is exercised end-to-end by the golden fixture suite (subtask 9). The
//! Rust syscall closures it calls are covered directly in `syscall::tests`.
//!
//! This is the **core** prelude plus the framework-facing Element/Node breadth
//! (`elem.js`/`elem2.js`: the `instanceof` interface constructors, `style`,
//! `dataset`, faithful `cloneNode`, form-control reflections, and
//! the DOMTokenList surface — `classList`/`relList` — in `tokenlist.js`, and
//! the `document`/`createDocumentFragment` breadth React/Vue/jQuery probe in
//! `doc.js`), the
//! geometry facade, the §7 environment breadth, the §5 event loop, and the §6
//! network layer: `Node`/`Element`/`Document`, `querySelector`,
//! the HTML-string direction pair `innerHTML`/`outerHTML` and its one
//! serializer (`markup.js`), `textContent`, `console`, the
//! geometry facade (`getBoundingClientRect`/`offset*`/`getComputedStyle`, js.md
//! §8), the environment shims (`env.js`: cookie, `self`/`window` aliasing, the
//! viewport facts, spec-legal denials — with the four ambient objects that are
//! real interfaces, `Storage`/`Location`/`History`/`MediaQueryList`, in
//! `envobj.js`), the
//! WHATWG-subset `URL`/`URLSearchParams` (`url.js`, over the `__frot_url_parse`
//! syscall), the
//! event registry and interfaces (`events.js`: `EventTarget`/`Event`/
//! `CustomEvent`, `addEventListener`/`dispatchEvent`), the virtual-clock loop
//! (`loop.js`: `setTimeout`/`setInterval`/`requestAnimationFrame` and the
//! `DOMContentLoaded`/`load` lifecycle — WHEN the host fires, not what an event
//! IS), and `fetch`/`XMLHttpRequest` over the
//! once-then-frozen subfetch cache (`net.js`, js.md §6).

/// **Cite the document, not just the section.** `identity.md` and `js.md` both
/// number their sections 1-13, and the same number means different things in
/// each — §11 is *Residuals* in one and *Non-goals* in the other. A bare `(§11)`
/// once sent an auditing agent to the wrong document and had it report a
/// correctly-declared residual as undeclared (`bl-6438`), so every section
/// reference in this module and the prelude names its file.
///
/// The concatenated prelude source. Each module is an IIFE over `globalThis`, so
/// order matters only where one module extends another's globals: `elem.js`/
/// `elem2.js` (and `doc.js`, `env.js`, `loop.js`) run after `dom.js` because
/// they extend the `document` and `Node` it defines — `elem2.js` after
/// `elem.js`, `doc.js` after `elem2.js` — and
/// `net.js` runs after `events.js` because its XHR uses `g.Event`. `url.js` is
/// standalone (it extends no other module, only the `__frot_url_parse` syscall),
/// so its position is free.
pub const SOURCE: &str = concat!(
    // brand.js FIRST: the one Function#toString wrapper + branding registry every
    // later module (and the six later capability balls) registers through (§8).
    include_str!("prelude/brand.js"),
    "\n",
    // iterator.js next: replaces the engine's two process-aborting iterator
    // helpers (bl-5249). Needs only brand.js, and must precede any page script.
    include_str!("prelude/iterator.js"),
    "\n",
    include_str!("prelude/console.js"),
    "\n",
    include_str!("prelude/dom.js"),
    "\n",
    // markup.js after dom.js: it owns the HTML-string <-> arena direction pair
    // (`innerHTML`/`outerHTML`) over the Node prototype dom.js defines (bl-273b).
    include_str!("prelude/markup.js"),
    "\n",
    include_str!("prelude/elem.js"),
    "\n",
    include_str!("prelude/elem2.js"),
    "\n",
    // DOMTokenList (bl-3a36): classList/relList as the spec-named interface —
    // after elem2.js (extends the same Node prototype), before anything reads
    // classList.
    include_str!("prelude/tokenlist.js"),
    "\n",
    include_str!("prelude/geometry.js"),
    "\n",
    // events.js before every module that is an EventTarget: it owns the ONE
    // listener registry and the EventTarget interface whose prototype
    // permissions.js, worker.js, idb.js and loop.js all reach (bl-6438). Needs
    // only brand.js and dom.js.
    include_str!("prelude/events.js"),
    "\n",
    // doc.js (bl-6da7): the `document` object's own breadth and the
    // DocumentFragment it hands out, split off elem2.js on the seam that file
    // already named. AFTER events.js, because the staging fragment is an
    // EventTarget and inherits that interface (bl-643d); elem2.js's
    // appendChild/insertBefore drain a fragment through the `drain` slot doc.js
    // sets, and both are loaded before any page script can build one.
    include_str!("prelude/doc.js"),
    "\n",
    // The identity surface (bl-3972), derived from __frot_env_profile: after
    // dom.js (extends document/Node), any order among themselves.
    include_str!("prelude/navigator.js"),
    "\n",
    // Permissions API + Notification (bl-1548): coherent presence, no grant. AFTER
    // navigator.js because it extends Navigator.prototype (navigator.permissions).
    include_str!("prelude/permissions.js"),
    "\n",
    include_str!("prelude/screen.js"),
    "\n",
    include_str!("prelude/intl.js"),
    "\n",
    include_str!("prelude/crypto.js"),
    "\n",
    // Worker/SharedWorker (bl-342a): coherent constructor presence, no thread —
    // needs only brand.js, so its position among the identity surface is free.
    include_str!("prelude/worker.js"),
    "\n",
    include_str!("prelude/env.js"),
    "\n",
    // The environment's four ambient objects as interfaces (bl-643d): Storage,
    // Location, History, MediaQueryList. Split out of env.js; after events.js
    // because MediaQueryList inherits its EventTarget.
    include_str!("prelude/envobj.js"),
    "\n",
    include_str!("prelude/url.js"),
    "\n",
    include_str!("prelude/loop.js"),
    "\n",
    // Abort surface (bl-e81b): AbortController/AbortSignal over events.js's
    // EventTarget and loop.js's virtual-clock setTimeout.
    include_str!("prelude/abort.js"),
    "\n",
    // Focus management (bl-3a36): activeElement + focus()/blur() — after
    // events.js, because moving focus dispatches blur/focus through
    // Node.dispatchEvent.
    include_str!("prelude/focus.js"),
    "\n",
    // Observers (bl-07ab). observer.js is the GENUINE MutationObserver seam: it
    // captures the five raw mutation syscalls and republishes them wrapped, so
    // it must load after every module that calls them by global name at run
    // time (all do — none captures the raw functions) and before page scripts;
    // its notify path uses g.reportError (loop.js). observer2.js is the
    // MutationObserver interface over observer.js's __frot_mo_hook seam (split
    // under the 300-line cap). viewobserver.js is the genuine-initial-delivery
    // IntersectionObserver/ResizeObserver over the §8 geometry syscall,
    // delivering on loop.js's timer queue.
    include_str!("prelude/observer.js"),
    "\n",
    include_str!("prelude/observer2.js"),
    "\n",
    include_str!("prelude/viewobserver.js"),
    "\n",
    include_str!("prelude/net.js"),
    "\n",
    // IndexedDB (bl-8dde): coherent IDBFactory + interface-zoo presence, no
    // persistence. AFTER events.js because IDBVersionChangeEvent subclasses its
    // `g.Event`; open()/deleteDatabase() hand out a permanently-pending request.
    include_str!("prelude/idb.js"),
    "\n",
    // Canvas 2D fingerprint (bl-05e6): a coherent, deterministic simulation.
    // canvaspng.js (the PNG serialiser) first, then canvasexp.js (the two digest
    // expansions a page reads back — the bitmap and the twelve TextMetrics fields —
    // plus the ImageData they arrive in), then canvas.js (the context: state
    // properties, draw API, element bridge, and the digest fold seeded by the
    // profile's fixed `canvasSeed`), then canvaselem.js (getContext/toDataURL on the
    // element). After dom.js (extends Node) and env.js is immaterial — it reads the
    // always-bound profile syscall.
    include_str!("prelude/canvaspng.js"),
    "\n",
    include_str!("prelude/canvasexp.js"),
    "\n",
    include_str!("prelude/canvas.js"),
    "\n",
    include_str!("prelude/canvaselem.js"),
    "\n",
    // WebGL fingerprint (bl-f624): a coherent, deterministic simulation. webglpix.js
    // (the shared FNV digest + xorshift bitmap expansion) first, then webgl.js (the
    // branded WebGL/WebGL2 contexts, masked VENDOR/RENDERER, coherent Mesa/llvmpipe
    // UNMASKED via WEBGL_debug_renderer_info, limits/extensions/precision, and the
    // deterministic readPixels/toDataURL). After canvaselem.js (which routes
    // getContext('webgl'…) here) and canvaspng.js (reuses its PNG serialiser).
    include_str!("prelude/webglpix.js"),
    "\n",
    include_str!("prelude/webgl.js"),
    "\n",
    // Web Audio fingerprint (bl-8733): a coherent, deterministic simulation.
    // audiobuf.js (the FNV digest + xorshift FLOAT expansion + the branded-class
    // helper) first, then audionode.js (AudioParam, the AudioNode zoo, AudioBuffer,
    // and the node/buffer factories), then audio.js (BaseAudioContext /
    // AudioContext / OfflineAudioContext, the create* API, and startRendering whose
    // rendered buffer is a deterministic function of the graph digest). After
    // events.js because OfflineAudioCompletionEvent subclasses its Event; it reads the
    // always-bound profile syscall for the `audio` SSOT facts.
    include_str!("prelude/audiobuf.js"),
    "\n",
    include_str!("prelude/audionode.js"),
    "\n",
    include_str!("prelude/audio.js"),
    "\n",
    // nativebrand.js LAST: sweeps the whole existing web-API surface into the
    // brand registry so no function's toString discloses prelude source (§8).
    include_str!("prelude/nativebrand.js"),
);
