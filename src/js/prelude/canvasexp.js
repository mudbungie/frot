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
  var slots = g.__frot_slots;
  var SEED = JSON.parse(g.__frot_env_profile()).canvasSeed >>> 0;

  function def(o, k, v, enumerable) {
    Object.defineProperty(o, k, { value: v, configurable: true, enumerable: !!enumerable });
  }

  // --- ImageData: constructable, Firefox-shaped ------------------------------
  // Published as the global interface; canvas.js's `getImageData`/`createImageData`
  // read it back from there, as a page does.
  //
  // Re-read on Firefox 153.0esr (identity.md §3.17, bl-643d) and two things were
  // wrong. The instance owned `data`, `width`, `height` and `colorSpace` where a
  // real one owns NOTHING — all three come off `ImageData.prototype`, in the
  // order width, height, data. And `colorSpace` **does not exist on 153esr at
  // all**: not on the prototype, not on the instance, and `new ImageData(1,1)
  // .colorSpace` is `undefined`. frot published it as `'srgb'`, so a page could
  // read a member off frot's ImageData that no Firefox has — a costume with an
  // extra button. It is removed. (`getContextAttributes().colorSpace` is a
  // different object and stays: that one was measured present, §3.15.)
  var ImageData = g.__frot_iface('ImageData', null, function (inst, args) {
    if (args.length < 2) {
      throw new TypeError('ImageData constructor requires at least 2 arguments');
    }
    var a = args[0];
    var st = slots(inst);
    if (typeof a === 'object') {
      st.data = a;
      st.width = args[1] >>> 0;
      st.height = args[2] === undefined ? (a.length / 4 / args[1]) >>> 0 : args[2] >>> 0;
    } else {
      st.width = a >>> 0;
      st.height = args[1] >>> 0;
      st.data = new Uint8ClampedArray(st.width * st.height * 4);
    }
  });
  g.__frot_ifaceattrs(ImageData.prototype, ['width', 'height', 'data']);

  // --- the bitmap ------------------------------------------------------------
  // A never-painted canvas is transparent (like a real one); a painted one expands
  // its digest through an xorshift PRNG into opaque RGBA noise — stable per
  // (seed, draws, size), content-varying, never uniform and never random per call.
  // Cached on the context under the key that determines it, so repeated reads of an
  // unchanged canvas cost nothing.
  function pixels(ctx, w, h) {
    var st = slots(ctx);
    var key = w + 'x' + h + ':' + st.d + ':' + (st.drawn ? 1 : 0);
    if (st.pxKey === key) return st.px;
    var px = new Uint8ClampedArray(w * h * 4);
    if (st.drawn) {
      var s = (SEED ^ st.d ^ Math.imul(w, 0x9e3779b1) ^ Math.imul(h, 0x85ebca77)) >>> 0;
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
    st.px = px;
    st.pxKey = key;
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
