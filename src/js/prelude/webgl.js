// WebGL fingerprint — a COHERENT, DETERMINISTIC simulation (bl-f624,
// identity.md §10/§11, js.md §7). Firefox exposes WebGL(2)RenderingContext and a
// context from getContext('webgl'…); a null context is a louder tell than a
// costume (§10), so frot returns branded, Firefox-shaped contexts. VENDOR/RENDERER
// are Firefox's masked "Mozilla"; the real GPU strings surface only through the
// WEBGL_debug_renderer_info extension and name Mesa llvmpipe (software) — coherent
// for Firefox-on-Linux, never over-claiming hardware. Limits, extensions, and
// precision are one real llvmpipe build's set (the `webgl` SSOT, src/fetch/webgl.rs,
// via __frot_env_profile). readPixels/toDataURL are a deterministic digest
// expansion (webglpix.js), stable across invocations. Runs after canvaselem.js
// (which routes getContext here) and canvaspng.js; no syscall — determinism
// forbids entropy.
(function (g) {
  'use strict';
  var brand = g.__frot_brand;
  var mix = g.__frot_webgl_mix;
  var fill = g.__frot_webgl_fill;
  var prof = JSON.parse(g.__frot_env_profile());
  var P = prof.webgl;
  var CSEED = prof.canvasSeed >>> 0;

  function def(o, k, v) {
    Object.defineProperty(o, k, { value: v, enumerable: true, configurable: true });
  }
  function argstr(a) {
    var parts = [];
    for (var i = 0; i < a.length; i++) parts.push(String(a[i]));
    return parts.join(',');
  }

  // Standard WebGL enum constants (spec, platform-independent). Exposed on the
  // context prototype and the key getParameter() resolves its value against.
  var ENUM = {
    VENDOR: 0x1f00, RENDERER: 0x1f01, VERSION: 0x1f02, SHADING_LANGUAGE_VERSION: 0x8b8c,
    MAX_TEXTURE_SIZE: 0x0d33, MAX_CUBE_MAP_TEXTURE_SIZE: 0x851c, MAX_RENDERBUFFER_SIZE: 0x84e8,
    MAX_VIEWPORT_DIMS: 0x0d3a, MAX_VERTEX_ATTRIBS: 0x8869, MAX_VERTEX_UNIFORM_VECTORS: 0x8dfb,
    MAX_FRAGMENT_UNIFORM_VECTORS: 0x8dfd, MAX_VARYING_VECTORS: 0x8dfc,
    MAX_VERTEX_TEXTURE_IMAGE_UNITS: 0x8b4c, MAX_TEXTURE_IMAGE_UNITS: 0x8872,
    MAX_COMBINED_TEXTURE_IMAGE_UNITS: 0x8b4d, ALIASED_LINE_WIDTH_RANGE: 0x846e,
    ALIASED_POINT_SIZE_RANGE: 0x846d, MAX_TEXTURE_MAX_ANISOTROPY_EXT: 0x84ff,
    TEXTURE_MAX_ANISOTROPY_EXT: 0x84fe, RED_BITS: 0x0d52, GREEN_BITS: 0x0d53, BLUE_BITS: 0x0d54,
    ALPHA_BITS: 0x0d55, DEPTH_BITS: 0x0d56, STENCIL_BITS: 0x0d57, MAX_SAMPLES: 0x8d57,
    MAX_3D_TEXTURE_SIZE: 0x8073, MAX_ARRAY_TEXTURE_LAYERS: 0x88ff, MAX_DRAW_BUFFERS: 0x8824,
    MAX_COLOR_ATTACHMENTS: 0x8cdf, MAX_VERTEX_UNIFORM_BLOCKS: 0x8a2b,
    MAX_FRAGMENT_UNIFORM_BLOCKS: 0x8a2d, MAX_UNIFORM_BUFFER_BINDINGS: 0x8a2f,
    MAX_TEXTURE_LOD_BIAS: 0x84fd, UNMASKED_VENDOR_WEBGL: 0x9245, UNMASKED_RENDERER_WEBGL: 0x9246,
    RGBA: 0x1908, UNSIGNED_BYTE: 0x1401, COLOR_BUFFER_BIT: 0x4000, DEPTH_BUFFER_BIT: 0x0100,
    STENCIL_BUFFER_BIT: 0x0400, COMPILE_STATUS: 0x8b81, LINK_STATUS: 0x8b82, NO_ERROR: 0,
    FRAGMENT_SHADER: 0x8b30, VERTEX_SHADER: 0x8b31, HIGH_FLOAT: 0x8df2, MEDIUM_FLOAT: 0x8df1,
    LOW_FLOAT: 0x8df0, HIGH_INT: 0x8df5, MEDIUM_INT: 0x8df4, LOW_INT: 0x8df3,
    ARRAY_BUFFER: 0x8892, ELEMENT_ARRAY_BUFFER: 0x8893, STATIC_DRAW: 0x88e4, TEXTURE_2D: 0x0de1,
    TRIANGLES: 0x0004, FLOAT: 0x1406, TEXTURE0: 0x84c0,
  };

  // The getParameter map for a context version: number -> value. String facts and
  // the debug-renderer UNMASKED strings first, then the numeric/range limits from
  // the SSOT (WebGL2 adds its extra table). Every param name resolves via ENUM.
  function byNumber(version) {
    var m = {};
    m[ENUM.VENDOR] = P.maskedVendor;
    m[ENUM.RENDERER] = P.maskedRenderer;
    m[ENUM.VERSION] = version === 2 ? P.version2 : P.version1;
    m[ENUM.SHADING_LANGUAGE_VERSION] = version === 2 ? P.glsl2 : P.glsl1;
    m[ENUM.UNMASKED_VENDOR_WEBGL] = P.unmaskedVendor;
    m[ENUM.UNMASKED_RENDERER_WEBGL] = P.unmaskedRenderer;
    addParams(m, P.params1);
    if (version === 2) addParams(m, P.params2);
    return m;
  }
  function addParams(m, tbl) {
    Object.keys(tbl).forEach(function (name) {
      m[ENUM[name]] = tbl[name];
    });
  }
  var MAPS = { 1: byNumber(1), 2: byNumber(2) };

  // getParameter: a fixed, coherent value or null (Firefox's answer for an
  // unknown/unsupported enum). Range/dims come back as the right typed array —
  // Float32Array for the ALIASED_* ranges, Int32Array for MAX_VIEWPORT_DIMS —
  // freshly built each call, exactly like a real context.
  function getParameter(pname) {
    var v = MAPS[this._v][pname];
    if (v === undefined) return null;
    if (Array.isArray(v)) {
      return pname === ENUM.ALIASED_LINE_WIDTH_RANGE || pname === ENUM.ALIASED_POINT_SIZE_RANGE
        ? new Float32Array(v)
        : new Int32Array(v);
    }
    return v;
  }

  // --- Extensions ------------------------------------------------------------
  function setOf(list) {
    var s = {};
    list.forEach(function (n) {
      s[n] = true;
    });
    return s;
  }
  var EXTLIST = { 1: P.extensions1, 2: P.extensions2 };
  var EXTSET = { 1: setOf(P.extensions1), 2: setOf(P.extensions2) };
  function getSupportedExtensions() {
    return EXTLIST[this._v].slice();
  }
  // A supported extension returns a Firefox-shaped object (its @@toStringTag is the
  // extension name; the constant-bearing ones carry their constants/methods); an
  // unsupported one returns null. Coherent with getSupportedExtensions().
  function extObject(name) {
    var o = {};
    Object.defineProperty(o, Symbol.toStringTag, { value: name, configurable: true });
    if (name === 'WEBGL_debug_renderer_info') {
      def(o, 'UNMASKED_VENDOR_WEBGL', ENUM.UNMASKED_VENDOR_WEBGL);
      def(o, 'UNMASKED_RENDERER_WEBGL', ENUM.UNMASKED_RENDERER_WEBGL);
    } else if (name === 'EXT_texture_filter_anisotropic') {
      def(o, 'MAX_TEXTURE_MAX_ANISOTROPY_EXT', ENUM.MAX_TEXTURE_MAX_ANISOTROPY_EXT);
      def(o, 'TEXTURE_MAX_ANISOTROPY_EXT', ENUM.TEXTURE_MAX_ANISOTROPY_EXT);
    } else if (name === 'WEBGL_lose_context') {
      o.loseContext = brand(function () {}, 'loseContext');
      o.restoreContext = brand(function () {}, 'restoreContext');
    }
    return o;
  }
  function getExtension(name) {
    return EXTSET[this._v][name] ? extObject(name) : null;
  }

  // getShaderPrecisionFormat: desktop GL / llvmpipe reports IEEE-754 highp for
  // every float qualifier (mediump/lowp identical to highp — the desktop
  // signature, not a mobile 15/15/10 mediump) and 32-bit for every int qualifier.
  var Precision = g.__frot_iface('WebGLShaderPrecisionFormat', {});
  function getShaderPrecisionFormat(shaderType, precisionType) {
    var isFloat =
      precisionType === ENUM.HIGH_FLOAT ||
      precisionType === ENUM.MEDIUM_FLOAT ||
      precisionType === ENUM.LOW_FLOAT;
    var o = Object.create(Precision.prototype);
    def(o, 'rangeMin', isFloat ? 127 : 31);
    def(o, 'rangeMax', isFloat ? 127 : 31);
    def(o, 'precision', isFloat ? 23 : 0);
    return o;
  }

  function getContextAttributes() {
    return {
      alpha: true, antialias: true, depth: true, desynchronized: false,
      failIfMajorPerformanceCaveat: false, powerPreference: 'default',
      premultipliedAlpha: true, preserveDrawingBuffer: false, stencil: false,
      xrCompatible: false,
    };
  }

  // readPixels writes deterministic RGBA into the caller's view (a READ — it does
  // not fold the digest, so two reads of one scene agree, as on real hardware).
  function readPixels(x, y, width, height, format, type, pixels) {
    fill(pixels, this._seed, this._d, width >>> 0, height >>> 0, this._drawn);
  }

  // --- Method zoo ------------------------------------------------------------
  // Draw/state calls fold name+args into the digest in order (so output tracks the
  // exact sequence); the PAINT set additionally marks the buffer non-blank.
  var RECORD = [
    'shaderSource', 'compileShader', 'attachShader', 'detachShader', 'deleteShader',
    'deleteProgram', 'deleteBuffer', 'deleteTexture', 'deleteFramebuffer', 'deleteRenderbuffer',
    'linkProgram', 'useProgram', 'validateProgram', 'bindBuffer', 'bufferData', 'bufferSubData',
    'enableVertexAttribArray', 'disableVertexAttribArray', 'vertexAttribPointer', 'vertexAttrib1f',
    'bindAttribLocation', 'viewport', 'scissor', 'clearColor', 'clearDepth', 'clearStencil',
    'colorMask', 'depthMask', 'depthFunc', 'enable', 'disable', 'blendFunc', 'blendEquation',
    'activeTexture', 'bindTexture', 'texImage2D', 'texParameteri', 'texParameterf', 'pixelStorei',
    'generateMipmap', 'bindFramebuffer', 'framebufferTexture2D', 'bindRenderbuffer',
    'renderbufferStorage', 'uniform1f', 'uniform2f', 'uniform3f', 'uniform4f', 'uniform1i',
    'uniformMatrix2fv', 'uniformMatrix3fv', 'uniformMatrix4fv', 'uniform1fv', 'hint', 'lineWidth',
    'frontFace', 'cullFace', 'flush', 'finish', 'bindVertexArray',
  ];
  var PAINT = ['clear', 'drawArrays', 'drawElements', 'drawArraysInstanced', 'drawElementsInstanced'];
  var NEWOBJ = [
    'createShader', 'createProgram', 'createBuffer', 'createTexture', 'createFramebuffer',
    'createRenderbuffer', 'createVertexArray',
  ];
  function defRecord(proto, name, paints) {
    proto[name] = brand(function () {
      this._d = mix(this._d, name + '(' + argstr(arguments) + ')');
      if (paints) this._drawn = true;
    }, name);
  }
  function defNew(proto, name) {
    proto[name] = brand(function () {
      this._d = mix(this._d, name);
      return {};
    }, name);
  }
  function fixed(proto, name, value) {
    proto[name] = brand(function () {
      return typeof value === 'function' ? value() : value;
    }, name);
  }
  function installMethods(proto) {
    RECORD.forEach(function (n) {
      defRecord(proto, n, false);
    });
    PAINT.forEach(function (n) {
      defRecord(proto, n, true);
    });
    NEWOBJ.forEach(function (n) {
      defNew(proto, n);
    });
    proto.getParameter = brand(getParameter, 'getParameter');
    proto.getSupportedExtensions = brand(getSupportedExtensions, 'getSupportedExtensions');
    proto.getExtension = brand(getExtension, 'getExtension');
    proto.getShaderPrecisionFormat = brand(getShaderPrecisionFormat, 'getShaderPrecisionFormat');
    proto.getContextAttributes = brand(getContextAttributes, 'getContextAttributes');
    proto.readPixels = brand(readPixels, 'readPixels');
    fixed(proto, 'isContextLost', false);
    fixed(proto, 'getError', 0);
    fixed(proto, 'getShaderParameter', true);
    fixed(proto, 'getProgramParameter', true);
    fixed(proto, 'getAttribLocation', 0);
    fixed(proto, 'getUniformLocation', function () {
      return {};
    });
    fixed(proto, 'getShaderInfoLog', '');
    fixed(proto, 'getProgramInfoLog', '');
  }

  // --- Interfaces ------------------------------------------------------------
  var accessors = {
    canvas: function () {
      return this._canvas;
    },
    drawingBufferWidth: function () {
      return this._canvas.width;
    },
    drawingBufferHeight: function () {
      return this._canvas.height;
    },
  };
  function build(name) {
    var Ctor = g.__frot_iface(name, accessors);
    Object.keys(ENUM).forEach(function (k) {
      Object.defineProperty(Ctor.prototype, k, { value: ENUM[k], enumerable: true });
    });
    installMethods(Ctor.prototype);
    return Ctor;
  }
  var GL1 = build('WebGLRenderingContext');
  var GL2 = build('WebGL2RenderingContext');

  function ctxSeed(version) {
    return mix(mix(CSEED, 'webgl'), 'v' + version);
  }
  // get-or-create the one WebGL context for a canvas. A second request for the
  // SAME version returns it; a DIFFERENT version returns null — a canvas binds one
  // context type for life, exactly as a real browser enforces.
  function context(canvas, version) {
    var ex = canvas._ctxgl;
    if (ex) return ex._v === version ? ex : null;
    var ctx = Object.create(version === 2 ? GL2.prototype : GL1.prototype);
    var seed = ctxSeed(version);
    Object.defineProperty(ctx, '_canvas', { value: canvas });
    Object.defineProperty(ctx, '_v', { value: version });
    Object.defineProperty(ctx, '_seed', { value: seed });
    Object.defineProperty(ctx, '_d', { value: mix(seed, 'base'), writable: true, configurable: true });
    Object.defineProperty(ctx, '_drawn', { value: false, writable: true, configurable: true });
    Object.defineProperty(canvas, '_ctxgl', { value: ctx, configurable: true });
    return ctx;
  }
  // the deterministic image/png data URL for a WebGL canvas (the toDataURL() body
  // when a webgl context exists). Expands the digest over the full surface and
  // PNG-encodes it through the shared serialiser.
  function dataURL(canvas) {
    var ctx = canvas._ctxgl;
    var w = canvas.width;
    var h = canvas.height;
    var px = new Uint8ClampedArray(w * h * 4);
    fill(px, ctx._seed, ctx._d, w, h, ctx._drawn);
    return g.__frot_canvas_png(px, w, h);
  }
  [['__frot_webgl_ctx', context], ['__frot_webgl_dataurl', dataURL]].forEach(function (p) {
    brand(p[1], p[0]);
    Object.defineProperty(g, p[0], { value: p[1], configurable: true, writable: true });
  });
})(globalThis);
