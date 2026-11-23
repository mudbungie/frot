// Environment shims (js.md §7) — no persistence. What is left here is what is
// NOT an interface: the `self`/`window` aliasing, the document.cookie bridge onto
// the Rust jar, and the viewport facts. The four ambient OBJECTS that used to
// live here — localStorage/sessionStorage, location, history, and matchMedia's
// return — moved to envobj.js when they became real interfaces with their
// members on a prototype in Gecko's order (bl-643d); the navigator/screen/Intl/
// crypto identity surface moved to its own modules (bl-3972). Layered on the
// __frot_cookie_* / __frot_viewport_* syscalls; runs after dom.js so it can
// extend `document` and `Node`.
(function (g) {
  'use strict';
  var slots = g.__frot_slots;

  // --- self/window aliasing (js.md §1 spike, subtask 8) ---------------------
  // The global *is* the window: UMD bundles probe `self`, framework/router code
  // reads `window.*`, frame-busters compare `top`/`parent` to `self`. A top
  // window is its own parent; a frame's opener overwrites both, and
  // `frameElement`, when it opens it (frame.js, js.md §14).
  g.self = g;
  g.window = g;
  g.top = g;
  g.parent = g;
  g.frameElement = null;

  // --- isSecureContext (bl-b0f8) ------------------------------------------------
  // Firefox ALWAYS defines it, true or false; frot defined nothing. Shape read
  // off 153.0esr (identity.md §3.19): an OWN accessor of the window —
  // enumerable, configurable, no setter, getter named `get isSecureContext`.
  // The value is the host's one answer (`syscall/env/secure.rs`), the same one
  // crypto.js gates its `[SecureContext]` members on.
  Object.defineProperty(g, 'isSecureContext', {
    get: g.__frot_brand(function () {
      return g.__frot_secure_context();
    }, 'get isSecureContext'),
    enumerable: true,
    configurable: true,
  });

  // --- document.cookie: the one shared jar (bl-6dad, identity.md §9) ---------
  // Not a second in-memory string: get/set delegate to the Rust cookie jar the
  // transport owns (__frot_cookie_get/set at the final document URL). So a
  // Set-Cookie from the document GET is visible here iff non-HttpOnly, a JS
  // write feeds a later same-origin GET, and HttpOnly never enters JS.
  Object.defineProperty(g.Document.prototype, 'cookie', {
    configurable: true,
    get: function () {
      return g.__frot_cookie_get();
    },
    set: function (v) {
      g.__frot_cookie_set(String(v));
    },
  });

  // --- navigator / screen / Intl / crypto: the identity surface (§7, §8) -----
  // Moved out of this file (bl-3972): every navigator/screen/Intl/crypto fact now
  // derives from the one BrowserProfile SSOT through __frot_env_profile, so no
  // identity literal lives here and the JS persona cannot contradict the wire
  // (identity.md §4/§8). See navigator.js / screen.js / intl.js / crypto.js.

  // --- viewport & width facts (§7/§8) ---------------------------------------
  // A fixed 1280×720 viewport (layout.rs VIEWPORT_WIDTH/VIEWPORT_HEIGHT). Pages
  // gate desktop chrome on width — mdbook reads window.innerWidth, and a NaN
  // compare takes the mobile branch and strips its sidebar — so the widths frot
  // already knows are exposed honestly. window inner == outer (no browser
  // chrome). Element clientWidth/clientHeight == offsetWidth/offsetHeight
  // (borderless, scrollbar-less model, §8). The documentElement reports the
  // viewport itself, as browsers do — the primary way a page reads viewport size.
  var VW = g.__frot_viewport_width();
  var VH = g.__frot_viewport_height();
  g.innerWidth = g.outerWidth = VW;
  g.innerHeight = g.outerHeight = VH;
  g.__frot_ifaceattrs(g.Element.prototype, {
    clientWidth: function () {
      return this === g.document.documentElement ? VW : g.__frot_rect(slots(this).id)[2];
    },
    clientHeight: function () {
      return this === g.document.documentElement ? VH : g.__frot_rect(slots(this).id)[3];
    },
  });

  // canvas `getContext`/`toDataURL` are the 2D-fingerprint masquerade, owned by
  // canvas.js (bl-05e6): getContext('2d') returns a branded, deterministic
  // context; getContext('webgl') stays null (bl-f624). Not defined here.
  // WebSocket, EventSource, WebAssembly, navigator.serviceWorker: never defined
  // -> `typeof` is 'undefined', so feature detection falls through (§6, §7).
  // Absence is the denial; no code lies otherwise. (indexedDB and Worker/
  // SharedWorker are instead a coherent PRESENCE masquerade — idb.js/worker.js.)
})(globalThis);
