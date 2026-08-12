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
  // reads `window.*`, frame-busters compare `top`/`parent` to `self`. One arena,
  // no frames (js.md §11), so they all point at the single global.
  g.self = g;
  g.window = g;
  g.top = g;
  g.parent = g;
  g.frameElement = null;

  // --- document.cookie: the one shared jar (bl-6dad, identity.md §9) ---------
  // Not a second in-memory string: get/set delegate to the Rust cookie jar the
  // transport owns (__frot_cookie_get/set at the final document URL). So a
  // Set-Cookie from the document GET is visible here iff non-HttpOnly, a JS
  // write feeds a later same-origin GET, and HttpOnly never enters JS.
  Object.defineProperty(g.document, 'cookie', {
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
  if (g.Node) {
    Object.defineProperty(g.Node.prototype, 'clientWidth', {
      get: function () {
        var de = g.document.documentElement;
        return de && slots(this).id === slots(de).id ? VW : this.offsetWidth;
      },
    });
    Object.defineProperty(g.Node.prototype, 'clientHeight', {
      get: function () {
        var de = g.document.documentElement;
        return de && slots(this).id === slots(de).id ? VH : this.offsetHeight;
      },
    });
  }

  // canvas `getContext`/`toDataURL` are the 2D-fingerprint masquerade, owned by
  // canvas.js (bl-05e6): getContext('2d') returns a branded, deterministic
  // context; getContext('webgl') stays null (bl-f624). Not defined here.
  // WebSocket, EventSource, WebAssembly, navigator.serviceWorker: never defined
  // -> `typeof` is 'undefined', so feature detection falls through (§6, §7).
  // Absence is the denial; no code lies otherwise. (indexedDB and Worker/
  // SharedWorker are instead a coherent PRESENCE masquerade — idb.js/worker.js.)
})(globalThis);
