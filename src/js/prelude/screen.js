// screen / devicePixelRatio / visibility (identity.md §8, js.md §7/§8, bl-3972 /
// bl-1cb7). Geometry derives from the ONE layout viewport constant (1280×720,
// __frot_viewport_*), never a second screen constant — depth/DPR from the persona
// profile. Everything is coherent by construction: screen == avail == the
// viewport (no chrome, no taskbar in frot's model), DPR 1, 24-bit colour, and a
// visible/focused document, so a page cannot catch screen and window disagreeing.
// Runs after brand.js (needs __frot_iface) and dom.js (extends document).
(function (g) {
  'use strict';
  var P = JSON.parse(g.__frot_env_profile());
  var W = g.__frot_viewport_width();
  var H = g.__frot_viewport_height();

  // A ScreenOrientation shim: a fixed landscape-primary desktop, enough for the
  // feature detection routers do (`screen.orientation.type`), no event surface.
  var Orientation = g.__frot_iface('ScreenOrientation', {
    type: function () { return 'landscape-primary'; },
    angle: function () { return 0; },
  });
  var orientation = Object.create(Orientation.prototype);

  var Screen = g.__frot_iface('Screen', {
    width: function () { return W; },
    height: function () { return H; },
    availWidth: function () { return W; },
    availHeight: function () { return H; },
    availLeft: function () { return 0; },
    availTop: function () { return 0; },
    colorDepth: function () { return P.colorDepth; },
    pixelDepth: function () { return P.colorDepth; },
    orientation: function () { return orientation; },
  });
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
