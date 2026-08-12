// The two digest EXPANSIONS behind the 2D-canvas masquerade (bl-05e6 / bl-d22f,
// identity.md §11). frot runs neither a rasteriser nor a font engine, yet the two
// things a fingerprinter reads back off a canvas are pixels and text metrics. Both
// are DETERMINISTIC expansions of one 32-bit digest (the fixed profile `canvasSeed`
// folded with the exact draw sequence / the font+string — canvas.js owns the fold):
// the same digest yields the same bytes and the same magnitudes on every
// invocation, a changed draw or a changed string diverges, and nothing is random
// per call (the tell a randomising privacy tool shows). Realism against a reference
// Firefox render is the declared identity.md §11 residual, stated here once for both.
// What is NOT a residual is the SHAPE: `ImageData`'s fields and the TWELVE
// `TextMetrics` fields in their prototype order were READ off Firefox 153.0esr
// (`bl-d22f`, identity.md §3.12) — a missing key is the tell this file closes.
// Runs after brand.js (needs __frot_brand/__frot_iface) and before canvas.js.
(function (g) {
  'use strict';
  var brand = g.__frot_brand;
  var SEED = JSON.parse(g.__frot_env_profile()).canvasSeed >>> 0;

  function def(o, k, v, enumerable) {
    Object.defineProperty(o, k, { value: v, configurable: true, enumerable: !!enumerable });
  }

  // --- ImageData: constructable, Firefox-shaped ------------------------------
  // Published as the global interface; canvas.js's `getImageData`/`createImageData`
  // read it back from there, as a page does.
  (function () {
    var holder = {};
    holder.ImageData = function (a, b, c) {
      if (arguments.length < 2) {
        throw new TypeError('ImageData constructor requires at least 2 arguments');
      }
      var data;
      var w;
      var h;
      if (typeof a === 'object') {
        data = a;
        w = b >>> 0;
        h = c === undefined ? (data.length / 4 / w) >>> 0 : c >>> 0;
      } else {
        w = a >>> 0;
        h = b >>> 0;
        data = new Uint8ClampedArray(w * h * 4);
      }
      def(this, 'data', data, true);
      def(this, 'width', w, true);
      def(this, 'height', h, true);
      def(this, 'colorSpace', 'srgb', true);
    };
    var C = brand(holder.ImageData, 'ImageData');
    Object.defineProperty(C.prototype, Symbol.toStringTag, {
      value: 'ImageData',
      configurable: true,
    });
    Object.defineProperty(g, 'ImageData', { value: C, configurable: true, writable: true });
  })();

  // --- the bitmap ------------------------------------------------------------
  // A never-painted canvas is transparent (like a real one); a painted one expands
  // its digest through an xorshift PRNG into opaque RGBA noise — stable per
  // (seed, draws, size), content-varying, never uniform and never random per call.
  // Cached on the context under the key that determines it, so repeated reads of an
  // unchanged canvas cost nothing.
  function pixels(ctx, w, h) {
    var key = w + 'x' + h + ':' + ctx._d + ':' + (ctx._drawn ? 1 : 0);
    if (ctx._pxKey === key) return ctx._px;
    var px = new Uint8ClampedArray(w * h * 4);
    if (ctx._drawn) {
      var s = (SEED ^ ctx._d ^ Math.imul(w, 0x9e3779b1) ^ Math.imul(h, 0x85ebca77)) >>> 0;
      if (s === 0) s = 0x9e3779b1;
      for (var p = 0; p < w * h; p++) {
        s ^= s << 13; s >>>= 0;
        s ^= s >>> 17;
        s ^= s << 5; s >>>= 0;
        px[p * 4] = s & 0xff;
        px[p * 4 + 1] = (s >>> 8) & 0xff;
        px[p * 4 + 2] = (s >>> 16) & 0xff;
        px[p * 4 + 3] = 255;
      }
    }
    def(ctx, '_px', px);
    def(ctx, '_pxKey', key);
    return px;
  }

  // --- TextMetrics -----------------------------------------------------------
  // The TWELVE fields a real `measureText()` result carries, in the order Firefox
  // 153.0esr defines them on `TextMetrics.prototype` (measured, `bl-d22f`; frot
  // published the first seven, and the five it lacked — the em-height and baseline
  // group — are exactly what a font-metrics fingerprinter reads). They are
  // ACCESSORS ON THE PROTOTYPE, as Firefox's are, not own properties of the result:
  // an instance owns nothing (its numbers live in a WeakMap), so
  // `Object.keys(m)`/`JSON.stringify(m)` read empty, as they do on a real one.
  var FIELDS = ['width', 'actualBoundingBoxLeft', 'actualBoundingBoxRight',
    'fontBoundingBoxAscent', 'fontBoundingBoxDescent', 'actualBoundingBoxAscent',
    'actualBoundingBoxDescent', 'emHeightAscent', 'emHeightDescent',
    'hangingBaseline', 'alphabeticBaseline', 'ideographicBaseline'];
  var store = new WeakMap();
  var accessors = {};
  FIELDS.forEach(function (k, i) {
    accessors[k] = function () {
      return store.get(this)[i];
    };
  });
  var TextMetrics = g.__frot_iface('TextMetrics', accessors);

  // The magnitudes: one digest, twelve slices of it, in FIELDS order. Deliberately
  // NOT modelled on any real font — frot has no glyphs to measure, so a
  // font-shaped-looking number would be fiction with a straight face. They are what
  // the residual above says they are: a stable, string-varying expansion. `width`
  // keeps ~7px/char plus sub-pixel entropy because a page lays out with it.
  function metrics(d, len) {
    var width = len * 7 + (d % 1000) / 1000;
    var asc = 7 + ((d >>> 10) % 4);
    var em = asc + ((d >>> 14) % 2);
    var o = Object.create(TextMetrics.prototype);
    store.set(o, [
      width, // width
      0, // actualBoundingBoxLeft
      width, // actualBoundingBoxRight
      asc + 1, // fontBoundingBoxAscent
      3, // fontBoundingBoxDescent
      asc, // actualBoundingBoxAscent
      2, // actualBoundingBoxDescent
      em, // emHeightAscent
      (d >>> 16) % 3, // emHeightDescent
      em + ((d >>> 18) % 3), // hangingBaseline
      (d >>> 20) % 2, // alphabeticBaseline
      (d >>> 22) % 3, // ideographicBaseline
    ]);
    return o;
  }

  [['__frot_canvas_pixels', pixels], ['__frot_canvas_metrics', metrics]].forEach(function (p) {
    brand(p[1], p[0]);
    Object.defineProperty(g, p[0], { value: p[1], configurable: true, writable: true });
  });
})(globalThis);
