// The canvas ELEMENT surface for the 2D-fingerprint masquerade (bl-05e6): the
// `HTMLCanvasElement` interface plus `getContext`/`toDataURL` on Node.prototype.
// Split from canvas.js purely to keep both files under the size cap; the two
// drawing/serialisation factories live there and are reached through the
// `__frot_canvas_ctx` / `__frot_canvas_dataurl` non-enumerable helpers it exports.
// Runs after canvas.js (needs those helpers) and dom.js (extends Node). WebGL is
// bl-f624's job — `getContext('webgl'…)` stays null.
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

  // getContext('2d') returns the branded, deterministic context; any other type
  // (webgl/webgl2/bitmaprenderer) stays a spec-legal null — honest absence until
  // bl-f624. A non-canvas element gets undefined (as in a real browser).
  Node.prototype.getContext = brand(function (type) {
    if (this.tagName !== 'CANVAS') return undefined;
    if (String(type == null ? '' : type).toLowerCase() !== '2d') return null;
    return g.__frot_canvas_ctx(this);
  }, 'getContext');

  // toDataURL() returns the deterministic image/png data URL. The requested MIME
  // type is ignored — frot encodes only PNG (Firefox's default); a jpeg/webp
  // request returns a coherent PNG, a declared residual (identity.md §11).
  Node.prototype.toDataURL = brand(function () {
    return this.tagName === 'CANVAS' ? g.__frot_canvas_dataurl(this) : undefined;
  }, 'toDataURL');
})(globalThis);
