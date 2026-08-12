// The canvas ELEMENT surface for the 2D-fingerprint masquerade (bl-05e6): the
// `HTMLCanvasElement` interface plus `getContext`/`toDataURL` on Node.prototype.
// Split from canvas.js purely to keep both files under the size cap; the two
// drawing/serialisation factories live there and are reached through the
// `__frot_canvas_ctx` / `__frot_canvas_dataurl` non-enumerable helpers it exports.
// Runs after canvas.js (needs those helpers) and dom.js (extends Node). `getContext`
// also routes the WebGL context types to webgl.js's `__frot_webgl_ctx` (bl-f624);
// a canvas binds ONE context type for life, so a cross-type request returns null.
(function (g) {
  'use strict';
  var brand = g.__frot_brand;
  var Node = g.Node;

  // HTMLCanvasElement: an `instanceof` interface matching CANVAS elements, exactly
  // like elem.js's tag-keyed interfaces. Every node is a `Node`; @@hasInstance
  // answers by tag, so `c instanceof HTMLCanvasElement` holds without a class tree.
  var HTMLCanvasElement = function () {};
  Object.defineProperty(HTMLCanvasElement, Symbol.hasInstance, {
    value: function (o) {
      return !!o && typeof o === 'object' && o.nodeType === 1 && o.tagName === 'CANVAS';
    },
  });
  brand(HTMLCanvasElement, 'HTMLCanvasElement');
  Object.defineProperty(g, 'HTMLCanvasElement', {
    value: HTMLCanvasElement,
    configurable: true,
    writable: true,
  });

  // getContext('2d') returns the branded 2D context; 'webgl'/'experimental-webgl'
  // and 'webgl2' route to the WebGL simulation (bl-f624). A canvas binds ONE
  // context type, so a request for a different family than one already taken
  // returns null. 'moz-webgl'/'webkit-3d'/'webgl2-compute'/'bitmaprenderer' are a
  // spec-legal null (Firefox returns none). A non-canvas element gets undefined.
  Node.prototype.getContext = brand(function (type, options) {
    if (this.tagName !== 'CANVAS') return undefined;
    var t = String(type == null ? '' : type).toLowerCase();
    // The 2D options bag reaches the context: Gecko echoes `alpha` and
    // `willReadFrequently` back through `getContextAttributes()` (measured,
    // `bl-706b`), so they have to be remembered at creation.
    if (t === '2d') return this._ctxgl ? null : g.__frot_canvas_ctx(this, options);
    if (t === 'webgl' || t === 'experimental-webgl') {
      return this._ctx2d ? null : g.__frot_webgl_ctx(this, 1);
    }
    if (t === 'webgl2') return this._ctx2d ? null : g.__frot_webgl_ctx(this, 2);
    return null;
  }, 'getContext');

  // toDataURL() returns the deterministic image/png data URL for whichever context
  // backs the canvas (WebGL if one was taken, else the 2D bitmap). The requested
  // MIME type is ignored — frot encodes only PNG (Firefox's default); a jpeg/webp
  // request returns a coherent PNG, a declared residual (identity.md §11).
  Node.prototype.toDataURL = brand(function () {
    if (this.tagName !== 'CANVAS') return undefined;
    return this._ctxgl ? g.__frot_webgl_dataurl(this) : g.__frot_canvas_dataurl(this);
  }, 'toDataURL');
})(globalThis);
