// 2D-canvas fingerprint — a COHERENT, DETERMINISTIC simulation, not a raster
// (bl-05e6, identity.md §10/§11, js.md §7/§11). frot has no rasteriser, but §10's
// coherence bar makes a null 2D context a louder tell than a costume: Firefox
// returns one, and canvas fingerprinting is a most-probed surface. So `getContext('2d')`
// returns a branded `CanvasRenderingContext2D` whose draw ops and state writes fold
// into a per-context digest. This file owns that CONTEXT — its properties, its draw
// API, and the element bridge; the two expansions of the digest a page reads back
// (the bitmap and the twelve `TextMetrics` magnitudes) live in canvasexp.js, which
// states their shared §11 residual once. The whole surface was re-read off Firefox
// 153.0esr in `bl-d22f` (identity.md §3.12). Runs after brand.js/dom.js/canvaspng.js
// /canvasexp.js; no syscall (determinism forbids entropy).
(function (g) {
  'use strict';
  var brand = g.__frot_brand;
  var SEED = JSON.parse(g.__frot_env_profile()).canvasSeed >>> 0;
  var pixels = g.__frot_canvas_pixels;
  var metrics = g.__frot_canvas_metrics;

  // FNV-1a over a string's UTF-16 code units (folded as two bytes each). The one
  // mixing primitive: draw digest, pixel PRNG seed, and text metrics all derive
  // from what it folds.
  function mix(h, s) {
    h = h >>> 0;
    for (var i = 0; i < s.length; i++) {
      var c = s.charCodeAt(i);
      h = Math.imul(h ^ (c & 0xff), 0x01000193) >>> 0;
      h = Math.imul(h ^ ((c >>> 8) & 0xff), 0x01000193) >>> 0;
    }
    return h >>> 0;
  }
  var BASE = mix(SEED, '2d'); // the digest a fresh context starts from.

  function def(o, k, v, enumerable) {
    Object.defineProperty(o, k, { value: v, configurable: true, enumerable: !!enumerable });
  }

  // --- CanvasGradient / CanvasPattern: recording stand-ins -------------------
  // Fingerprinters occasionally paint via a gradient; a real branded object whose
  // addColorStop folds into the digest keeps the draw coherent (stops vary output).
  var CanvasGradient = g.__frot_iface('CanvasGradient', {});
  var CanvasPattern = g.__frot_iface('CanvasPattern', {});
  function gradient(ctx, tag, args) {
    ctx._d = mix(ctx._d, tag + '(' + argstr(args) + ')');
    var o = Object.create(CanvasGradient.prototype);
    o.addColorStop = brand(function (off, color) {
      ctx._d = mix(ctx._d, 'stop(' + off + ',' + color + ')');
    }, 'addColorStop');
    return o;
  }

  function argstr(args) {
    var parts = [];
    for (var i = 0; i < args.length; i++) parts.push(String(args[i]));
    return parts.join(',');
  }

  // --- CanvasRenderingContext2D ----------------------------------------------
  var Ctx2D = g.__frot_iface('CanvasRenderingContext2D', {
    canvas: function () {
      return this._canvas;
    },
  });
  var proto = Ctx2D.prototype;

  // Read/write state properties: each stores its value in a non-enumerable slot and
  // folds the assignment into the digest, so a state change (colour, font, alpha…)
  // coherently changes the resulting image, as on a real canvas painted afterwards.
  //
  // Every name and default below was READ off Firefox 153.0esr (`bl-d22f`, §3.12),
  // which cost the set two corrections. `imageSmoothingQuality` is GONE:
  // `'imageSmoothingQuality' in ctx` is FALSE on Firefox — Gecko does not implement
  // it, so publishing it (at a spec-derived `'low'`) was frot claiming a capability
  // no Firefox has, the same class of tell as an over-specific WebGL renderer
  // string. The text-shaping five — letterSpacing, wordSpacing, fontKerning,
  // fontStretch, textRendering — are the other direction: real, present, and
  // missing here, with the defaults measured beside them.
  var DEFAULTS = {
    fillStyle: '#000000', strokeStyle: '#000000', globalAlpha: 1,
    lineWidth: 1, lineCap: 'butt', lineJoin: 'miter', miterLimit: 10,
    lineDashOffset: 0, font: '10px sans-serif', textAlign: 'start',
    textBaseline: 'alphabetic', direction: 'inherit', filter: 'none',
    globalCompositeOperation: 'source-over', imageSmoothingEnabled: true,
    shadowBlur: 0, shadowColor: 'rgba(0, 0, 0, 0)',
    shadowOffsetX: 0, shadowOffsetY: 0, letterSpacing: '0px', wordSpacing: '0px',
    fontKerning: 'auto', fontStretch: 'normal', textRendering: 'auto',
  };
  Object.keys(DEFAULTS).forEach(function (k) {
    var slot = '_s_' + k;
    Object.defineProperty(proto, k, {
      enumerable: true,
      configurable: true,
      get: brand(function () {
        return Object.prototype.hasOwnProperty.call(this, slot) ? this[slot] : DEFAULTS[k];
      }, 'get ' + k),
      set: brand(function (v) {
        def(this, slot, v);
        this._d = mix(this._d, k + '=' + String(v));
      }, 'set ' + k),
    });
  });

  // Methods that PAINT (mark the bitmap non-blank) vs path/state ops that only fold
  // into the digest — each call's name+args enter it in order, so output tracks the
  // exact draw sequence.
  var PAINT = ['fillRect', 'strokeRect', 'clearRect', 'fillText', 'strokeText',
    'fill', 'stroke', 'drawImage', 'putImageData'];
  var PATH = ['beginPath', 'closePath', 'moveTo', 'lineTo', 'bezierCurveTo',
    'quadraticCurveTo', 'arc', 'arcTo', 'ellipse', 'rect', 'roundRect', 'save',
    'restore', 'scale', 'rotate', 'translate', 'transform', 'setTransform',
    'resetTransform', 'clip', 'setLineDash'];
  function defMethod(name, paints) {
    proto[name] = brand(function () {
      this._d = mix(this._d, name + '(' + argstr(arguments) + ')');
      if (paints) this._drawn = true;
    }, name);
  }
  PAINT.forEach(function (n) {
    defMethod(n, true);
  });
  PATH.forEach(function (n) {
    defMethod(n, false);
  });

  // The four keys, in this order, and these values, are what a bare
  // `getContext('2d').getContextAttributes()` returns on Firefox 153.0esr
  // (measured, `bl-d22f`). frot's `getContext` takes no options — a request for
  // `{alpha: false}` is neither honoured nor echoed, because whether Gecko echoes a
  // non-default attribute here was not measured and is not going to be guessed
  // (§11). Absent, this threw a TypeError where every real browser answers.
  proto.getContextAttributes = brand(function () {
    return {
      alpha: true,
      colorSpace: 'srgb',
      desynchronized: false,
      willReadFrequently: false,
    };
  }, 'getContextAttributes');

  proto.measureText = brand(function (text) {
    var s = String(text);
    return metrics(mix(mix(BASE, 'measure'), this.font + '|' + s), s.length);
  }, 'measureText');
  proto.getLineDash = brand(function () {
    return [];
  }, 'getLineDash');
  proto.isPointInPath = brand(function () {
    return false;
  }, 'isPointInPath');
  proto.isPointInStroke = brand(function () {
    return false;
  }, 'isPointInStroke');
  proto.createLinearGradient = brand(function () {
    return gradient(this, 'linear', arguments);
  }, 'createLinearGradient');
  proto.createRadialGradient = brand(function () {
    return gradient(this, 'radial', arguments);
  }, 'createRadialGradient');
  proto.createConicGradient = brand(function () {
    return gradient(this, 'conic', arguments);
  }, 'createConicGradient');
  proto.createPattern = brand(function () {
    this._d = mix(this._d, 'pattern(' + argstr(arguments) + ')');
    return Object.create(CanvasPattern.prototype);
  }, 'createPattern');
  proto.createImageData = brand(function (a, b) {
    var w = typeof a === 'object' ? a.width : a >>> 0;
    var h = typeof a === 'object' ? a.height : b >>> 0;
    return new g.ImageData(w, h);
  }, 'createImageData');

  proto.getImageData = brand(function (sx, sy, sw, sh) {
    sx |= 0;
    sy |= 0;
    sw = Math.abs(sw | 0);
    sh = Math.abs(sh | 0);
    var cw = dim(this._canvas, 'width', 300);
    var ch = dim(this._canvas, 'height', 150);
    var full = pixels(this, cw, ch);
    var out = new Uint8ClampedArray(sw * sh * 4);
    for (var y = 0; y < sh; y++) {
      for (var x = 0; x < sw; x++) {
        var srcx = sx + x;
        var srcy = sy + y;
        if (srcx < 0 || srcy < 0 || srcx >= cw || srcy >= ch) continue;
        var di = (y * sw + x) * 4;
        var si = (srcy * cw + srcx) * 4;
        out[di] = full[si];
        out[di + 1] = full[si + 1];
        out[di + 2] = full[si + 2];
        out[di + 3] = full[si + 3];
      }
    }
    return new g.ImageData(out, sw, sh);
  }, 'getImageData');

  // --- canvas element bridge -------------------------------------------------
  // width/height reflection lives here; the getContext/toDataURL surface is in
  // canvaselem.js (size cap) and reaches the two factories exported at the end.
  var Node = g.Node;

  // A canvas dimension reflected from its content attribute (SSOT), default 300×150
  // per the HTML spec. Shared by the reflection getters and both factories.
  function dim(canvas, name, dflt) {
    var v = canvas.getAttribute(name);
    var n = v == null ? NaN : parseInt(v, 10);
    return isFinite(n) && n >= 0 ? n : dflt;
  }

  // width/height are tag-guarded accessors on Node.prototype — the same shared-
  // prototype shape residual getContext had (§11). Resizing resets the bitmap.
  function reflect(name, dflt) {
    Object.defineProperty(Node.prototype, name, {
      configurable: true,
      get: brand(function () {
        return this.tagName === 'CANVAS' ? dim(this, name, dflt) : undefined;
      }, 'get ' + name),
      set: brand(function (v) {
        if (this.tagName !== 'CANVAS') return;
        this.setAttribute(name, String(v >>> 0));
        if (this._ctx2d) {
          this._ctx2d._drawn = false;
          this._ctx2d._d = BASE;
        }
      }, 'set ' + name),
    });
  }
  reflect('width', 300);
  reflect('height', 150);

  // get-or-create the one 2D context for a canvas (the getContext('2d') body).
  function context(canvas) {
    if (!canvas._ctx2d) {
      var ctx = Object.create(proto);
      def(ctx, '_canvas', canvas);
      Object.defineProperty(ctx, '_d', { value: BASE, configurable: true, writable: true });
      Object.defineProperty(ctx, '_drawn', { value: false, configurable: true, writable: true });
      def(canvas, '_ctx2d', ctx);
    }
    return canvas._ctx2d;
  }
  // the deterministic image/png data URL (the toDataURL() body); an un-contexted
  // canvas is transparent — a valid blank PNG, as in a real browser.
  function dataURL(canvas) {
    var w = dim(canvas, 'width', 300);
    var h = dim(canvas, 'height', 150);
    var px = canvas._ctx2d ? pixels(canvas._ctx2d, w, h) : new Uint8ClampedArray(w * h * 4);
    return g.__frot_canvas_png(px, w, h);
  }
  [['__frot_canvas_ctx', context], ['__frot_canvas_dataurl', dataURL]].forEach(function (p) {
    brand(p[1], p[0]);
    Object.defineProperty(g, p[0], { value: p[1], configurable: true, writable: true });
  });
})(globalThis);
