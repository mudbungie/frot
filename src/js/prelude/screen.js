// screen / devicePixelRatio / visibility (identity.md §8, js.md §7/§8, bl-3972 /
// bl-1cb7). Geometry derives from the ONE layout viewport constant (1280×720,
// __frot_viewport_*), never a second screen constant — depth/DPR from the persona
// profile. Everything is coherent by construction: screen == avail == the
// viewport (no chrome, no taskbar in frot's model), DPR 1, 24-bit colour, and a
// visible/focused document, so a page cannot catch screen and window disagreeing.
// Runs after brand.js (needs __frot_iface) and dom.js (extends document).
(function (g) {
  'use strict';
  var onEvent = g.__frot_onevent;
  var P = JSON.parse(g.__frot_env_profile());
  var W = g.__frot_viewport_width();
  var H = g.__frot_viewport_height();

  // A ScreenOrientation shim: a fixed landscape-primary desktop. Shape READ off
  // Firefox 153.0esr (`bl-1ab7`, identity.md §3.12) — the prototype carries
  // `lock`, `unlock`, `type`, `angle`, `onchange`, in that order, and frot
  // published only two of them, so feature detection could see the difference.
  var Orientation = g.__frot_iface('ScreenOrientation', {
    type: function () { return 'landscape-primary'; },
    angle: function () { return 0; },
  });
  // `lock()` rejects: with a TypeError when the required argument is missing and
  // otherwise `SecurityError: The operation is insecure.` outside fullscreen —
  // both measured, and frot is never in fullscreen, so the rejection is honest
  // rather than a refusal frot invented. `unlock()` returns undefined.
  Orientation.prototype.lock = g.__frot_brand(function lock(o) {
    if (arguments.length < 1) {
      return Promise.reject(new TypeError(
        'ScreenOrientation.lock: At least 1 argument required, but only 0 passed'));
    }
    return Promise.reject(g.__frot_domerror('SecurityError', 'The operation is insecure.'));
  }, 'lock');
  Orientation.prototype.unlock = g.__frot_brand(function unlock() {}, 'unlock');
  onEvent(Orientation.prototype, 'onchange');
  var orientation = Object.create(Orientation.prototype);

  // Own-property ORDER matches a real `Screen.prototype` (measured): the two
  // legacy `moz*` methods first, then the metrics, then `top`/`left`, then the
  // `moz*` orientation pair. Gecko still exposes all six; a Screen without them
  // is a Screen no Firefox ships, and they are exactly what a feature-detecting
  // fingerprinter probes for.
  var Screen = g.__frot_iface('Screen', {
    availWidth: function () { return W; },
    availHeight: function () { return H; },
    width: function () { return W; },
    height: function () { return H; },
    colorDepth: function () { return P.colorDepth; },
    pixelDepth: function () { return P.colorDepth; },
    top: function () { return 0; },
    left: function () { return 0; },
    availTop: function () { return 0; },
    availLeft: function () { return 0; },
    mozOrientation: function () { return 'landscape-primary'; },
    orientation: function () { return orientation; },
  });
  // `mozLockOrientation(type)` returns false (it never locks) and
  // `mozUnlockOrientation()` returns undefined — both measured, both legacy
  // no-ops in Gecko itself, so frot's inertness here is the real behaviour.
  Screen.prototype.mozLockOrientation = g.__frot_brand(
    function mozLockOrientation(o) { return false; }, 'mozLockOrientation');
  Screen.prototype.mozUnlockOrientation = g.__frot_brand(
    function mozUnlockOrientation() {}, 'mozUnlockOrientation');
  onEvent(Screen.prototype, 'onmozorientationchange');
  var screen = Object.create(Screen.prototype);
  Object.defineProperty(g, 'screen', {
    get: function () { return screen; },
    configurable: true,
    enumerable: true,
  });

  // devicePixelRatio + scroll/screen offsets: a borderless top-left viewport with
  // no zoom. Configurable so the probe instrument (bl-bd4e) can wrap it.
  [
    ['devicePixelRatio', P.devicePixelRatio],
    ['screenX', 0], ['screenY', 0], ['screenLeft', 0], ['screenTop', 0],
    ['scrollX', 0], ['scrollY', 0], ['pageXOffset', 0], ['pageYOffset', 0],
  ].forEach(function (pair) {
    Object.defineProperty(g, pair[0], {
      value: pair[1],
      configurable: true,
      enumerable: true,
      writable: true,
    });
  });

  // Visibility/focus: frot takes one impression of a foreground tab, so the
  // honest report is visible + focused (a page gating render on these unblocks).
  if (g.document) {
    Object.defineProperty(g.document, 'visibilityState', {
      get: function () { return 'visible'; },
      configurable: true,
    });
    Object.defineProperty(g.document, 'hidden', {
      get: function () { return false; },
      configurable: true,
    });
    g.document.hasFocus = g.__frot_brand(function hasFocus() { return true; });
  }
})(globalThis);
