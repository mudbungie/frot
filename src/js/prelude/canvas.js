// 2D-canvas fingerprint — a COHERENT, DETERMINISTIC simulation, not a raster
// (bl-05e6, identity.md §10/§11, js.md §7/§11). frot has no rasteriser, but §10's
// coherence bar makes a null 2D context a louder tell than a costume: Firefox
// returns one, and canvas fingerprinting is a most-probed surface. So `getContext('2d')`
// returns a branded `CanvasRenderingContext2D` whose draw ops fold into a per-
// context digest; `toDataURL()`/`getImageData()` are a deterministic function of
// (the FIXED profile `canvasSeed` + exact draw sequence + dimensions): the SAME
// draws hash identically every invocation, DIFFERENT draws diverge, never random
// per call (the tell privacy tools show). The pixels are a digest expansion, not a
// glyph render — a declared §11 residual for pixel realism. WebGL stays bl-f624's.
// Runs after brand.js/dom.js/canvaspng.js; no syscall (determinism forbids entropy).
(function (g) {
  'use strict';
  var brand = g.__frot_brand;
  var SEED = JSON.parse(g.__frot_env_profile()).canvasSeed >>> 0;

  // FNV-1a over a string's UTF-16 code units (folded as two bytes each). The one
  // mixing primitive: draw digest, pixel PRNG seed, and measureText width all use it.
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

  // --- ImageData: constructable, Firefox-shaped ------------------------------
  var ImageData = (function () {
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
    return C;
  })();
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

  // --- TextMetrics: deterministic, font/text-derived -------------------------
  var TextMetrics = g.__frot_iface('TextMetrics', {});
  function textMetrics(font, text) {
    var hh = mix(mix(BASE, 'measure'), font + '|' + text);
    // A plausible advance width: ~7px/char plus sub-pixel entropy from the digest.
    var width = text.length * 7 + (hh % 1000) / 1000;
    var asc = 7 + ((hh >>> 10) % 4);
    var o = Object.create(TextMetrics.prototype);
    def(o, 'width', width, true);
    def(o, 'actualBoundingBoxLeft', 0, true);
    def(o, 'actualBoundingBoxRight', width, true);
    def(o, 'actualBoundingBoxAscent', asc, true);
    def(o, 'actualBoundingBoxDescent', 2, true);
    def(o, 'fontBoundingBoxAscent', asc + 1, true);
    def(o, 'fontBoundingBoxDescent', 3, true);
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
  var DEFAULTS = {
    fillStyle: '#000000', strokeStyle: '#000000', globalAlpha: 1,
    lineWidth: 1, lineCap: 'butt', lineJoin: 'miter', miterLimit: 10,
    lineDashOffset: 0, font: '10px sans-serif', textAlign: 'start',
    textBaseline: 'alphabetic', direction: 'inherit', filter: 'none',
    globalCompositeOperation: 'source-over', imageSmoothingEnabled: true,
    imageSmoothingQuality: 'low', shadowBlur: 0, shadowColor: 'rgba(0, 0, 0, 0)',
    shadowOffsetX: 0, shadowOffsetY: 0,
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

  proto.measureText = brand(function (text) {
    return textMetrics(this.font, String(text));
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
    return new ImageData(w, h);
  }, 'createImageData');

  // The deterministic bitmap: a never-painted canvas is transparent (like a real
  // one); a painted one expands its digest through an xorshift PRNG into opaque RGBA
  // noise — stable per (seed, draws, size), content-varying, never uniform/random.
  function pixels(ctx) {
    var w = dim(ctx._canvas, 'width', 300);
    var h = dim(ctx._canvas, 'height', 150);
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
  proto.getImageData = brand(function (sx, sy, sw, sh) {
    sx |= 0;
    sy |= 0;
    sw = Math.abs(sw | 0);
    sh = Math.abs(sh | 0);
    var full = pixels(this);
    var cw = dim(this._canvas, 'width', 300);
    var ch = dim(this._canvas, 'height', 150);
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
    return new ImageData(out, sw, sh);
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
    var px = canvas._ctx2d ? pixels(canvas._ctx2d) : new Uint8ClampedArray(w * h * 4);
    return g.__frot_canvas_png(px, w, h);
  }
  [['__frot_canvas_ctx', context], ['__frot_canvas_dataurl', dataURL]].forEach(function (p) {
    brand(p[1], p[0]);
    Object.defineProperty(g, p[0], { value: p[1], configurable: true, writable: true });
  });
})(globalThis);
